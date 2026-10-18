use crate::support;

use std::fs;
use std::path::PathBuf;

use support::rendering::{TestRenderer, from_png};
use tintenfisch::document::Document;
use tintenfisch::error::ErrorKind;
use tintenfisch::file::SourceFile;
use tintenfisch::save::{FullRewriter, IncrementalWriter};
use tintenfisch::store::ObjectStore;
use tintenfisch::syntax::Object;

struct Encrypted {
    file: &'static str,
    user_password: &'static str,
    plain_golden: &'static str,
    pages: usize,
}

const ENCRYPTED: [Encrypted; 4] = [
    Encrypted { file: "rc4-40", user_password: "", plain_golden: "render-features", pages: 4 },
    Encrypted { file: "rc4-128", user_password: "secret", plain_golden: "chrome-article", pages: 1 },
    Encrypted { file: "aes-128", user_password: "", plain_golden: "image-features", pages: 1 },
    Encrypted { file: "aes-256", user_password: "secret", plain_golden: "shading-features", pages: 1 },
];

fn bytes(file: &str) -> Vec<u8> {
    fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/encrypted/{file}.pdf"))).unwrap()
}

fn golden(name: &str, page: usize) -> tintenfisch::render::Raster {
    from_png(
        &fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/{name}-{page}.png"))).unwrap(),
    )
}

#[test]
fn every_encrypted_fixture_renders_like_its_plain_source() {
    for encrypted in &ENCRYPTED {
        let renderer = TestRenderer::with_password(bytes(encrypted.file), encrypted.user_password);
        assert_eq!(renderer.page_count(), encrypted.pages, "{}", encrypted.file);
        for page in 0..encrypted.pages {
            let rendered = renderer.render(page, 1.0, None);
            assert!(
                rendered == golden(encrypted.plain_golden, page),
                "{} page {page} differs from its plain golden",
                encrypted.file
            );
        }
    }
}

#[test]
fn a_user_password_is_required_and_checked() {
    for file in ["rc4-128", "aes-256"] {
        let missing = SourceFile::open(bytes(file)).err().unwrap();
        assert_eq!(missing.kind, ErrorKind::NeedsPassword, "{file}");
        let wrong = SourceFile::open_with_password(bytes(file), "guess").err().unwrap();
        assert_eq!(wrong.kind, ErrorKind::NeedsPassword, "{file}");
        assert!(SourceFile::open_with_password(bytes(file), "owner").is_ok(), "{file}: owner password");
    }
    assert!(SourceFile::open(bytes("rc4-40")).is_ok());
}

#[test]
fn saving_keeps_the_encryption_and_the_content() {
    for encrypted in &ENCRYPTED {
        let mut store =
            ObjectStore::new(SourceFile::open_with_password(bytes(encrypted.file), encrypted.user_password).unwrap());
        let note = store
            .transact("note", "", |transaction| Ok(transaction.allocate(Object::String(b"plain note".to_vec()))))
            .unwrap();
        for (mode, saved) in [
            ("incremental", IncrementalWriter::new(&store).write().unwrap()),
            ("full", FullRewriter::new(&store).write().unwrap()),
        ] {
            let reopened = SourceFile::open_with_password(saved.clone(), encrypted.user_password).unwrap();
            assert!(reopened.trailer().contains("Encrypt"), "{} {mode}: encryption dropped", encrypted.file);
            if !encrypted.user_password.is_empty() {
                assert!(SourceFile::open(saved.clone()).is_err(), "{} {mode}: opens without password", encrypted.file);
            }
            if mode == "incremental" {
                assert_eq!(
                    *reopened.object(note).unwrap(),
                    Object::String(b"plain note".to_vec()),
                    "{}",
                    encrypted.file
                );
                assert!(
                    !saved.windows(10).any(|window| window == b"plain note"),
                    "{}: note written in clear",
                    encrypted.file
                );
            }
            let renderer = TestRenderer::with_password(saved, encrypted.user_password);
            assert!(
                renderer.render(0, 1.0, None) == golden(encrypted.plain_golden, 0),
                "{} {mode}: render changed",
                encrypted.file
            );
        }
    }
}

#[test]
fn a_document_reports_that_it_needs_a_password() {
    let error = Document::open(bytes("aes-256"), "").err().unwrap();
    assert_eq!(error.kind, ErrorKind::NeedsPassword);
}
