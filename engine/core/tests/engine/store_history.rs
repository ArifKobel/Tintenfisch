use crate::support;

use std::rc::Rc;

use support::fixtures::{self, Fixture};
use support::graph::canonical_graph;
use support::random::Random;
use tintenfisch::Error;
use tintenfisch::file::{SourceFile, XrefEntry};
use tintenfisch::save::{FullRewriter, IncrementalWriter, SaveMode};
use tintenfisch::store::ObjectStore;
use tintenfisch::syntax::{Dict, Name, ObjId, Object, Stream};

fn store_of(bytes: Vec<u8>) -> ObjectStore {
    ObjectStore::new(SourceFile::open(bytes).unwrap())
}

fn editable(fixture: &Fixture) -> bool {
    !SourceFile::open(fixture.bytes.clone()).unwrap().is_repaired()
}

fn random_object(random: &mut Random, live: &[ObjId]) -> Object {
    match random.below(8) {
        0 => Object::Int(random.below(10_000) as i64 - 5000),
        1 => Object::Real(random.below(400) as f64 / 4.0 - 50.0),
        2 => Object::String((0..random.below(24)).map(|_| random.below(256) as u8).collect()),
        3 => Object::Name(Name::from(["Alpha", "Beta", "Gamma"][random.below(3)])),
        4 => Object::Array((0..random.below(4)).map(|_| Object::Ref(*random.pick(live).unwrap())).collect()),
        5 => {
            let dict: Dict = [
                (Name::from("Kind"), Object::Name(Name::from("Random"))),
                (Name::from("Link"), Object::Ref(*random.pick(live).unwrap())),
            ]
            .into_iter()
            .collect();
            Object::Dict(dict)
        }
        6 => {
            let data: Vec<u8> = (0..random.below(300)).map(|_| b"q Q BT ET 0 1 cm "[random.below(17)]).collect();
            Object::Stream(Stream { dict: Dict::new(), raw: Rc::from(data) })
        }
        _ => Object::Bool(random.chance(50)),
    }
}

fn mutate(store: &mut ObjectStore, random: &mut Random, steps: usize) {
    let root = store.trailer().get("Root").and_then(Object::as_reference).unwrap();
    for step in 0..steps {
        match random.below(10) {
            0 => {
                store.undo();
            }
            1 => {
                store.redo();
            }
            _ => {
                let live: Vec<ObjId> = store.ids().into_iter().filter(|id| *id != root).collect();
                let operations = 1 + random.below(3);
                let mut choices = Random::new(random.next());
                store
                    .transact(&format!("step {step}"), "", |transaction| {
                        for _ in 0..operations {
                            let target = choices.pick(&live).copied();
                            let reference = if live.is_empty() { vec![root] } else { live.clone() };
                            match (choices.below(3), target) {
                                (0, Some(id)) if transaction.store().is_live(id) => {
                                    transaction.put(id, random_object(&mut choices, &reference))?
                                }
                                (1, Some(id)) if transaction.store().is_live(id) => transaction.free(id)?,
                                _ => {
                                    transaction.allocate(random_object(&mut choices, &reference));
                                }
                            }
                        }
                        Ok(())
                    })
                    .unwrap();
            }
        }
    }
}

#[test]
fn undoing_every_change_saves_the_original_bytes() {
    for fixture in fixtures::all().into_iter().filter(editable) {
        for seed in 0..12 {
            let mut store = store_of(fixture.bytes.clone());
            let mut random = Random::new(seed);
            mutate(&mut store, &mut random, 25);
            while store.undo().is_some() {}
            assert!(!store.is_modified(), "{} seed {seed}", fixture.name);
            let saved = SaveMode::Incremental.write(&store).unwrap();
            assert!(saved == fixture.bytes, "{} seed {seed}: bytes differ after undoing everything", fixture.name);
        }
    }
}

#[test]
fn both_writers_reproduce_the_edited_document() {
    for fixture in fixtures::all() {
        for seed in 0..8 {
            let mut store = store_of(fixture.bytes.clone());
            mutate(&mut store, &mut Random::new(seed), 20);
            let expected = canonical_graph(&store);
            let full = SourceFile::open(FullRewriter::new(&store).write().unwrap()).unwrap();
            assert_eq!(canonical_graph(&full), expected, "{} seed {seed}: full rewrite", fixture.name);
            assert!(!full.is_repaired(), "{} seed {seed}: full rewrite needed repair", fixture.name);
            if editable(&fixture) {
                let incremental = SourceFile::open(IncrementalWriter::new(&store).write().unwrap()).unwrap();
                assert_eq!(canonical_graph(&incremental), expected, "{} seed {seed}: incremental", fixture.name);
                assert!(!incremental.is_repaired(), "{} seed {seed}: incremental needed repair", fixture.name);
            }
        }
    }
}

#[test]
fn an_incremental_update_keeps_the_original_bytes_as_its_prefix_and_its_xref_format() {
    for fixture in fixtures::all().into_iter().filter(editable) {
        let mut store = store_of(fixture.bytes.clone());
        store.transact("note", "", |transaction| Ok(transaction.allocate(Object::String(b"added".to_vec())))).unwrap();
        let saved = SaveMode::Incremental.write(&store).unwrap();
        assert!(saved.starts_with(&fixture.bytes), "{}", fixture.name);
        let reopened = SourceFile::open(saved).unwrap();
        assert_eq!(
            reopened.xref().latest().unwrap().format,
            store.source().xref().latest().unwrap().format,
            "{}",
            fixture.name
        );
    }
}

#[test]
fn a_freed_object_is_saved_as_free_with_the_next_generation() {
    let mut store = store_of(fixtures::incremental_with_freed_object());
    store.transact("free", "", |transaction| transaction.free(ObjId::new(6, 0))).unwrap();
    let reopened = SourceFile::open(SaveMode::Incremental.write(&store).unwrap()).unwrap();
    assert_eq!(reopened.xref().entry(6), Some(XrefEntry::Free { next_generation: 1 }));
    assert_eq!(*reopened.object(ObjId::new(6, 0)).unwrap(), Object::Null);
}

#[test]
fn the_document_id_keeps_its_permanent_part_and_renews_the_revision_part() {
    let mut store = store_of(fixtures::incremental_with_freed_object());
    store.transact("note", "", |transaction| Ok(transaction.allocate(Object::Int(1)))).unwrap();
    let first = SourceFile::open(SaveMode::Incremental.write(&store).unwrap()).unwrap();
    let first_id = first.trailer().get("ID").unwrap().as_array().unwrap().to_vec();
    assert_eq!(first_id.len(), 2);
    let mut again = ObjectStore::new(first);
    again.transact("note", "", |transaction| Ok(transaction.allocate(Object::Int(2)))).unwrap();
    let second = SourceFile::open(SaveMode::Incremental.write(&again).unwrap()).unwrap();
    let second_id = second.trailer().get("ID").unwrap().as_array().unwrap().to_vec();
    assert_eq!(second_id[0], first_id[0]);
    assert_ne!(second_id[1], first_id[1]);
    let rewritten = SourceFile::open(FullRewriter::new(&again).write().unwrap()).unwrap();
    assert_eq!(rewritten.trailer().get("ID").unwrap().as_array().unwrap()[0], first_id[0]);
}

#[test]
fn a_failing_transaction_leaves_no_trace() {
    let mut store = store_of(fixtures::incremental_with_freed_object());
    let next = store.next_num();
    let result: Result<(), Error> = store.transact("broken", "", |transaction| {
        transaction.allocate(Object::Int(1));
        transaction.put(ObjId::new(3, 0), Object::Null)?;
        Err(Error::new("refused"))
    });
    assert!(result.is_err());
    assert!(!store.is_modified());
    assert!(!store.can_undo());
    assert_eq!(store.next_num(), next);
}

#[test]
fn undo_and_redo_restore_the_ui_state_of_their_side() {
    let mut store = store_of(fixtures::incremental_with_freed_object());
    store
        .transact("rename", "caret 3", |transaction| {
            transaction.put(ObjId::new(6, 0), Object::String(b"changed".to_vec()))?;
            transaction.set_ui_after("caret 9");
            Ok(())
        })
        .unwrap();
    let undone = store.undo().unwrap();
    assert_eq!((undone.label.as_str(), undone.ui_state.as_str(), undone.touched), ("rename", "caret 3", vec![6]));
    assert_eq!(*store.object(ObjId::new(6, 0)).unwrap(), Object::String(b"second revision note".to_vec()));
    assert_eq!(store.redo().unwrap().ui_state, "caret 9");
    assert_eq!(*store.object(ObjId::new(6, 0)).unwrap(), Object::String(b"changed".to_vec()));
}

#[test]
fn a_new_change_discards_what_could_have_been_redone() {
    let mut store = store_of(fixtures::incremental_with_freed_object());
    store.transact("one", "", |transaction| Ok(transaction.allocate(Object::Int(1)))).unwrap();
    store.undo();
    assert!(store.can_redo());
    store.transact("two", "", |transaction| Ok(transaction.allocate(Object::Int(2)))).unwrap();
    assert!(!store.can_redo());
}

#[test]
fn a_transaction_that_changes_nothing_is_not_recorded() {
    let mut store = store_of(fixtures::incremental_with_freed_object());
    let revision = store.revision();
    store
        .transact("noop", "", |transaction| {
            let original = (*transaction.object(ObjId::new(6, 0))?).clone();
            transaction.put(ObjId::new(6, 0), Object::Int(0))?;
            transaction.put(ObjId::new(6, 0), original)
        })
        .unwrap();
    assert!(!store.can_undo());
    assert_eq!(store.revision(), revision);
}

#[test]
fn the_save_mode_follows_the_history_and_the_source() {
    let mut store = store_of(fixtures::incremental_with_freed_object());
    assert_eq!(SaveMode::required_for(&store, false), SaveMode::Incremental);
    assert_eq!(SaveMode::required_for(&store, true), SaveMode::FullRewrite);
    store
        .transact("redact", "", |transaction| {
            transaction.force_rewrite();
            Ok(())
        })
        .unwrap();
    assert_eq!(SaveMode::required_for(&store, false), SaveMode::FullRewrite);
    store.undo();
    assert_eq!(SaveMode::required_for(&store, false), SaveMode::Incremental);
    let repaired = store_of(fixtures::without_xref());
    assert_eq!(SaveMode::required_for(&repaired, false), SaveMode::FullRewrite);
    assert!(
        IncrementalWriter::new(&{
            let mut store = repaired;
            store.transact("x", "", |transaction| Ok(transaction.allocate(Object::Int(1)))).unwrap();
            store
        })
        .write()
        .is_err()
    );
}

#[test]
fn objects_cannot_be_replaced_or_freed_when_they_do_not_exist() {
    let mut store = store_of(fixtures::incremental_with_freed_object());
    assert!(store.transact("x", "", |transaction| transaction.put(ObjId::new(5, 0), Object::Int(1))).is_err());
    assert!(store.transact("x", "", |transaction| transaction.free(ObjId::new(99, 0))).is_err());
    assert!(store.transact("x", "", |transaction| transaction.put(ObjId::new(3, 1), Object::Int(1))).is_err());
}

#[test]
fn a_trial_shows_the_change_and_then_leaves_no_trace() {
    let mut store = store_of(fixtures::incremental_with_freed_object());
    let revision = store.revision();
    let seen = store
        .trial(
            |transaction| transaction.put(ObjId::new(6, 0), Object::String(b"tried".to_vec())),
            |trying| ((*trying.object(ObjId::new(6, 0)).unwrap()).clone(), trying.revision()),
        )
        .unwrap();
    assert_eq!(seen.0, Object::String(b"tried".to_vec()));
    assert_ne!(seen.1, revision);
    assert_eq!(store.revision(), revision);
    assert!(!store.is_modified() && !store.can_undo());
    store.transact("real", "", |transaction| transaction.put(ObjId::new(6, 0), Object::Int(1))).unwrap();
    assert_ne!(store.revision(), seen.1, "a real revision never repeats a trial's");
}
