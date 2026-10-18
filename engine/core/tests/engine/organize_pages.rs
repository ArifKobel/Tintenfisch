use crate::support;

use std::fs;
use std::path::PathBuf;

use support::fixtures;
use support::random::Random;
use support::rendering::library;
use tintenfisch::document::Document;
use tintenfisch::error::ErrorKind;
use tintenfisch::fonts::FontLibrary;
use tintenfisch::pages::PageId;
use tintenfisch::syntax::Object;

struct Pages {
    document: Document,
    library: FontLibrary,
}

impl Pages {
    fn open(bytes: Vec<u8>) -> Self {
        Self { document: Document::open(bytes, "").unwrap(), library: library() }
    }

    fn ids(&self) -> Vec<PageId> {
        self.document.view(&self.library).pages().unwrap().iter().map(|page| page.id).collect()
    }

    fn rotations(&self) -> Vec<u16> {
        self.document.view(&self.library).pages().unwrap().iter().map(|page| page.rotate).collect()
    }

    fn texts(&self) -> Vec<String> {
        let view = self.document.view(&self.library);
        self.ids()
            .iter()
            .map(|page| {
                view.text_layout(*page)
                    .unwrap()
                    .lines()
                    .iter()
                    .map(|line| line.clusters.iter().map(|cluster| cluster.text.as_str()).collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    }

    fn render(&self, page: PageId) -> Vec<u8> {
        self.document.view(&self.library).render(page, 1.0, None).unwrap().rgba
    }

    fn catalog(&self) -> tintenfisch::syntax::Dict {
        let store = self.document.store();
        let root = store.trailer().get("Root").and_then(Object::as_reference).unwrap();
        store.object(root).unwrap().as_dict().cloned().unwrap()
    }

    fn resolved(&self, object: &Object) -> Object {
        self.document.store().resolve(object).unwrap()
    }
}

fn encrypted(file: &str) -> Vec<u8> {
    fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/encrypted/{file}.pdf"))).unwrap()
}

#[test]
fn flattening_the_tree_leaves_every_page_as_it_was() {
    let original = Pages::open(fixtures::organize_features());
    let pictures: Vec<_> = original.ids().iter().map(|page| original.render(*page)).collect();
    let mut pages = Pages::open(fixtures::organize_features());
    let ids = pages.ids();
    pages.document.move_pages(&[ids[2]], Some(ids[0])).unwrap();
    pages.document.move_pages(&[ids[2]], None).unwrap();
    assert_eq!(pages.ids(), ids);
    assert_eq!(pages.rotations(), [0, 90, 90]);
    for (page, picture) in ids.iter().zip(&pictures) {
        assert!(pages.render(*page) == *picture, "{page} changed when the tree was flattened");
    }
}

#[test]
fn rotation_adds_to_what_a_page_inherited() {
    let mut pages = Pages::open(fixtures::organize_features());
    let ids = pages.ids();
    pages.document.rotate_pages(&[ids[0], ids[2]], 1).unwrap();
    assert_eq!(pages.rotations(), [90, 90, 180]);
    pages.document.rotate_pages(&[ids[2]], -3).unwrap();
    assert_eq!(pages.rotations(), [90, 90, 270]);
    pages.document.undo().unwrap();
    pages.document.undo().unwrap();
    assert_eq!(pages.rotations(), [0, 90, 90]);
}

#[test]
fn deleting_a_page_prunes_everything_that_pointed_at_it() {
    let mut pages = Pages::open(fixtures::organize_features());
    let ids = pages.ids();
    pages.document.delete_pages(&[ids[1]]).unwrap();
    assert_eq!(pages.ids(), [ids[0], ids[2]]);
    assert_eq!(pages.texts(), ["Page one", "Page three"]);
    let catalog = pages.catalog();

    let outlines = pages.resolved(catalog.get("Outlines").unwrap());
    let outlines = outlines.as_dict().unwrap();
    let part = pages.resolved(outlines.get("First").unwrap());
    let part = part.as_dict().unwrap();
    assert_eq!(part.get("Title").and_then(Object::as_string), Some(&b"Part"[..]), "the item to page two is gone");
    assert!(part.get("A").is_none(), "an item with children keeps them and loses its target");
    assert!(part.get("Prev").is_none() && outlines.get("Last") == outlines.get("First"));
    let section = pages.resolved(part.get("First").unwrap());
    assert_eq!(section.as_dict().unwrap().get("Title").and_then(Object::as_string), Some(&b"Section three"[..]));
    assert!(section.as_dict().unwrap().get("Prev").is_none() && part.get("First") == part.get("Last"));
    assert_eq!(part.get("Count").and_then(Object::as_int), Some(1));
    assert_eq!(outlines.get("Count").and_then(Object::as_int), Some(2));

    let dests = pages.resolved(catalog.get("Dests").unwrap());
    assert!(dests.as_dict().unwrap().get("two").is_none() && dests.as_dict().unwrap().get("three").is_some());
    let names = pages.resolved(catalog.get("Names").unwrap());
    let tree = pages.resolved(names.as_dict().unwrap().get("Dests").unwrap());
    let entries = tree.as_dict().unwrap().get("Names").and_then(Object::as_array).unwrap().to_vec();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].as_string(), Some(&b"p3"[..]));

    let first = pages.document.store().object(ids[0].0).unwrap();
    let links = first.as_dict().unwrap().get("Annots").and_then(Object::as_array).unwrap();
    assert!(links.is_empty(), "the link to page two is gone");
    let form = pages.resolved(catalog.get("AcroForm").unwrap());
    assert!(form.as_dict().unwrap().get("Fields").and_then(Object::as_array).unwrap().is_empty());

    let reopened = Pages::open(pages.document.save(false).unwrap());
    assert_eq!(reopened.texts(), ["Page one", "Page three"]);
    let optimized = pages.document.save(true).unwrap();
    assert!(!String::from_utf8_lossy(&optimized).contains("Page two"), "an optimized save drops the deleted page");
}

#[test]
fn the_last_page_cannot_be_deleted() {
    let mut pages = Pages::open(fixtures::organize_features());
    let error = pages.document.delete_pages(&pages.ids()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert_eq!(pages.ids().len(), 3);
}

#[test]
fn pages_move_in_front_of_another_or_to_the_end_in_document_order() {
    let mut pages = Pages::open(fixtures::organize_features());
    let ids = pages.ids();
    pages.document.move_pages(&[ids[2]], Some(ids[0])).unwrap();
    assert_eq!(pages.ids(), [ids[2], ids[0], ids[1]]);
    pages.document.move_pages(&[ids[1], ids[2]], None).unwrap();
    assert_eq!(pages.ids(), [ids[0], ids[2], ids[1]]);
    pages.document.move_pages(&[ids[0], ids[2]], Some(ids[2])).unwrap();
    assert_eq!(pages.ids(), [ids[0], ids[2], ids[1]], "a page moved in front of itself stays in the block");
}

#[test]
fn a_blank_page_goes_in_front_of_the_page_it_is_inserted_before() {
    let mut pages = Pages::open(fixtures::organize_features());
    let ids = pages.ids();
    let blank = pages.document.insert_blank_page(Some(ids[1]), (595.0, 842.0)).unwrap();
    assert_eq!(pages.ids(), [ids[0], blank, ids[1], ids[2]]);
    let summary = pages.document.view(&pages.library).summaries().unwrap();
    assert_eq!((summary[1].width, summary[1].height), (595.0, 842.0));
}

#[test]
fn a_merged_document_brings_its_pages_exactly_as_they_were() {
    for (bytes, password) in [(fixtures::editing_features(), ""), (encrypted("rc4-128"), "secret")] {
        let source = Document::open(bytes, password).unwrap();
        let library = library();
        let source_pages: Vec<_> = source.view(&library).pages().unwrap().iter().map(|page| page.id).collect();
        let pictures: Vec<_> =
            source_pages.iter().map(|page| source.view(&library).render(*page, 1.0, None).unwrap().rgba).collect();
        let mut pages = Pages::open(fixtures::organize_features());
        let ids = pages.ids();
        let merged = pages.document.insert_document(&source, Some(ids[2])).unwrap();
        assert_eq!(pages.ids().len(), 3 + source_pages.len());
        assert_eq!(pages.ids()[2..2 + merged.len()], merged[..]);
        for (page, picture) in merged.iter().zip(&pictures) {
            assert!(pages.render(*page) == *picture, "{page} does not look like its source");
        }
        let reopened = Pages::open(pages.document.save(false).unwrap());
        assert_eq!(reopened.ids().len(), 3 + source_pages.len());
    }
}

#[test]
fn extracted_pages_form_a_document_of_their_own_without_the_others() {
    let pages = Pages::open(fixtures::organize_features());
    let ids = pages.ids();
    let extracted = pages.document.extract_pages(&[ids[2], ids[0]]).unwrap();
    let reopened = Pages::open(extracted.clone());
    assert_eq!(reopened.texts(), ["Page three", "Page one"]);
    assert_eq!(reopened.rotations(), [90, 0]);
    let store = reopened.document.store();
    for id in store.ids() {
        if let Some(stream) = store.object(id).unwrap().as_stream() {
            let data = store.decode_stream(stream).unwrap().data;
            assert!(!String::from_utf8_lossy(&data).contains("Page two"), "content of a left page came along");
        }
    }
    assert!(reopened.catalog().get("Outlines").is_none());
}

#[test]
fn random_page_operations_follow_the_model_and_undo_completely() {
    for fixture in fixtures::all() {
        let Ok(mut document) = Document::open(fixture.bytes.clone(), "") else { continue };
        let library = library();
        let opened = document.save(false).unwrap();
        let state = |document: &Document| -> Vec<(PageId, u16)> {
            document.view(&library).pages().unwrap().iter().map(|page| (page.id, page.rotate)).collect()
        };
        let mut model = state(&document);
        if model.is_empty() {
            continue;
        }
        let mut random = Random::new(fixture.bytes.len() as u64);
        for step in 0..12 {
            let chosen: Vec<PageId> = model.iter().filter(|_| random.chance(40)).map(|(page, _)| *page).collect();
            let target = random.pick(&model).map(|(page, _)| *page);
            let context = format!("{} step {step}", fixture.name);
            match random.below(4) {
                0 => {
                    let turns = random.below(7) as i64 - 3;
                    document.rotate_pages(&chosen, turns).unwrap_or_else(|error| panic!("{context}: {error}"));
                    for (page, rotate) in &mut model {
                        if chosen.contains(page) {
                            *rotate = ((*rotate as i64 + 90 * turns).rem_euclid(360)) as u16;
                        }
                    }
                }
                1 if chosen.len() < model.len() => {
                    document.delete_pages(&chosen).unwrap_or_else(|error| panic!("{context}: {error}"));
                    model.retain(|(page, _)| !chosen.contains(page));
                }
                2 => {
                    document.move_pages(&chosen, target).unwrap_or_else(|error| panic!("{context}: {error}"));
                    let moved: Vec<_> = model.iter().filter(|(page, _)| chosen.contains(page)).copied().collect();
                    let anchor = target.and_then(|target| {
                        let from = model.iter().position(|(page, _)| *page == target)?;
                        model[from..].iter().find(|(page, _)| !chosen.contains(page)).map(|(page, _)| *page)
                    });
                    model.retain(|(page, _)| !chosen.contains(page));
                    let at = anchor
                        .and_then(|anchor| model.iter().position(|(page, _)| *page == anchor))
                        .unwrap_or(model.len());
                    model.splice(at..at, moved);
                }
                _ => {
                    let blank = document
                        .insert_blank_page(target, (200.0, 100.0))
                        .unwrap_or_else(|error| panic!("{context}: {error}"));
                    let at = target
                        .and_then(|target| model.iter().position(|(page, _)| *page == target))
                        .unwrap_or(model.len());
                    model.insert(at, (blank, 0));
                }
            }
            assert_eq!(state(&document), model, "{context}");
        }
        let last = state(&document);
        while document.undo().is_some() {}
        assert!(document.save(false).unwrap() == opened, "{}: undo left changes behind", fixture.name);
        while document.redo().is_some() {}
        assert_eq!(state(&document), last, "{}: redo did not return", fixture.name);
    }
}

#[test]
fn a_new_page_inherits_nothing_from_the_root_of_the_tree() {
    let fixture = fixtures::all().into_iter().find(|fixture| fixture.name == "nested-page-tree.pdf").unwrap();
    let mut pages = Pages::open(fixture.bytes);
    let before = pages.rotations();
    assert!(before.iter().any(|rotate| *rotate != 0), "the fixture's tree passes a rotation down");
    let blank = pages.document.insert_blank_page(None, (100.0, 100.0)).unwrap();
    assert_eq!(pages.ids().last(), Some(&blank));
    assert_eq!(pages.rotations()[..before.len()], before[..]);
    assert_eq!(pages.rotations().last(), Some(&0));
}
