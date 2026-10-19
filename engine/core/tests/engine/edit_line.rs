use crate::support;

use support::fixtures;
use support::rendering::library;
use tintenfisch::content::ContentStream;
use tintenfisch::document::Document;
use tintenfisch::edit::ParagraphEdit;
use tintenfisch::fonts::FontLibrary;
use tintenfisch::pages::PageId;
use tintenfisch::syntax::Object;
use tintenfisch::text::{LayoutLine, TextGlyph};

struct Editing {
    document: Document,
    library: FontLibrary,
    pages: Vec<PageId>,
}

impl Editing {
    fn new(bytes: Vec<u8>) -> Self {
        let library = library();
        let document = Document::open(bytes, "").unwrap();
        let pages = document.view(&library).pages().unwrap().iter().map(|page| page.id).collect();
        Self { document, library, pages }
    }

    fn lines(&self, page: usize) -> Vec<LayoutLine> {
        self.document.view(&self.library).text_layout(self.pages[page]).unwrap().lines().to_vec()
    }

    fn line_index(&self, page: usize, text: &str) -> usize {
        self.lines(page).iter().position(|line| line_text(line) == text).unwrap_or_else(|| panic!("no line {text:?}"))
    }

    fn glyphs(&self, page: usize) -> Vec<TextGlyph> {
        self.document.view(&self.library).text_glyphs(self.pages[page]).unwrap()
    }

    fn edit(&mut self, page: usize, line: &str, text: &str) {
        let edit = ParagraphEdit::new(self.pages[page], self.line_index(page, line), text);
        self.document.edit(&edit, &self.library, "caret").unwrap();
    }

    fn render(&self, page: usize) -> Vec<u8> {
        self.document.view(&self.library).render(self.pages[page], 1.0, None).unwrap().rgba
    }

    fn content(&self, page: usize) -> ContentStream {
        let view = self.document.view(&self.library);
        ContentStream::load(self.document.store(), &view.page(self.pages[page]).unwrap()).unwrap()
    }
}

fn line_text(line: &LayoutLine) -> String {
    line.clusters.iter().map(|cluster| cluster.text.as_str()).collect()
}

fn same_place(a: &TextGlyph, b: &TextGlyph) -> bool {
    (a.start - b.start).abs() < 1e-6
        && (a.baseline - b.baseline).abs() < 1e-6
        && (a.end - b.end).abs() < 1e-6
        && a.text == b.text
}

const EDITS: [(&str, &str); 9] = [
    ("Plain line to edit", "Plain line we edited"),
    ("Condensed line", "Condensed lines"),
    ("Rotated line", "Rotated lines"),
    ("Clipped words here", "Clipped word here"),
    ("Marked text", "Marked texts"),
    ("Invisible OCR layer", "Invisible OCR layers"),
    ("Second stream line", "2nd stream line"),
    ("Kerned runs", "Ker runs"),
    ("First continued", "Firsts continued"),
];

#[test]
fn every_edited_line_reads_as_its_new_text_and_no_other_glyph_moves() {
    for (before, after) in EDITS {
        let mut editing = Editing::new(fixtures::editing_features());
        let edited_line = editing.line_index(0, before);
        let lines_before = editing.lines(0);
        let glyphs_before = editing.glyphs(0);
        editing.edit(0, before, after);
        let lines_after = editing.lines(0);
        assert_eq!(line_text(&lines_after[edited_line]), after, "{before:?}");
        for (index, line) in lines_before.iter().enumerate().filter(|(index, _)| *index != edited_line) {
            assert_eq!(line_text(&lines_after[index]), line_text(line), "{before:?} changed line {index}");
        }
        let untouched: Vec<&TextGlyph> = glyphs_before
            .iter()
            .filter(|glyph| {
                !lines_before[edited_line].clusters.iter().any(|cluster| cluster.origins.contains(&glyph.origin))
            })
            .collect();
        let glyphs_after = editing.glyphs(0);
        for glyph in untouched {
            assert!(glyphs_after.iter().any(|after| same_place(glyph, after)), "{before:?}: {:?} moved", glyph.text);
        }
    }
}

#[test]
fn surviving_glyphs_keep_their_place_and_the_rest_of_the_line_moves_as_one() {
    let mut editing = Editing::new(fixtures::editing_features());
    let line = editing.line_index(0, "First continued");
    let before = editing.lines(0)[line].clone();
    editing.edit(0, "First continued", "Firsts continued");
    let after = editing.lines(0)[line].clone();
    for index in 0..5 {
        assert!((after.clusters[index].left - before.clusters[index].left).abs() < 1e-6, "prefix glyph {index} moved");
    }
    let shift = after.clusters[6].left - before.clusters[5].left;
    assert!(shift > 4.0, "the suffix makes room for the inserted letter");
    let inserted = &after.clusters[5];
    assert!((after.clusters[6].left - inserted.right).abs() < 1e-6, "the suffix starts where the insertion ends");
    assert!((shift - (inserted.right - inserted.left)).abs() < 1e-6, "the suffix moves by exactly the inserted width");
    for index in 5..before.clusters.len() {
        let moved = after.clusters[index + 1].left - before.clusters[index].left;
        assert!((moved - shift).abs() < 1e-6, "suffix glyph {index} moved by {moved}, not {shift}");
    }
}

#[test]
fn bytes_outside_the_rewritten_operations_stay_identical() {
    for (before, after) in EDITS {
        let mut editing = Editing::new(fixtures::editing_features());
        let old = editing.content(0);
        editing.edit(0, before, after);
        let new = editing.content(0);
        let edited_ordinals: Vec<usize> = old
            .operations
            .iter()
            .filter(|operation| {
                !new.operations.iter().any(|candidate| {
                    candidate.operator == operation.operator && candidate.operands == operation.operands
                })
            })
            .map(|operation| operation.ordinal)
            .collect();
        assert!(!edited_ordinals.is_empty(), "{before:?}: nothing was rewritten");
        for (segment, old_segment) in old.segments.iter().enumerate() {
            let new_data = &new.segments[segment].data;
            let mut cursor = 0;
            for operation in old
                .operations
                .iter()
                .filter(|operation| operation.segment == segment && !edited_ordinals.contains(&operation.ordinal))
            {
                let bytes = &old_segment.data[operation.span.clone()];
                let found = new_data[cursor..].windows(bytes.len()).position(|window| window == bytes);
                let found = found.unwrap_or_else(|| panic!("{before:?}: {:?} missing", String::from_utf8_lossy(bytes)));
                cursor += found + bytes.len();
            }
        }
    }
}

#[test]
fn a_page_sharing_the_stream_keeps_it_untouched() {
    let mut editing = Editing::new(fixtures::editing_features());
    let other_before = editing.render(1);
    let shared = editing.content(1).segments[0].stream;
    editing.edit(0, "Plain line to edit", "Plain line edited");
    assert_eq!(editing.render(1), other_before);
    assert_eq!(editing.content(1).segments[0].stream, shared);
    assert_ne!(editing.content(0).segments[0].stream, shared, "the edited page got its own copy");
    assert_eq!(line_text(&editing.lines(1)[0]), "Plain line to edit");
}

#[test]
fn invisible_text_stays_invisible() {
    let mut editing = Editing::new(fixtures::editing_features());
    let picture = editing.render(0);
    editing.edit(0, "Invisible OCR layer", "Invisible OCR text layer");
    assert_eq!(editing.render(0), picture);
    let line = editing.line_index(0, "Invisible OCR text layer");
    let glyphs = editing.glyphs(0);
    let origins = &editing.lines(0)[line].clusters;
    assert!(
        glyphs
            .iter()
            .filter(|glyph| origins.iter().any(|cluster| cluster.origins.contains(&glyph.origin)))
            .all(|glyph| glyph.hidden)
    );
}

#[test]
fn rotated_text_stays_on_its_baseline_angle() {
    let mut editing = Editing::new(fixtures::editing_features());
    let angle = editing.lines(0)[editing.line_index(0, "Rotated line")].angle;
    assert!((angle.to_degrees() + 30.0).abs() < 0.1, "fixture is turned by 30 degrees, got {}", angle.to_degrees());
    editing.edit(0, "Rotated line", "Rotated lines");
    let line = &editing.lines(0)[editing.line_index(0, "Rotated lines")];
    assert!((line.angle - angle).abs() < 1e-9);
}

#[test]
fn the_preview_shows_exactly_what_the_commit_writes() {
    for (before, after) in EDITS {
        let mut editing = Editing::new(fixtures::editing_features());
        let edit = ParagraphEdit::new(editing.pages[0], editing.line_index(0, before), after);
        let untouched = editing.render(0);
        let previewed = editing
            .document
            .preview_edit(&edit, &editing.library, |view| view.render(edit.page, 1.0, None).unwrap().rgba)
            .unwrap();
        assert_eq!(editing.render(0), untouched, "{before:?}: the preview leaked into the document");
        editing.document.edit(&edit, &editing.library, "").unwrap();
        assert!(editing.render(0) == previewed, "{before:?}: preview and commit differ");
    }
}

#[test]
fn undo_returns_to_the_original_file() {
    let original = fixtures::editing_features();
    let mut editing = Editing::new(original.clone());
    editing.edit(0, "Kerned runs", "Kerned and spaced runs");
    let undone = editing.document.undo().unwrap();
    assert_eq!(undone.ui_state, "caret");
    assert_eq!(editing.document.save(false).unwrap(), original);
    editing.document.redo().unwrap();
    assert_eq!(line_text(&editing.lines(0)[editing.line_index(0, "Kerned and spaced runs")]), "Kerned and spaced runs");
}

#[test]
fn a_character_the_font_lacks_comes_from_an_embedded_fallback() {
    let mut editing = Editing::new(fixtures::editing_features());
    let picture = editing.render(0);
    editing.edit(0, "Plain line to edit", "Plain line to edit Ω");
    assert!(editing.lines(0).iter().any(|line| line_text(line) == "Plain line to edit Ω"));
    assert_ne!(editing.render(0), picture, "the fallback glyph is drawn");
    let saved = editing.document.save(false).unwrap();
    let reopened = Editing::new(saved);
    assert!(reopened.lines(0).iter().any(|line| line_text(line) == "Plain line to edit Ω"));
    let view = reopened.document.view(&reopened.library);
    let fonts = view.page(reopened.pages[0]).unwrap().resources.get("Font").cloned().unwrap();
    let fonts = reopened.document.store().resolve(&fonts).unwrap();
    assert!(fonts.as_dict().unwrap().get("TfFallback0").is_some_and(|font| matches!(font, Object::Ref(_))));
}

#[test]
fn a_character_the_font_lacks_comes_from_its_registered_replacement_first() {
    let fixture = fixtures::all().into_iter().find(|fixture| fixture.name == "chrome-article.pdf").unwrap();
    let mut editing = Editing::new(fixture.bytes);
    let replacement = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../public/fonts/LiberationSerif-Bold.ttf"),
    )
    .unwrap();
    for font in editing.document.view(&editing.library).fonts(editing.pages[0]).unwrap() {
        if font.embedded {
            editing.library.register_replacement(&font.base_font, replacement.clone());
        }
    }
    let first = line_text(&editing.lines(0)[0]);
    editing.edit(0, &first, &format!("{first} Ω"));
    let view = editing.document.view(&editing.library);
    let fonts = view.page(editing.pages[0]).unwrap().resources.get("Font").cloned().unwrap();
    let fonts = editing.document.store().resolve(&fonts).unwrap();
    let base_fonts: Vec<String> = fonts
        .as_dict()
        .unwrap()
        .iter()
        .filter_map(|(_, font)| editing.document.store().resolve(font).ok())
        .filter_map(|font| {
            font.as_dict()
                .and_then(|dict| dict.name("BaseFont"))
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
        })
        .collect();
    assert!(base_fonts.iter().any(|name| name.ends_with("+LiberationSerifBold")), "{base_fonts:?}");
    assert!(line_text(&editing.lines(0)[0]).ends_with('Ω'));
}
