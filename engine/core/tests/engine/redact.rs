use crate::support;

use support::fixtures;
use support::rendering::library;
use tintenfisch::annotations::{Markup, MarkupStyle};
use tintenfisch::document::Document;
use tintenfisch::error::ErrorKind;
use tintenfisch::fonts::FontLibrary;
use tintenfisch::image::{ImageDecoder, ImageDescription};
use tintenfisch::pages::PageId;
use tintenfisch::syntax::{Dict, Object, decode_text};
use tintenfisch::text::TextPosition;

const SECRETS: [&str; 7] =
    ["4711", "0815", "SECRET-ALT", "Form secret", "Hidden secret", "top secret", "Note on the secret"];
const AREAS: [(f64, f64, f64, f64); 3] =
    [(109.0, 352.0, 220.0, 374.0), (260.0, 170.0, 320.0, 270.0), (10.0, 50.0, 400.0, 145.0)];

struct Sheet {
    document: Document,
    library: FontLibrary,
    page: PageId,
}

impl Sheet {
    fn open(bytes: Vec<u8>) -> Self {
        let library = library();
        let document = Document::open(bytes, "").unwrap();
        let page = document.view(&library).pages().unwrap()[0].id;
        Self { document, library, page }
    }

    fn redacted() -> Self {
        let mut sheet = Sheet::open(fixtures::redaction_features());
        sheet.document.apply_redactions(&sheet.library).unwrap();
        sheet
    }

    fn text(&self) -> String {
        let layout = self.document.view(&self.library).text_layout(self.page).unwrap();
        layout
            .lines()
            .iter()
            .map(|line| line.clusters.iter().map(|cluster| cluster.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render(&self) -> (u32, Vec<u8>) {
        let raster = self.document.view(&self.library).render(self.page, 1.0, None).unwrap();
        (raster.width, raster.rgba)
    }

    fn everything(&self) -> String {
        let store = self.document.store();
        let mut found = String::new();
        for id in store.ids() {
            let object = store.object(id).unwrap();
            collect_strings(&object, &mut found);
            if let Some(stream) = object.as_stream()
                && let Ok(decoded) = store.decode_stream(stream)
            {
                found.push_str(&String::from_utf8_lossy(&decoded.data));
            }
        }
        found
    }

    fn page_dict(&self) -> Dict {
        self.document.store().object(self.page.0).unwrap().as_dict().cloned().unwrap()
    }
}

fn collect_strings(object: &Object, found: &mut String) {
    match object {
        Object::String(bytes) => {
            found.push_str(&decode_text(bytes));
            found.push('\n');
        }
        Object::Array(items) => items.iter().for_each(|item| collect_strings(item, found)),
        Object::Dict(dict) => dict.iter().for_each(|(_, value)| collect_strings(value, found)),
        Object::Stream(stream) => stream.dict.iter().for_each(|(_, value)| collect_strings(value, found)),
        _ => {}
    }
}

fn inside_an_area(x: f64, y: f64, margin: f64) -> bool {
    AREAS.iter().any(|(x0, y0, x1, y1)| x >= x0 - margin && x <= x1 + margin && y >= y0 - margin && y <= y1 + margin)
}

#[test]
fn the_fixture_shows_its_secrets_before() {
    let sheet = Sheet::open(fixtures::redaction_features());
    let everything = sheet.everything();
    for secret in SECRETS {
        assert!(everything.contains(secret), "{secret} is in the fixture");
    }
}

#[test]
fn covered_text_vanishes_from_every_stream_and_from_extraction() {
    let sheet = Sheet::redacted();
    let reopened = Sheet::open(sheet.document.save(false).unwrap());
    let everything = reopened.everything();
    let text = reopened.text();
    for secret in SECRETS {
        assert!(!everything.contains(secret), "{secret} survived in the file");
        assert!(!text.contains(secret), "{secret} is still extracted");
    }
    assert!(text.contains("Account number") && text.contains("Public line stays"), "{text}");
    assert!(!everything.contains("350 120 l"), "a path drawn wholly inside an area is removed");
    assert!(everything.contains("330 130 40 30 re"), "a path reaching out of the areas stays");
}

#[test]
fn everything_outside_the_areas_stays_pixel_identical_and_the_areas_are_filled() {
    let before = Sheet::open(fixtures::redaction_features()).render();
    let after = Sheet::redacted().render();
    let width = before.0 as usize;
    for (index, (a, b)) in before.1.chunks(4).zip(after.1.chunks(4)).enumerate() {
        let (x, y) = ((index % width) as f64 + 0.5, 400.0 - (index / width) as f64 - 0.5);
        if !inside_an_area(x, y, 1.0) {
            assert_eq!(a, b, "pixel at {x}, {y} changed outside the areas");
        }
        if x > 112.0 && x < 217.0 && y > 355.0 && y < 371.0 {
            assert_eq!(b, [0, 0, 0, 255], "the first area is filled black at {x}, {y}");
        }
        if y > 53.0 && y < 142.0 && x > 13.0 && x < 397.0 {
            assert_eq!(b, [51, 51, 51, 255], "the third area takes its interior color at {x}, {y}");
        }
    }
}

#[test]
fn covered_pixels_of_images_are_erased_in_the_file() {
    let sheet = Sheet::redacted();
    let store = sheet.document.store();
    let resources = store.resolve(sheet.page_dict().get("Resources").unwrap()).unwrap().as_dict().cloned().unwrap();
    let xobjects = store.resolve(resources.get("XObject").unwrap()).unwrap().as_dict().cloned().unwrap();
    let image = store.object(xobjects.get("Img").and_then(Object::as_reference).unwrap()).unwrap();
    let stream = image.as_stream().unwrap();
    let description = ImageDescription::read(&stream.dict, &resources, store).unwrap();
    let decoded = ImageDecoder::new(store, &resources).decode(&description, &stream.raw).unwrap();
    let pixel = |x: usize, y: usize| decoded.rgba[(y * 8 + x) * 4..(y * 8 + x) * 4 + 3].to_vec();
    assert_eq!(pixel(7, 4), [0, 0, 0], "the covered half is black");
    assert_eq!(pixel(1, 4), [65, 140, 200], "the uncovered half keeps its pixels");
    let inline =
        xobjects.iter().find(|(name, _)| String::from_utf8_lossy(name.as_bytes()).starts_with("TfRedactedImage"));
    assert!(inline.is_some(), "the covered inline image became an erased XObject");
    let content = sheet.everything();
    assert!(!content.contains(" ID "), "no inline image is left in the content");
}

#[test]
fn covered_annotations_fields_and_bookmark_titles_go() {
    let sheet = Sheet::redacted();
    let store = sheet.document.store();
    let annotations: Vec<String> = sheet
        .page_dict()
        .get("Annots")
        .and_then(Object::as_array)
        .unwrap()
        .iter()
        .map(|entry| {
            store
                .resolve(entry)
                .unwrap()
                .as_dict()
                .unwrap()
                .name("Subtype")
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                .unwrap()
        })
        .collect();
    assert_eq!(annotations, ["Link"], "the note, the field and the marks are gone, the link stays");
    assert!(sheet.document.fields().unwrap().is_empty());
    let everything = sheet.everything();
    assert!(everything.contains("Public section") && everything.contains("Redacted"));
}

#[test]
fn applying_forces_a_fresh_file_and_undo_brings_everything_back() {
    let original = fixtures::redaction_features();
    let mut sheet = Sheet::redacted();
    assert!(sheet.document.store().forces_rewrite());
    let saved = sheet.document.save(false).unwrap();
    assert!(!String::from_utf8_lossy(&saved).contains("Account number 4711"), "the old revision is not kept");
    sheet.document.undo().unwrap();
    assert!(sheet.document.save(false).unwrap() == original);
    let error = Sheet::open(fixtures::editing_features()).document.apply_redactions(&library()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
}

#[test]
fn marks_made_from_a_selection_and_a_dragged_area_redact_what_they_cover() {
    let mut sheet = Sheet::open(fixtures::reflow_features());
    let style = MarkupStyle { color: [0.9, 0.1, 0.1], width: 1.0, author: "Reviewer".to_string(), date: String::new() };
    let word =
        Markup::RedactText { anchor: TextPosition { line: 0, offset: 0 }, focus: TextPosition { line: 0, offset: 5 } };
    let area = Markup::RedactArea { from: (15.0, 96.0), to: (77.0, 112.0) };
    for markup in [word, area] {
        sheet.document.add_markup(sheet.page, &markup, &style, &sheet.library).unwrap();
    }
    let kinds: Vec<String> =
        sheet.document.annotations(sheet.page).unwrap().iter().map(|summary| summary.subtype.clone()).collect();
    assert_eq!(kinds, ["Redact", "Redact"]);
    assert!(sheet.text().starts_with("Lines of one"), "{}", sheet.text());
    sheet.document.apply_redactions(&sheet.library).unwrap();
    let text = sheet.text();
    assert!(!text.contains("Lines") && !text.contains("Positioned"), "{text}");
    assert!(text.contains("of one block wrap") && text.contains("after the block"), "{text}");
    assert!(sheet.document.annotations(sheet.page).unwrap().is_empty());
}

#[test]
#[ignore = "writes a redacted page to PREVIEW_DIR for comparing with other viewers"]
fn write_redacted_preview() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    std::fs::write(format!("{directory}/unredacted.pdf"), fixtures::redaction_features()).unwrap();
    let sheet = Sheet::redacted();
    std::fs::write(format!("{directory}/redacted.pdf"), sheet.document.save(false).unwrap()).unwrap();
    let raster = sheet.document.view(&sheet.library).render(sheet.page, 2.0, None).unwrap();
    std::fs::write(format!("{directory}/redacted.png"), support::rendering::to_png(&raster)).unwrap();
}
