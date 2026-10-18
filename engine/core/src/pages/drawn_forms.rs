use std::collections::BTreeSet;

use crate::store::ObjectStore;
use crate::syntax::{Dict, ObjId, Object};

const MAX_DEPTH: usize = 12;

pub fn drawn_forms(store: &ObjectStore, resources: &Dict) -> BTreeSet<ObjId> {
    let mut found = BTreeSet::new();
    collect(store, resources, &mut found, 0);
    found
}

fn collect(store: &ObjectStore, resources: &Dict, found: &mut BTreeSet<ObjId>, depth: usize) {
    if depth >= MAX_DEPTH {
        return;
    }
    let Some(xobjects) = resources.get("XObject").and_then(|entry| store.resolve(entry).ok()) else { return };
    let Some(xobjects) = xobjects.as_dict() else { return };
    for (_, entry) in xobjects.iter() {
        let Some(id) = entry.as_reference() else { continue };
        if found.contains(&id) {
            continue;
        }
        let Ok(object) = store.object(id) else { continue };
        let Some(stream) = object.as_stream() else { continue };
        if !stream.dict.name("Subtype").is_some_and(|subtype| subtype.is("Form")) {
            continue;
        }
        found.insert(id);
        if let Some(own) =
            stream.dict.get("Resources").and_then(|own| store.resolve(own).ok()).as_ref().and_then(Object::as_dict)
        {
            collect(store, own, found, depth + 1);
        }
    }
}
