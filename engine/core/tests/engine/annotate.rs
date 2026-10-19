use crate::support;

use support::fixtures;
use support::rendering::library;
use tintenfisch::annotations::{Markup, MarkupStyle};
use tintenfisch::document::Document;
use tintenfisch::fonts::FontLibrary;
use tintenfisch::pages::PageId;
use tintenfisch::syntax::Object;
use tintenfisch::text::TextPosition;

const RED: [f64; 3] = [0.85, 0.1, 0.1];

fn style() -> MarkupStyle {
    MarkupStyle { color: RED, width: 2.0, author: "Reviewer".to_string(), date: "D:20261018120000Z".to_string() }
}

struct Sheet {
    document: Document,
    library: FontLibrary,
}

impl Sheet {
    fn open(bytes: Vec<u8>) -> Self {
        Self { document: Document::open(bytes, "").unwrap(), library: library() }
    }

    fn page(&self, index: usize) -> PageId {
        self.document.view(&self.library).pages().unwrap()[index].id
    }

    fn render(&self, page: PageId) -> (u32, Vec<u8>) {
        let raster = self.document.view(&self.library).render(page, 1.0, None).unwrap();
        (raster.width, raster.rgba)
    }

    fn add(&mut self, page: PageId, markup: &Markup) -> tintenfisch::syntax::ObjId {
        self.document.add_markup(page, markup, &style(), &self.library).unwrap()
    }
}

fn changed_area((width, before): &(u32, Vec<u8>), (_, after): &(u32, Vec<u8>)) -> Option<(u32, u32, u32, u32)> {
    let mut area: Option<(u32, u32, u32, u32)> = None;
    for (index, (a, b)) in before.chunks(4).zip(after.chunks(4)).enumerate() {
        if a != b {
            let (x, y) = (index as u32 % width, index as u32 / width);
            area = Some(match area {
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                None => (x, y, x, y),
            });
        }
    }
    area
}

fn kinds() -> Vec<(&'static str, Markup)> {
    vec![
        ("Text", Markup::Note { at: (40.0, 60.0), text: "Check this".to_string() }),
        ("Ink", Markup::Ink { strokes: vec![vec![(30.0, 120.0), (60.0, 140.0), (90.0, 118.0)], vec![(100.0, 100.0)]] }),
        ("Square", Markup::Rectangle { from: (120.0, 40.0), to: (200.0, 90.0) }),
        ("Circle", Markup::Ellipse { from: (120.0, 120.0), to: (220.0, 170.0) }),
        ("Line", Markup::Arrow { from: (30.0, 220.0), to: (150.0, 190.0) }),
        ("Stamp", Markup::Stamp { at: (200.0, 250.0), label: "Approved".to_string() }),
    ]
}

#[test]
fn every_kind_draws_inside_its_rectangle_lists_saves_and_undoes() {
    for (subtype, markup) in kinds() {
        let original = fixtures::organize_features();
        let mut sheet = Sheet::open(original.clone());
        let page = sheet.page(0);
        let before = sheet.render(page);
        let id = sheet.add(page, &markup);
        let after = sheet.render(page);
        let summary =
            sheet.document.annotations(page).unwrap().into_iter().find(|annotation| annotation.id == id).unwrap();
        assert_eq!(summary.subtype, subtype);
        assert_eq!((summary.author.as_str(), summary.color), ("Reviewer", Some(RED)));
        let (x0, y0, x1, y1) = changed_area(&before, &after).unwrap_or_else(|| panic!("{subtype} draws nothing"));
        let rect = summary.rect;
        assert!(
            x0 as f64 >= rect.x0 - 1.0
                && y0 as f64 >= rect.y0 - 1.0
                && x1 as f64 <= rect.x1 + 1.0
                && y1 as f64 <= rect.y1 + 1.0,
            "{subtype} draws outside its rectangle: {:?} in {rect:?}",
            (x0, y0, x1, y1)
        );

        let reopened = Sheet::open(sheet.document.save(false).unwrap());
        assert_eq!(
            reopened.document.annotations(reopened.page(0)).unwrap().len(),
            1,
            "{subtype} did not survive saving"
        );
        assert!(reopened.render(reopened.page(0)) == after, "{subtype} looks different after saving");

        sheet.document.undo().unwrap();
        assert!(sheet.render(page) == before);
        assert!(sheet.document.save(false).unwrap() == original, "{subtype}: undo left changes behind");
    }
}

#[test]
fn a_highlight_covers_the_selected_lines_and_quotes_them() {
    let fixture = fixtures::all().into_iter().find(|fixture| fixture.name == "chrome-article.pdf").unwrap();
    let mut sheet = Sheet::open(fixture.bytes);
    let page = sheet.page(0);
    let markup =
        Markup::Highlight { anchor: TextPosition { line: 1, offset: 4 }, focus: TextPosition { line: 2, offset: 6 } };
    let before = sheet.render(page);
    let id = sheet.add(page, &markup);
    let summary = &sheet.document.annotations(page).unwrap()[0];
    assert_eq!(summary.id, id);
    assert!(
        summary.contents.starts_with("portable document format") && summary.contents.ends_with("state."),
        "{:?}",
        summary.contents
    );
    let annotation = sheet.document.store().object(id).unwrap();
    let quads = annotation.as_dict().unwrap().get("QuadPoints").and_then(Object::as_array).unwrap().len();
    assert_eq!(quads, 16, "one quad per selected line");
    let (width, after) = sheet.render(page);
    let rect = summary.rect;
    let inside = |rgba: &[u8]| {
        rgba.chunks(4)
            .enumerate()
            .filter(|(index, _)| {
                let (x, y) = ((*index as u32 % width) as f64, (*index as u32 / width) as f64);
                x >= rect.x0 && x <= rect.x1 && y >= rect.y0 && y <= rect.y1
            })
            .map(|(_, pixel)| pixel.to_vec())
            .collect::<Vec<_>>()
    };
    let (before_inside, after_inside) = (inside(&before.1), inside(&after));
    let tinted = before_inside.iter().zip(&after_inside).filter(|(a, b)| b[2] as i32 + 40 < a[2] as i32).count();
    assert!(tinted * 3 > before_inside.len(), "most of the highlighted area takes the color");
    let dark = |pixels: &[Vec<u8>]| pixels.iter().filter(|pixel| pixel[0] < 70 && pixel[1] < 70).count();
    assert!(dark(&before_inside) > 100);
    assert!(dark(&after_inside) * 10 >= dark(&before_inside) * 9, "the text stays readable under the highlight");
}

fn highlight(from: (usize, usize), to: (usize, usize)) -> Markup {
    Markup::Highlight {
        anchor: TextPosition { line: from.0, offset: from.1 },
        focus: TextPosition { line: to.0, offset: to.1 },
    }
}

fn highlights(sheet: &Sheet, page: PageId) -> Vec<(tintenfisch::syntax::ObjId, String, Option<[f64; 3]>)> {
    sheet
        .document
        .annotations(page)
        .unwrap()
        .into_iter()
        .filter(|summary| summary.subtype == "Highlight")
        .map(|summary| (summary.id, summary.contents, summary.color))
        .collect()
}

fn article() -> (Sheet, PageId) {
    let fixture = fixtures::all().into_iter().find(|fixture| fixture.name == "chrome-article.pdf").unwrap();
    let sheet = Sheet::open(fixture.bytes);
    let page = sheet.page(0);
    (sheet, page)
}

#[test]
fn highlighting_highlighted_text_again_in_the_same_color_changes_nothing() {
    let (mut sheet, page) = article();
    let first = sheet.add(page, &highlight((1, 4), (2, 6)));
    let saved = sheet.document.save(false).unwrap();
    let again = sheet.add(page, &highlight((1, 8), (1, 20)));
    assert_eq!(again, first);
    assert_eq!(highlights(&sheet, page).len(), 1);
    assert!(sheet.document.save(false).unwrap() == saved, "nothing was written");
}

#[test]
fn highlighting_the_same_text_in_another_color_recolors_it() {
    let (mut sheet, page) = article();
    sheet.add(page, &highlight((1, 4), (2, 6)));
    let yellow = MarkupStyle { color: [1.0, 0.85, 0.1], ..style() };
    sheet.document.add_markup(page, &highlight((1, 4), (2, 6)), &yellow, &sheet.library).unwrap();
    let found = highlights(&sheet, page);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].2, Some([1.0, 0.85, 0.1]));
}

#[test]
fn overlapping_highlights_of_one_color_merge_and_keep_a_written_note() {
    let (mut sheet, page) = article();
    let first = sheet.add(page, &highlight((1, 4), (1, 20)));
    sheet.document.set_annotation_contents(first, "Look this up", "D:20261019120000Z", &sheet.library).unwrap();
    sheet.add(page, &highlight((1, 12), (2, 6)));
    let found = highlights(&sheet, page);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1, "Look this up");
    let quads = sheet.document.store().object(found[0].0).unwrap();
    let quads = quads.as_dict().unwrap().get("QuadPoints").and_then(Object::as_array).unwrap().len();
    assert_eq!(quads, 16, "the merged highlight spans both lines");
    sheet.document.undo().unwrap();
    assert_eq!(highlights(&sheet, page).len(), 1, "one undo brings back the first highlight alone");
}

#[test]
fn a_highlight_in_another_color_that_only_overlaps_stays() {
    let (mut sheet, page) = article();
    sheet.add(page, &highlight((1, 4), (1, 20)));
    let yellow = MarkupStyle { color: [1.0, 0.85, 0.1], ..style() };
    sheet.document.add_markup(page, &highlight((1, 12), (2, 6)), &yellow, &sheet.library).unwrap();
    assert_eq!(highlights(&sheet, page).len(), 2);
}

#[test]
fn notes_and_stamps_stand_upright_on_a_turned_page() {
    let mut sheet = Sheet::open(fixtures::organize_features());
    let page = sheet.page(1);
    let stamp = sheet.add(page, &Markup::Stamp { at: (150.0, 150.0), label: "Draft".to_string() });
    let summary =
        sheet.document.annotations(page).unwrap().into_iter().find(|annotation| annotation.id == stamp).unwrap();
    assert!(
        summary.rect.width() > summary.rect.height() * 1.5,
        "the stamp reads across the shown page: {:?}",
        summary.rect
    );
}

#[test]
fn deleting_a_note_takes_its_popup_along_and_contents_can_change() {
    let mut sheet = Sheet::open(fixtures::organize_features());
    let page = sheet.page(2);
    let note = sheet.add(page, &Markup::Note { at: (20.0, 20.0), text: "First thought".to_string() });
    let annots = |sheet: &Sheet| {
        let page = sheet.document.store().object(page.0).unwrap();
        page.as_dict().unwrap().get("Annots").and_then(Object::as_array).map_or(0, <[Object]>::len)
    };
    assert_eq!(annots(&sheet), 2, "note and popup");
    let redrawn = sheet
        .document
        .set_annotation_contents(note, "Grüße, second thought", "D:20261018130000Z", &sheet.library)
        .unwrap();
    assert!(redrawn.is_empty(), "a note keeps its icon");
    assert_eq!(sheet.document.annotations(page).unwrap()[0].contents, "Grüße, second thought");
    sheet.document.delete_annotation(page, note).unwrap();
    assert_eq!(annots(&sheet), 0);
    assert!(sheet.document.annotations(page).unwrap().is_empty());
}

#[test]
#[ignore = "writes an annotated document to PREVIEW_DIR for comparing with other viewers"]
fn write_annotated_preview() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let fixture = fixtures::all().into_iter().find(|fixture| fixture.name == "chrome-article.pdf").unwrap();
    let mut sheet = Sheet::open(fixture.bytes);
    let page = sheet.page(0);
    let yellow = MarkupStyle { color: [1.0, 0.85, 0.1], ..style() };
    let highlight =
        Markup::Highlight { anchor: TextPosition { line: 1, offset: 4 }, focus: TextPosition { line: 2, offset: 6 } };
    sheet.document.add_markup(page, &highlight, &yellow, &sheet.library).unwrap();
    for (_, markup) in kinds() {
        let moved = match markup {
            Markup::Note { at, text } => Markup::Note { at: (at.0 + 300.0, at.1 + 300.0), text },
            Markup::Ink { strokes } => Markup::Ink {
                strokes: strokes
                    .into_iter()
                    .map(|stroke| stroke.into_iter().map(|(x, y)| (x + 40.0, y + 380.0)).collect())
                    .collect(),
            },
            Markup::Rectangle { from, to } => {
                Markup::Rectangle { from: (from.0 + 200.0, from.1 + 380.0), to: (to.0 + 200.0, to.1 + 380.0) }
            }
            Markup::Ellipse { from, to } => {
                Markup::Ellipse { from: (from.0 + 260.0, from.1 + 380.0), to: (to.0 + 260.0, to.1 + 380.0) }
            }
            Markup::Arrow { from, to } => {
                Markup::Arrow { from: (from.0 + 40.0, from.1 + 380.0), to: (to.0 + 40.0, to.1 + 380.0) }
            }
            Markup::Stamp { at, label } => Markup::Stamp { at: (at.0 + 200.0, at.1 + 400.0), label },
            other => other,
        };
        sheet.add(page, &moved);
    }
    std::fs::write(format!("{directory}/annotated.pdf"), sheet.document.save(false).unwrap()).unwrap();
    let raster = sheet.document.view(&sheet.library).render(page, 1.5, None).unwrap();
    std::fs::write(format!("{directory}/annotated.png"), support::rendering::to_png(&raster)).unwrap();
}
