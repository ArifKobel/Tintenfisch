use crate::support;

use support::fixtures;
use support::rendering::library;
use tintenfisch::annotations::{Markup, MarkupStyle};
use tintenfisch::document::Document;
use tintenfisch::error::ErrorKind;
use tintenfisch::fonts::FontLibrary;
use tintenfisch::forms::NewField;
use tintenfisch::forms::NewFieldKind;
use tintenfisch::geometry::Rect;
use tintenfisch::pages::PageId;
use tintenfisch::signing::{SignatureMark, SignatureReport, SignatureStatus, SigningDetails, SigningIdentity};
use tintenfisch::syntax::{ObjId, Object};

const PASSWORD: &str = "tintenfisch-test";

struct Office {
    document: Document,
    library: FontLibrary,
    page: PageId,
}

impl Office {
    fn open(bytes: Vec<u8>) -> Self {
        let library = library();
        let document = Document::open(bytes, "").unwrap();
        let page = document.view(&library).pages().unwrap()[0].id;
        Self { document, library, page }
    }

    fn with_signature_field() -> (Self, ObjId) {
        let mut office = Office::open(fixtures::editing_features());
        let mark = SignatureMark::Ink {
            strokes: vec![vec![(0.0, 10.0), (40.0, 0.0), (80.0, 10.0)]],
            width: 2.0,
            color: [0.1, 0.2, 0.6],
        };
        let field = office
            .document
            .add_signature(office.page, Rect::new(200.0, 300.0, 380.0, 360.0), "Approval", &mark)
            .unwrap();
        (office, field)
    }

    fn sign(&mut self, field: ObjId, certificate: &str) -> Vec<u8> {
        self.document.sign_field(field, &identity(certificate), &details()).unwrap()
    }

    fn only_report(&self) -> SignatureReport {
        let reports = self.document.signatures().unwrap();
        assert_eq!(reports.len(), 1, "{reports:?}");
        reports.into_iter().next().unwrap()
    }
}

fn identity(certificate: &str) -> SigningIdentity {
    let bytes = std::fs::read(format!("tests/certificates/{certificate}")).unwrap();
    SigningIdentity::from_pkcs12(&bytes, PASSWORD).unwrap()
}

fn details() -> SigningDetails {
    SigningDetails {
        reason: "Approved".to_string(),
        location: "Hamburg".to_string(),
        date: "D:20261018120000Z".to_string(),
    }
}

#[test]
fn a_signed_field_checks_valid_and_the_signature_follows_the_pending_changes() {
    let (mut office, field) = Office::with_signature_field();
    let unsigned = office.document.save(false).unwrap();
    let signed = office.sign(field, "test-signer.p12");
    assert!(signed.starts_with(&unsigned), "the signature is appended to the saved changes");
    let report = office.only_report();
    assert_eq!(report.status, SignatureStatus::Valid);
    assert_eq!(report.signer.as_deref(), Some("Tintenfisch Test Signer"));
    assert_eq!(
        (report.field_name.as_str(), report.reason.as_str(), report.signed_at.as_str()),
        ("Approval", "Approved", "D:20261018120000Z")
    );
    assert!(!report.changed_after);
    assert!(office.document.undo().is_none(), "the signed file is the new base");
    let reopened = Office::open(signed);
    assert_eq!(reopened.only_report().status, SignatureStatus::Valid);
    let catalog = reopened.document.store().trailer().get("Root").and_then(Object::as_reference).unwrap();
    let form = reopened.document.store().object(catalog).unwrap().as_dict().unwrap().get("AcroForm").cloned().unwrap();
    let form = reopened.document.store().resolve(&form).unwrap();
    assert_eq!(form.as_dict().unwrap().get("SigFlags").and_then(Object::as_int), Some(3));
}

#[test]
fn a_certificate_file_in_the_legacy_format_signs_too() {
    let (mut office, field) = Office::with_signature_field();
    office.sign(field, "test-signer-legacy.p12");
    assert_eq!(office.only_report().status, SignatureStatus::Valid);
}

#[test]
fn an_incremental_change_after_signing_keeps_the_signature_valid() {
    let (mut office, field) = Office::with_signature_field();
    office.sign(field, "test-signer.p12");
    let note = Markup::Note { at: (40.0, 40.0), text: "Checked after signing".to_string() };
    let style = MarkupStyle { color: [0.9, 0.7, 0.1], width: 1.0, author: "Reviewer".to_string(), date: String::new() };
    office.document.add_markup(office.page, &note, &style, &office.library).unwrap();
    let edited = Office::open(office.document.save(false).unwrap());
    let report = edited.only_report();
    assert_eq!(report.status, SignatureStatus::Valid);
    assert!(report.changed_after, "the change after signing is noticed");
}

#[test]
fn a_changed_byte_breaks_the_signature() {
    let (mut office, field) = Office::with_signature_field();
    let mut signed = office.sign(field, "test-signer.p12");
    let at = signed.windows(14).position(|window| window == b"Plain line to ").unwrap();
    signed[at] = b'Q';
    let report = Office::open(signed).only_report();
    assert_eq!(report.status, SignatureStatus::Broken("the signed bytes have changed".to_string()));
}

#[test]
fn wrong_passwords_other_keys_and_fields_are_refused() {
    let bytes = std::fs::read("tests/certificates/test-signer.p12").unwrap();
    let wrong = SigningIdentity::from_pkcs12(&bytes, "guess").err().unwrap();
    assert_eq!(wrong.kind, ErrorKind::NeedsPassword);
    let ec = std::fs::read("tests/certificates/test-signer-ec.p12").unwrap();
    assert_eq!(SigningIdentity::from_pkcs12(&ec, PASSWORD).err().unwrap().kind, ErrorKind::Unsupported);
    let (mut office, field) = Office::with_signature_field();
    let text = NewField {
        kind: NewFieldKind::Text { multiline: false },
        name: "Name".to_string(),
        rect: Rect::new(20.0, 300.0, 150.0, 320.0),
    };
    let text = office.document.add_field(office.page, &text, &office.library).unwrap();
    let identity = identity("test-signer.p12");
    assert_eq!(office.document.sign_field(text, &identity, &details()).unwrap_err().kind, ErrorKind::Unsupported);
    office.document.sign_field(field, &identity, &details()).unwrap();
    assert_eq!(office.document.sign_field(field, &identity, &details()).unwrap_err().kind, ErrorKind::Unsupported);
    assert!(Office::open(fixtures::editing_features()).document.signatures().unwrap().is_empty());
}

#[test]
#[ignore = "writes signed files to PREVIEW_DIR for checking with openssl and pdfsig"]
fn write_signed_preview() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let (mut office, field) = Office::with_signature_field();
    let signed = office.sign(field, "test-signer.p12");
    std::fs::write(format!("{directory}/signed-digitally.pdf"), &signed).unwrap();
    let note = Markup::Note { at: (40.0, 40.0), text: "Checked after signing".to_string() };
    let style = MarkupStyle { color: [0.9, 0.7, 0.1], width: 1.0, author: "Reviewer".to_string(), date: String::new() };
    office.document.add_markup(office.page, &note, &style, &office.library).unwrap();
    std::fs::write(format!("{directory}/signed-then-noted.pdf"), office.document.save(false).unwrap()).unwrap();
}
