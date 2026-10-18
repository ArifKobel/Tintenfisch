use crate::support;

use support::fixtures;
use support::rendering::library;
use tintenfisch::document::Document;
use tintenfisch::geometry::Rect;
use tintenfisch::navigation::{OutlineItem, Target};
use tintenfisch::pages::PageId;
use tintenfisch::store::ObjectStore;
use tintenfisch::syntax::{ObjId, Object};

struct Shelf {
    document: Document,
    pages: Vec<PageId>,
}

impl Shelf {
    fn open(bytes: Vec<u8>) -> Self {
        let document = Document::open(bytes, "").unwrap();
        let pages = document.view(&library()).pages().unwrap().iter().map(|page| page.id).collect();
        Self { document, pages }
    }

    fn shape(&self) -> Vec<String> {
        let mut lines = Vec::new();
        outline_lines(&self.document.outline().unwrap(), 0, &mut lines);
        lines
    }

    fn find(&self, title: &str) -> OutlineItem {
        fn search(items: &[OutlineItem], title: &str) -> Option<OutlineItem> {
            items
                .iter()
                .find_map(|item| (item.title == title).then(|| item.clone()).or_else(|| search(&item.children, title)))
        }
        search(&self.document.outline().unwrap(), title).unwrap_or_else(|| panic!("no bookmark {title}"))
    }

    fn reopened(&self) -> Shelf {
        let shelf = Shelf::open(self.document.save(false).unwrap());
        assert_linked(shelf.document.store());
        shelf
    }
}

fn outline_lines(items: &[OutlineItem], depth: usize, lines: &mut Vec<String>) {
    for item in items {
        lines.push(format!("{}{}", "  ".repeat(depth), item.title));
        outline_lines(&item.children, depth + 1, lines);
    }
}

fn assert_linked(store: &ObjectStore) {
    let reference = |dict: &tintenfisch::syntax::Dict, key: &str| dict.get(key).and_then(Object::as_reference);
    let catalog = store.resolve(store.trailer().get("Root").unwrap()).unwrap();
    let root = reference(catalog.as_dict().unwrap(), "Outlines").unwrap();
    fn walk(
        store: &ObjectStore,
        node: ObjId,
        reference: &dyn Fn(&tintenfisch::syntax::Dict, &str) -> Option<ObjId>,
    ) -> i64 {
        let dict = store.object(node).unwrap().as_dict().cloned().unwrap();
        let mut children = Vec::new();
        let mut next = reference(&dict, "First");
        while let Some(child) = next {
            children.push(child);
            next = reference(store.object(child).unwrap().as_dict().unwrap(), "Next");
        }
        assert_eq!(reference(&dict, "Last"), children.last().copied(), "the last child of {node:?}");
        let mut shown = 0;
        for (index, &child) in children.iter().enumerate() {
            let item = store.object(child).unwrap().as_dict().cloned().unwrap();
            assert_eq!(reference(&item, "Parent"), Some(node));
            assert_eq!(reference(&item, "Prev"), index.checked_sub(1).map(|previous| children[previous]));
            let below = walk(store, child, reference);
            let count = item.get("Count").and_then(Object::as_int);
            if below > 0 {
                assert_eq!(count.map(i64::abs), Some(below), "the count of {child:?}");
            }
            shown += 1 + if count.is_some_and(|count| count > 0) { below } else { 0 };
        }
        shown
    }
    let shown = walk(store, root, &reference);
    let counted = store.object(root).unwrap().as_dict().unwrap().get("Count").and_then(Object::as_int);
    assert_eq!(counted.unwrap_or(0), shown, "the root counts what shows");
}

#[test]
fn reads_bookmarks_through_every_kind_of_destination() {
    let shelf = Shelf::open(fixtures::navigation_features());
    assert_eq!(shelf.shape(), ["Chapter 1", "  Section 1.1", "  Section 1.2", "Chapter 2", "Website"]);
    let [first, second, third] = shelf.pages[..] else { panic!() };
    assert_eq!(shelf.find("Chapter 1").target, Some(Target::Page { page: first, top: Some(380.0) }));
    assert!(shelf.find("Chapter 1").open);
    assert_eq!(shelf.find("Section 1.1").target, Some(Target::Page { page: second, top: Some(250.0) }));
    assert_eq!(shelf.find("Section 1.2").target, Some(Target::Page { page: second, top: Some(120.0) }));
    assert_eq!(shelf.find("Chapter 2").target, Some(Target::Page { page: third, top: None }));
    assert_eq!(shelf.find("Website").target, Some(Target::Uri("https://example.org/".into())));
    assert_eq!(shelf.document.top_to_display(first, 380.0).unwrap(), 20.0);
    assert_eq!(shelf.document.top_to_user(first, 20.0).unwrap(), 380.0);
}

#[test]
fn added_renamed_and_retargeted_bookmarks_round_trip_and_undo() {
    let original = fixtures::navigation_features();
    let mut shelf = Shelf::open(original.clone());
    let second = shelf.pages[1];
    let target = Target::Page { page: second, top: Some(300.0) };
    let added = shelf.document.add_bookmark(None, 1, "Überblick", Some(&target)).unwrap();
    let chapter = shelf.find("Chapter 1").id;
    shelf.document.add_bookmark(Some(chapter), 2, "Section 1.3", None).unwrap();
    shelf.document.rename_bookmark(shelf.find("Section 1.1").id, "Introduction").unwrap();
    shelf.document.retarget_bookmark(shelf.find("Website").id, &Target::Uri("https://example.org/new".into())).unwrap();
    let expected =
        ["Chapter 1", "  Introduction", "  Section 1.2", "  Section 1.3", "Überblick", "Chapter 2", "Website"];
    assert_eq!(shelf.shape(), expected);
    let reopened = shelf.reopened();
    assert_eq!(reopened.shape(), expected);
    assert_eq!(reopened.find("Überblick").id, added);
    assert_eq!(reopened.find("Überblick").target, Some(target));
    assert_eq!(reopened.find("Website").target, Some(Target::Uri("https://example.org/new".into())));
    for _ in 0..4 {
        shelf.document.undo();
    }
    assert_eq!(shelf.document.save(false).unwrap(), original);
}

#[test]
fn bookmarks_move_with_what_is_under_them_and_never_under_themselves() {
    let mut shelf = Shelf::open(fixtures::navigation_features());
    let chapter = shelf.find("Chapter 1").id;
    shelf.document.move_bookmark(shelf.find("Section 1.2").id, None, 3).unwrap();
    shelf.document.move_bookmark(shelf.find("Chapter 2").id, Some(chapter), 0).unwrap();
    shelf.document.move_bookmark(shelf.find("Website").id, Some(shelf.find("Chapter 2").id), 0).unwrap();
    assert_eq!(shelf.shape(), ["Chapter 1", "  Chapter 2", "    Website", "  Section 1.1", "Section 1.2"]);
    assert!(shelf.document.move_bookmark(chapter, Some(shelf.find("Website").id), 0).is_err());
    assert!(shelf.document.move_bookmark(chapter, Some(chapter), 0).is_err());
    shelf.reopened();
    shelf.document.delete_bookmark(chapter).unwrap();
    assert_eq!(shelf.shape(), ["Section 1.2"]);
    let reopened = shelf.reopened();
    assert_eq!(reopened.shape(), ["Section 1.2"]);
    shelf.document.delete_bookmark(shelf.find("Section 1.2").id).unwrap();
    assert!(shelf.reopened().shape().is_empty());
}

#[test]
fn a_document_without_bookmarks_gets_an_outline_for_the_first_one() {
    let mut shelf = Shelf::open(fixtures::editing_features());
    assert!(shelf.shape().is_empty());
    let page = shelf.pages[0];
    shelf.document.add_bookmark(None, 0, "Start", Some(&Target::Page { page, top: None })).unwrap();
    let reopened = shelf.reopened();
    assert_eq!(reopened.shape(), ["Start"]);
    assert_eq!(reopened.find("Start").target, Some(Target::Page { page, top: None }));
}

#[test]
fn links_are_read_added_changed_and_removed_in_display_space() {
    let original = fixtures::navigation_features();
    let mut shelf = Shelf::open(original.clone());
    let [first, second, third] = shelf.pages[..] else { panic!() };
    let links = shelf.document.links(first).unwrap();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].rect, Rect::new(28.0, 96.0, 160.0, 114.0));
    assert_eq!(links[0].target, Some(Target::Page { page: third, top: Some(200.0) }));
    assert_eq!(links[1].target, Some(Target::Uri("https://example.org/visit".into())));
    let area = Rect::new(30.0, 40.0, 130.0, 60.0);
    let added = shelf.document.add_link(first, area, &Target::Page { page: second, top: Some(350.0) }).unwrap();
    let moved = Rect::new(40.0, 150.0, 200.0, 170.0);
    shelf.document.update_link(first, links[1].id, moved, &Target::Uri("https://example.org/changed".into())).unwrap();
    shelf.document.delete_annotation(first, links[0].id).unwrap();
    let reopened = shelf.reopened();
    let links = reopened.document.links(first).unwrap();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].rect, moved);
    assert_eq!(links[0].target, Some(Target::Uri("https://example.org/changed".into())));
    assert_eq!(links[1].id, added);
    assert_eq!(links[1].rect, area);
    assert_eq!(links[1].target, Some(Target::Page { page: second, top: Some(350.0) }));
    let turned = shelf.document.links(third).unwrap();
    assert!(turned.is_empty());
    shelf
        .document
        .add_link(third, Rect::new(10.0, 20.0, 110.0, 50.0), &Target::Uri("https://example.org/".into()))
        .unwrap();
    assert_eq!(shelf.document.links(third).unwrap()[0].rect, Rect::new(10.0, 20.0, 110.0, 50.0));
    for _ in 0..4 {
        shelf.document.undo();
    }
    assert_eq!(shelf.document.save(false).unwrap(), original);
}

#[test]
#[ignore = "writes edited bookmarks and links to PREVIEW_DIR for comparing with other viewers"]
fn write_navigation_preview() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let mut shelf = Shelf::open(fixtures::navigation_features());
    let [first, second, third] = shelf.pages[..] else { panic!() };
    shelf.document.add_bookmark(None, 1, "Added overview", Some(&Target::Page { page: third, top: None })).unwrap();
    shelf.document.move_bookmark(shelf.find("Website").id, Some(shelf.find("Chapter 1").id), 0).unwrap();
    shelf.document.rename_bookmark(shelf.find("Section 1.1").id, "Renamed section").unwrap();
    shelf
        .document
        .add_link(first, Rect::new(28.0, 36.0, 120.0, 56.0), &Target::Page { page: second, top: Some(350.0) })
        .unwrap();
    shelf
        .document
        .add_link(first, Rect::new(28.0, 66.0, 160.0, 86.0), &Target::Uri("https://example.org/added".into()))
        .unwrap();
    std::fs::write(format!("{directory}/navigation.pdf"), shelf.document.save(false).unwrap()).unwrap();
}
