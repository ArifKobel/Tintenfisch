use crate::support;

use support::fixtures;
use support::rendering::library;
use support::synthetic_pdf::{Row, SyntheticPdf};
use tintenfisch::document::Document;
use tintenfisch::edit::{NewParagraph, ParagraphEdit};
use tintenfisch::error::ErrorKind;
use tintenfisch::fonts::{FallbackFace, FallbackFamily, FontLibrary};
use tintenfisch::pages::PageId;
use tintenfisch::text::LayoutLine;

const SANS: FallbackFace = FallbackFace { family: FallbackFamily::Sans, bold: false, italic: false };

struct Writer {
    document: Document,
    library: FontLibrary,
    page: PageId,
}

impl Writer {
    fn open(bytes: Vec<u8>) -> Self {
        let library = library();
        let document = Document::open(bytes, "").unwrap();
        let page = document.view(&library).pages().unwrap()[0].id;
        Self { document, library, page }
    }

    fn add(&mut self, at: (f64, f64), text: &str) {
        let new = NewParagraph {
            page: self.page,
            at,
            face: SANS,
            catalog: None,
            size: 14.0,
            color: [0.1, 0.2, 0.6],
            text: text.to_string(),
        };
        let plan = self.document.plan_new_text(&new, &self.library).unwrap();
        self.document.apply(&plan, "").unwrap();
    }

    fn lines(&self) -> Vec<LayoutLine> {
        self.document.view(&self.library).text_layout(self.page).unwrap().lines().to_vec()
    }

    fn texts(&self) -> Vec<String> {
        self.lines().iter().map(text_of).collect()
    }

    fn line(&self, text: &str) -> LayoutLine {
        self.lines()
            .into_iter()
            .find(|line| text_of(line) == text)
            .unwrap_or_else(|| panic!("no line {text:?} in {:?}", self.texts()))
    }
}

fn text_of(line: &LayoutLine) -> String {
    line.clusters.iter().map(|cluster| cluster.text.as_str()).collect()
}

#[test]
fn added_text_is_a_paragraph_where_it_was_put_that_saves_edits_and_undoes() {
    let original = fixtures::editing_features();
    let mut writer = Writer::open(original.clone());
    writer.add((200.0, 300.0), "Hello\nWorld");
    let hello = writer.line("Hello");
    let world = writer.line("World");
    assert!((hello.left() - 200.0).abs() < 0.5, "{}", hello.left());
    assert!(hello.top > 299.0 && hello.top < 304.0, "{}", hello.top);
    assert!((world.baseline - hello.baseline - 14.0 * 1.2).abs() < 0.5);
    assert_eq!(hello.angle, 0.0);
    let reopened = Writer::open(writer.document.save(false).unwrap());
    reopened.line("Hello");
    let line = writer.lines().iter().position(|line| text_of(line) == "Hello").unwrap();
    let edit = ParagraphEdit::new(writer.page, line, "Hullo\nWorld");
    let plan = writer.document.plan_edit(&edit, &writer.library).unwrap();
    writer.document.apply(&plan, "").unwrap();
    writer.line("Hullo");
    let line = writer.lines().iter().position(|line| text_of(line) == "Hullo").unwrap();
    let wider = ParagraphEdit::new(writer.page, line, "Hullo there\nWorld").with_width(150.0);
    let plan = writer.document.plan_edit(&wider, &writer.library).unwrap();
    writer.document.apply(&plan, "").unwrap();
    writer.line("Hullo there");
    while writer.document.undo().is_some() {}
    assert!(writer.document.save(false).unwrap() == original);
}

#[test]
fn added_text_stands_upright_on_a_turned_page_and_ignores_a_transformation_left_open() {
    let mut turned = Writer::open(fixtures::nested_page_tree());
    turned.add((50.0, 60.0), "Upright");
    let line = turned.line("Upright");
    assert!(line.angle.abs() < 1e-6 && (line.left() - 50.0).abs() < 0.5, "{} {}", line.angle, line.left());
    let width = |line: &LayoutLine| line.clusters.last().unwrap().right - line.left();
    let expected = (667.0 + 222.0 + 556.0 + 222.0 + 556.0) / 1000.0 * 14.0;
    for content in [
        &b"2 0 0 2 0 0 cm BT /F1 12 Tf 10 10 Td (Shown) Tj ET"[..],
        b"q 2 0 0 2 0 0 cm BT /F1 12 Tf 10 10 Td (Shown) Tj ET",
        b"50 Tz 2 Tc BT /F1 12 Tf 10 10 Td (Shown) Tj ET",
    ] {
        let mut page = Writer::open(single_page(content));
        page.add((100.0, 100.0), "Plain");
        let line = page.line("Plain");
        let label = String::from_utf8_lossy(content);
        assert!(
            (line.left() - 100.0).abs() < 0.5 && (line.size - 14.0).abs() < 0.1,
            "{label}: {} {}",
            line.left(),
            line.size
        );
        assert!(
            (width(&line) - expected).abs() < 0.05,
            "{label}: the page's state stays out of the new text, {} wide",
            width(&line)
        );
        page.line("Shown");
    }
}

fn single_page(content: &[u8]) -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.6");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
        .object(3, 0, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>")
        .stream(4, "", content)
        .object(5, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    let rows: Vec<(u32, Row)> =
        std::iter::once((0, Row::Free(65535))).chain((1..=5).map(|num| (num, Row::InUse(num)))).collect();
    let start = pdf.xref_table(&rows, "/Size 6 /Root 1 0 R");
    pdf.startxref(start).finish()
}

#[test]
fn added_text_is_set_in_the_face_size_and_color_asked_for() {
    let mut writer = Writer::open(fixtures::editing_features());
    let face = FallbackFace { family: FallbackFamily::Serif, bold: true, italic: true };
    let new = NewParagraph {
        page: writer.page,
        at: (60.0, 500.0),
        face,
        catalog: None,
        size: 20.0,
        color: [0.8, 0.1, 0.1],
        text: "Styled".into(),
    };
    let plan = writer.document.plan_new_text(&new, &writer.library).unwrap();
    writer.document.apply(&plan, "").unwrap();
    let line = writer.texts().iter().position(|text| text == "Styled").unwrap();
    let paragraph = writer.document.paragraph_text(writer.page, line, &writer.library).unwrap();
    let style = &paragraph.styles[0];
    assert!(style.bold && style.italic, "{style:?}");
    assert!(style.font.contains("Serif"), "{}", style.font);
    assert!((style.size - 20.0).abs() < 1e-6);
    assert_eq!(style.color, Some([0.8, 0.1, 0.1]));
}

#[test]
fn empty_text_plans_nothing_but_a_caret_and_unknown_characters_are_refused() {
    let writer = Writer::open(fixtures::editing_features());
    let empty = NewParagraph {
        page: writer.page,
        at: (10.0, 10.0),
        face: SANS,
        catalog: None,
        size: 12.0,
        color: [0.0; 3],
        text: String::new(),
    };
    let plan = writer.document.plan_new_text(&empty, &writer.library).unwrap();
    assert!(plan.is_empty());
    let layout = writer.document.view(&writer.library).text_layout(writer.page).unwrap();
    let caret = plan.caret(&layout, 0).unwrap();
    assert!(
        (caret.x - 10.0).abs() < 1e-6 && (caret.top - 10.0).abs() < 1e-9 && caret.top < caret.bottom,
        "the caret starts at the point clicked"
    );
    let unknown = NewParagraph { text: "\u{1F600}".to_string(), ..empty };
    assert_eq!(writer.document.plan_new_text(&unknown, &writer.library).err().unwrap().kind, ErrorKind::Unsupported);
}

#[test]
fn a_text_box_comment_shows_its_text_and_is_drawn_anew_when_the_text_changes() {
    use tintenfisch::annotations::{Markup, MarkupStyle};
    use tintenfisch::syntax::Object;
    let original = fixtures::editing_features();
    let mut writer = Writer::open(original.clone());
    let render = |writer: &Writer| writer.document.view(&writer.library).render(writer.page, 1.0, None).unwrap().rgba;
    let before = render(&writer);
    let style = MarkupStyle { color: [0.8, 0.1, 0.1], width: 1.0, author: "Reviewer".to_string(), date: String::new() };
    let markup = Markup::TextBox {
        from: (200.0, 250.0),
        to: (360.0, 300.0),
        text: "Please check the figures in this table".to_string(),
    };
    let id = writer.document.add_markup(writer.page, &markup, &style, &writer.library).unwrap();
    let shown = render(&writer);
    let changed: Vec<usize> = before
        .chunks(4)
        .zip(shown.chunks(4))
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, _)| index)
        .collect();
    assert!(!changed.is_empty());
    assert!(
        changed.iter().all(|index| (199..=361).contains(&(index % 400)) && (249..=301).contains(&(index / 400))),
        "drawn inside the box"
    );
    let summary =
        writer.document.annotations(writer.page).unwrap().into_iter().find(|summary| summary.id == id).unwrap();
    assert_eq!(
        (summary.subtype.as_str(), summary.contents.as_str()),
        ("FreeText", "Please check the figures in this table")
    );
    let dict = writer.document.store().object(id).unwrap().as_dict().cloned().unwrap();
    assert_eq!(dict.get("DA").and_then(Object::as_string), Some(b"/Helv 12 Tf 0.8 0.1 0.1 rg".as_slice()));
    let appearance =
        dict.get("AP").and_then(Object::as_dict).and_then(|ap| ap.get("N")).and_then(Object::as_reference).unwrap();
    let appearance = writer.document.store().object(appearance).unwrap();
    let content =
        String::from_utf8_lossy(&writer.document.store().decode_stream(appearance.as_stream().unwrap()).unwrap().data)
            .into_owned();
    assert!(
        content.contains("(Please check the figures in) Tj") && content.contains("(this table) Tj"),
        "wrapped by words: {content}"
    );
    let redrawn = writer.document.set_annotation_contents(id, "Fine", "D:20261019000000Z", &writer.library).unwrap();
    assert_eq!(redrawn, [writer.page]);
    let rewritten = render(&writer);
    assert!(rewritten != shown && rewritten != before, "the box shows its new text");
    let reopened = Writer::open(writer.document.save(false).unwrap());
    assert!(render(&reopened) == rewritten);
    while writer.document.undo().is_some() {}
    assert!(writer.document.save(false).unwrap() == original);
}

#[test]
#[ignore = "writes added text and a text box to PREVIEW_DIR for comparing with other viewers"]
fn write_added_text_preview() {
    use tintenfisch::annotations::{Markup, MarkupStyle};
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let mut writer = Writer::open(fixtures::editing_features());
    writer.add((200.0, 300.0), "Added paragraph\nsecond line");
    let style = MarkupStyle { color: [0.8, 0.1, 0.1], width: 1.0, author: "Reviewer".to_string(), date: String::new() };
    let markup = Markup::TextBox {
        from: (200.0, 200.0),
        to: (360.0, 250.0),
        text: "Please check the figures in this table".to_string(),
    };
    writer.document.add_markup(writer.page, &markup, &style, &writer.library).unwrap();
    std::fs::write(format!("{directory}/added-text.pdf"), writer.document.save(false).unwrap()).unwrap();
    let raster = writer.document.view(&writer.library).render(writer.page, 2.0, None).unwrap();
    std::fs::write(format!("{directory}/added-text.png"), support::rendering::to_png(&raster)).unwrap();
}

#[test]
fn new_text_takes_a_registered_catalog_font() {
    let mut writer = Writer::open(fixtures::editing_features());
    let serif_bold = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../public/fonts/LiberationSerif-Bold.ttf"),
    )
    .unwrap();
    writer.library.register_replacement("fontsource:liberation-serif:700:normal", serif_bold);
    let new = NewParagraph {
        page: writer.page,
        at: (60.0, 520.0),
        face: SANS,
        catalog: Some("fontsource:liberation-serif:700:normal".to_string()),
        size: 18.0,
        color: [0.0; 3],
        text: "Catalog".into(),
    };
    let plan = writer.document.plan_new_text(&new, &writer.library).unwrap();
    writer.document.apply(&plan, "").unwrap();
    let line = writer.texts().iter().position(|text| text == "Catalog").unwrap();
    let paragraph = writer.document.paragraph_text(writer.page, line, &writer.library).unwrap();
    assert_eq!(paragraph.styles[0].font, "Liberation Serif Bold");
}
