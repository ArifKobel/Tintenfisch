use crate::support;

use support::fixtures::{self, CONTENT};
use tintenfisch::file::{SectionFormat, SourceFile};
use tintenfisch::syntax::{ObjId, Object};

fn open(bytes: Vec<u8>) -> SourceFile {
    SourceFile::open(bytes).unwrap()
}

fn page(file: &SourceFile) -> Object {
    (*file.object(ObjId::new(3, 0)).unwrap()).clone()
}

#[test]
fn every_object_of_every_fixture_resolves_and_decodes() {
    for fixture in fixtures::all() {
        let file = SourceFile::open(fixture.bytes).unwrap_or_else(|error| panic!("{}: {error}", fixture.name));
        let catalog = file.catalog().unwrap_or_else(|error| panic!("{}: {error}", fixture.name));
        let pages = file.resolve(catalog.as_dict().unwrap().get("Pages").unwrap()).unwrap();
        assert!(pages.as_dict().is_some_and(|pages| pages.has_type("Pages")), "{}: no page tree", fixture.name);
        let ids: Vec<ObjId> = file.ids().collect();
        assert!(!ids.is_empty(), "{}", fixture.name);
        for id in ids {
            let object = file.object(id).unwrap_or_else(|error| panic!("{} {id:?}: {error}", fixture.name));
            if let Some(stream) = object.as_stream() {
                file.decode_stream(stream).unwrap_or_else(|error| panic!("{} {id:?}: {error}", fixture.name));
            }
        }
    }
}

#[test]
fn a_newer_revision_overrides_and_frees_objects() {
    let file = open(fixtures::incremental_with_freed_object());
    let page = page(&file);
    let media_box = page.as_dict().unwrap().get("MediaBox").unwrap().as_array().unwrap().to_vec();
    assert_eq!(media_box[2], Object::Int(300));
    assert_eq!(*file.object(ObjId::new(5, 0)).unwrap(), Object::Null);
    assert_eq!(*file.object(ObjId::new(6, 0)).unwrap(), Object::String(b"second revision note".to_vec()));
    assert_eq!(file.xref().latest().unwrap().format, SectionFormat::Table);
    assert!(!file.ids().any(|id| id.num == 5));
}

#[test]
fn objects_inside_object_streams_resolve_through_a_predicted_xref_stream() {
    let file = open(fixtures::xref_stream_with_object_streams());
    assert!(page(&file).as_dict().unwrap().has_type("Page"));
    let content = file.object(ObjId::new(4, 0)).unwrap();
    assert_eq!(file.decode_stream(content.as_stream().unwrap()).unwrap().data, CONTENT);
    assert_eq!(file.xref().latest().unwrap().format, SectionFormat::Stream);
    assert!(!file.trailer().contains("W"));
    assert!(!file.is_repaired());
}

#[test]
fn a_hybrid_file_finds_compressed_objects_the_table_marks_free() {
    let file = open(fixtures::hybrid());
    assert!(page(&file).as_dict().unwrap().has_type("Page"));
    assert!(!file.is_repaired());
}

#[test]
fn a_stream_update_chains_onto_a_table_revision() {
    let file = open(fixtures::table_then_stream_update());
    let catalog = file.catalog().unwrap();
    let note = file.resolve(catalog.as_dict().unwrap().get("Note").unwrap()).unwrap();
    assert_eq!(note, Object::String(b"compressed in the update".to_vec()));
    assert!(page(&file).as_dict().unwrap().has_type("Page"));
}

#[test]
fn a_single_wrong_offset_is_recovered_without_repairing_the_whole_file() {
    let file = open(fixtures::one_wrong_offset());
    assert!(!file.is_repaired());
    assert!(page(&file).as_dict().unwrap().has_type("Page"));
}

#[test]
fn a_file_without_cross_reference_data_is_repaired() {
    let file = open(fixtures::without_xref());
    assert!(file.is_repaired());
    assert_eq!(file.ids().count(), 4);
    assert!(page(&file).as_dict().unwrap().has_type("Page"));
}

#[test]
fn an_indirect_stream_length_is_honoured() {
    let file = open(fixtures::indirect_length());
    let content = file.object(ObjId::new(4, 0)).unwrap();
    assert_eq!(&*content.as_stream().unwrap().raw, CONTENT);
}

#[test]
fn a_reference_of_the_wrong_generation_resolves_to_null() {
    let file = open(fixtures::incremental_with_freed_object());
    assert_eq!(*file.object(ObjId::new(1, 3)).unwrap(), Object::Null);
    assert_eq!(*file.object(ObjId::new(99, 0)).unwrap(), Object::Null);
}

#[test]
fn files_that_are_not_pdf_are_refused() {
    assert!(SourceFile::open(b"plain text".to_vec()).is_err());
    assert!(SourceFile::open(b"%PDF-1.4\nnothing here".to_vec()).is_err());
}
