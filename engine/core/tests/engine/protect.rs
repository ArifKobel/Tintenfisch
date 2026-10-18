use crate::support;

use std::fs;
use std::path::PathBuf;

use support::fixtures;
use support::rendering::TestRenderer;
use tintenfisch::crypt::{Permissions, Protection};
use tintenfisch::document::Document;
use tintenfisch::error::ErrorKind;
use tintenfisch::file::SourceFile;
use tintenfisch::syntax::Object;

const NONE: Permissions = Permissions {
    print: false,
    print_faithful: false,
    modify: false,
    copy: false,
    annotate: false,
    fill_forms: false,
    assemble: false,
};

fn entropy(seed: u8) -> Vec<u8> {
    (0..Protection::ENTROPY).map(|index| (index as u8).wrapping_mul(31).wrapping_add(seed)).collect()
}

fn protection(user: &str, owner: &str, permissions: Permissions) -> Protection {
    Protection { user_password: user.into(), owner_password: owner.into(), permissions }
}

fn encrypted_fixture(file: &str) -> Vec<u8> {
    fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/encrypted/{file}.pdf"))).unwrap()
}

fn rendering(bytes: Vec<u8>, password: &str) -> Vec<u8> {
    TestRenderer::with_password(bytes, password).render(0, 1.0, None).rgba
}

#[test]
fn a_protected_file_opens_only_with_its_passwords_and_keeps_its_content() {
    let original = fixtures::editing_features();
    let mut document = Document::open(original.clone(), "").unwrap();
    let permissions = Permissions { print: true, copy: true, ..NONE };
    document.protect(&protection("open sesame", "boss", permissions), &entropy(1)).unwrap();
    assert!(document.protection().encrypted);
    let saved = document.save(false).unwrap();
    assert_eq!(Document::open(saved.clone(), "").err().unwrap().kind, ErrorKind::NeedsPassword);
    assert_eq!(Document::open(saved.clone(), "wrong").err().unwrap().kind, ErrorKind::NeedsPassword);
    let as_user = Document::open(saved.clone(), "open sesame").unwrap();
    assert_eq!(
        as_user.protection(),
        tintenfisch::document::ProtectionState { encrypted: true, permissions, owner_rights: false }
    );
    assert!(Document::open(saved.clone(), "boss").unwrap().protection().owner_rights);
    let file = SourceFile::open_with_password(saved.clone(), "boss").unwrap();
    let encrypt = file.trailer().get("Encrypt").and_then(Object::as_reference).unwrap();
    let dictionary = file.object(encrypt).unwrap();
    let dictionary = dictionary.as_dict().unwrap();
    assert_eq!((dictionary.int("V"), dictionary.int("R")), (Some(5), Some(6)));
    assert!(!saved.windows(b"Plain line".len()).any(|window| window == b"Plain line"), "the text is encrypted");
    assert_eq!(rendering(saved, "open sesame"), rendering(original, ""));
}

#[test]
fn a_permissions_password_alone_opens_freely_and_guards_the_protection() {
    let original = fixtures::editing_features();
    let mut document = Document::open(original.clone(), "").unwrap();
    document.protect(&protection("", "boss", NONE), &entropy(2)).unwrap();
    let saved = document.save(false).unwrap();
    let mut reopened = Document::open(saved, "").unwrap();
    assert_eq!(reopened.protection().permissions, NONE);
    assert!(!reopened.protection().owner_rights);
    assert_eq!(reopened.unprotect().err().unwrap().kind, ErrorKind::NeedsPassword);
    assert!(!reopened.grant_owner_rights("wrong"));
    assert!(reopened.grant_owner_rights("boss"));
    reopened.unprotect().unwrap();
    let plain = reopened.save(false).unwrap();
    assert!(!SourceFile::open(plain.clone()).unwrap().trailer().contains("Encrypt"));
    assert_eq!(rendering(plain, ""), rendering(original, ""));
}

#[test]
fn without_an_owner_password_nobody_gets_owner_rights() {
    let mut document = Document::open(fixtures::editing_features(), "").unwrap();
    document.protect(&protection("user", "", Permissions::ALL), &entropy(3)).unwrap();
    let saved = document.save(false).unwrap();
    assert!(!Document::open(saved.clone(), "user").unwrap().protection().owner_rights);
    assert!(Document::open(saved, "").is_err());
    let mut document = Document::open(fixtures::editing_features(), "").unwrap();
    assert!(document.protect(&protection("", "", Permissions::ALL), &entropy(4)).is_err());
    assert!(document.protect(&protection("user", "boss", Permissions::ALL), &[0; 12]).is_err());
}

#[test]
fn a_file_another_program_protected_gets_new_passwords_once_the_owner_password_is_known() {
    let mut document = Document::open(encrypted_fixture("aes-256"), "secret").unwrap();
    let before = rendering(encrypted_fixture("aes-256"), "secret");
    assert!(!document.protection().owner_rights);
    assert!(document.protect(&protection("new", "boss", Permissions::ALL), &entropy(5)).is_err());
    assert!(document.grant_owner_rights("owner"));
    document.protect(&protection("new", "boss", Permissions::ALL), &entropy(5)).unwrap();
    let saved = document.save(false).unwrap();
    assert!(Document::open(saved.clone(), "secret").is_err());
    assert_eq!(rendering(saved, "new"), before);
}

#[test]
fn removing_the_protection_of_an_rc4_file_gives_a_plain_file() {
    let source = encrypted_fixture("rc4-128");
    let mut document = Document::open(source.clone(), "owner").unwrap();
    assert!(document.protection().owner_rights);
    document.unprotect().unwrap();
    assert!(!document.protection().encrypted);
    let plain = document.save(false).unwrap();
    assert!(!SourceFile::open(plain.clone()).unwrap().trailer().contains("Encrypt"));
    assert_eq!(rendering(plain, ""), rendering(source, "secret"));
}

#[test]
#[ignore = "writes protected files to PREVIEW_DIR for checking them with other viewers"]
fn write_protected_preview() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let mut document = Document::open(fixtures::editing_features(), "").unwrap();
    document
        .protect(&protection("open sesame", "boss", Permissions { print: true, copy: true, ..NONE }), &entropy(7))
        .unwrap();
    fs::write(format!("{directory}/protected-user.pdf"), document.save(false).unwrap()).unwrap();
    let mut document = Document::open(fixtures::editing_features(), "").unwrap();
    document.protect(&protection("", "boss", Permissions { print: true, ..NONE }), &entropy(8)).unwrap();
    fs::write(format!("{directory}/protected-owner.pdf"), document.save(false).unwrap()).unwrap();
}
