use std::collections::BTreeMap;
use std::rc::Rc;

use crate::error::{Error, Result};
use crate::store::{ObjectStore, Slot};
use crate::syntax::{ObjId, Object};

pub(super) struct Recorded {
    pub before: BTreeMap<u32, Option<Slot>>,
    pub forces_rewrite: bool,
    pub ui_after: Option<String>,
}

pub struct Transaction<'s> {
    store: &'s mut ObjectStore,
    before: BTreeMap<u32, Option<Slot>>,
    forces_rewrite: bool,
    ui_after: Option<String>,
}

impl<'s> Transaction<'s> {
    pub(super) fn new(store: &'s mut ObjectStore) -> Self {
        Self { store, before: BTreeMap::new(), forces_rewrite: false, ui_after: None }
    }

    pub fn store(&self) -> &ObjectStore {
        self.store
    }

    pub fn object(&self, id: ObjId) -> Result<Rc<Object>> {
        self.store.object(id)
    }

    pub fn resolve(&self, object: &Object) -> Result<Object> {
        self.store.resolve(object)
    }

    pub fn put(&mut self, id: ObjId, object: Object) -> Result<()> {
        if !self.store.is_live(id) {
            return Err(Error::new(format!("cannot replace {id:?}: no such object")));
        }
        self.set(id.num, Slot::Object { generation: id.generation, object: Rc::new(object) });
        Ok(())
    }

    pub fn allocate(&mut self, object: Object) -> ObjId {
        let id = ObjId::new(self.store.next_num, 0);
        self.store.next_num += 1;
        self.set(id.num, Slot::Object { generation: 0, object: Rc::new(object) });
        id
    }

    pub fn free(&mut self, id: ObjId) -> Result<()> {
        if !self.store.is_live(id) {
            return Err(Error::new(format!("cannot free {id:?}: no such object")));
        }
        self.set(id.num, Slot::Freed { next_generation: id.generation.saturating_add(1) });
        Ok(())
    }

    pub fn force_rewrite(&mut self) {
        self.forces_rewrite = true;
    }

    pub fn set_ui_after(&mut self, ui_state: impl Into<String>) {
        self.ui_after = Some(ui_state.into());
    }

    fn set(&mut self, num: u32, slot: Slot) {
        let previous = self.store.overlay.get(&num).cloned();
        self.before.entry(num).or_insert(previous);
        if self.store.restates_source(num, &slot) {
            self.store.overlay.remove(&num);
        } else {
            self.store.overlay.insert(num, slot);
        }
    }

    pub(super) fn finish(self) -> Recorded {
        Recorded { before: self.before, forces_rewrite: self.forces_rewrite, ui_after: self.ui_after }
    }
}
