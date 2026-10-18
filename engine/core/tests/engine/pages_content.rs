use crate::support;

use support::fixtures;
use tintenfisch::content::{ContentParser, ContentStream, ContentWriter, Splice};
use tintenfisch::file::SourceFile;
use tintenfisch::geometry::Rect;
use tintenfisch::pages::{Page, PageTree};
use tintenfisch::save::SaveMode;
use tintenfisch::store::ObjectStore;
use tintenfisch::syntax::{Name, ObjId, Object};

fn store_of(bytes: Vec<u8>) -> ObjectStore {
    ObjectStore::new(SourceFile::open(bytes).unwrap())
}

fn pages_of(store: &ObjectStore) -> Vec<Page> {
    PageTree::new(store).pages().unwrap()
}

#[test]
fn inherited_attributes_resolve_from_the_nearest_ancestor() {
    let store = store_of(fixtures::nested_page_tree());
    let pages = pages_of(&store);
    assert_eq!(pages.iter().map(|page| page.id.0.num).collect::<Vec<_>>(), vec![4, 5, 6]);

    let first = &pages[0];
    assert_eq!(first.media_box, Rect::new(0.0, 0.0, 500.0, 700.0));
    assert_eq!(first.crop_box, Some(Rect::new(10.0, 10.0, 490.0, 690.0)));
    assert_eq!(first.rotate, 270);
    assert_eq!(first.resources.get("ProcSet").and_then(Object::as_array).map(<[Object]>::len), Some(2));
    assert_eq!(first.display_size(), (680.0, 480.0));
    assert_eq!(first.parent, Some(ObjId::new(3, 0)));

    let second = &pages[1];
    assert_eq!(second.media_box, Rect::new(0.0, 0.0, 300.0, 300.0));
    assert_eq!(second.visible_box(), Rect::new(10.0, 10.0, 300.0, 300.0));
    assert_eq!(second.rotate, 180);
    assert!(second.resources.contains("Font") && !second.resources.contains("ProcSet"));
    assert_eq!(second.contents, vec![ObjId::new(7, 0), ObjId::new(8, 0)]);

    let third = &pages[2];
    assert_eq!((third.rotate, third.crop_box, third.display_size()), (90, None, (700.0, 500.0)));
    assert!(third.contents.is_empty());
}

#[test]
fn a_page_listed_twice_gets_its_own_copy_and_cycles_are_ignored() {
    let mut store = store_of(fixtures::shared_page_with_cycle());
    let before: Vec<ObjId> = pages_of(&store).iter().map(|page| page.id.0).collect();
    assert_eq!(before, vec![ObjId::new(3, 0), ObjId::new(3, 0)]);

    assert_eq!(PageTree::separate_shared_pages(&mut store).unwrap(), 1);
    let after: Vec<ObjId> = pages_of(&store).iter().map(|page| page.id.0).collect();
    assert_eq!(after.len(), 2);
    assert_ne!(after[0], after[1]);
    assert!(store.is_modified() && !store.can_undo());

    let reopened = store_of(SaveMode::Incremental.write(&store).unwrap());
    let saved: Vec<ObjId> = pages_of(&reopened).iter().map(|page| page.id.0).collect();
    assert_eq!(saved, after);
    assert_eq!(PageTree::separate_shared_pages(&mut store).unwrap(), 0);
}

#[test]
fn several_content_streams_form_one_sequence_of_operations() {
    let store = store_of(fixtures::nested_page_tree());
    let page = &pages_of(&store)[1];
    let content = ContentStream::load(&store, page).unwrap();
    assert_eq!(content.segments.len(), 2);
    let operators: Vec<(&str, usize)> =
        content.operations.iter().map(|operation| (operation.operator.as_str(), operation.segment)).collect();
    assert_eq!(operators[..2], [("q", 0), ("RG", 0)]);
    assert_eq!(operators[operators.len() - 2..], [("Tj", 1), ("ET", 1)]);
    let ordinals: Vec<usize> = content.operations.iter().map(|operation| operation.ordinal).collect();
    assert_eq!(ordinals, (0..content.operations.len()).collect::<Vec<_>>());
}

#[test]
fn every_span_holds_exactly_its_operation_and_an_empty_splice_changes_nothing() {
    for fixture in fixtures::all() {
        let store = store_of(fixture.bytes);
        for page in pages_of(&store) {
            let content = ContentStream::load(&store, &page).unwrap();
            for segment in &content.segments {
                assert_eq!(Splice::new().apply(&segment.data).unwrap(), segment.data, "{}", fixture.name);
            }
            for operation in &content.operations {
                let bytes = &content.segments[operation.segment].data[operation.span.clone()];
                let reparsed = ContentParser::new(bytes, operation.segment).parse(operation.ordinal);
                assert_eq!(reparsed.len(), 1, "{}: {:?}", fixture.name, String::from_utf8_lossy(bytes));
                assert_eq!(reparsed[0].operator, operation.operator, "{}", fixture.name);
                assert_eq!(reparsed[0].operands, operation.operands, "{}", fixture.name);
            }
        }
    }
}

#[test]
fn a_spliced_operation_replaces_only_its_own_bytes() {
    for fixture in fixtures::all() {
        let store = store_of(fixture.bytes);
        for page in pages_of(&store) {
            let content = ContentStream::load(&store, &page).unwrap();
            let Some(target) = content.operations.iter().find(|operation| operation.is("Tf")) else {
                continue;
            };
            let mut writer = ContentWriter::new();
            writer.operation("Tf", &[Object::Name(Name::from("Replaced")), Object::Int(99)]);
            let replacement = writer.finish();
            let segment = &content.segments[target.segment].data;
            let mut splice = Splice::new();
            splice.replace(target.span.clone(), replacement.clone());
            let spliced = splice.apply(segment).unwrap();
            assert_eq!(&spliced[..target.span.start], &segment[..target.span.start], "{}", fixture.name);
            let tail = target.span.start + replacement.len();
            assert_eq!(&spliced[tail..], &segment[target.span.end..], "{}", fixture.name);
            let reparsed = ContentParser::new(&spliced, 0).parse(0);
            let original_count =
                content.operations.iter().filter(|operation| operation.segment == target.segment).count();
            assert_eq!(reparsed.len(), original_count, "{}", fixture.name);
        }
    }
}
