use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use crate::error::Result;
use crate::file::SourceFile;
use crate::file::filters::{Decoded, FilterChain};
use crate::store::transaction::Recorded;
use crate::store::{Change, History, Slot, SlotChange, Transaction};
use crate::syntax::{Dict, ObjId, Object, Stream};

#[derive(Clone, Debug, PartialEq)]
pub struct Reverted {
    pub label: String,
    pub touched: Vec<u32>,
    pub ui_state: String,
}

pub struct ObjectStore {
    source: SourceFile,
    pub(super) overlay: HashMap<u32, Slot>,
    pub(super) next_num: u32,
    history: History,
    revision: u64,
    revisions_issued: u64,
}

impl ObjectStore {
    pub fn new(source: SourceFile) -> Self {
        let next_num = source.xref().size().max(1);
        Self {
            source,
            overlay: HashMap::new(),
            next_num,
            history: History::default(),
            revision: 0,
            revisions_issued: 0,
        }
    }

    pub fn source(&self) -> &SourceFile {
        &self.source
    }

    pub fn trailer(&self) -> &Dict {
        self.source.trailer()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn next_num(&self) -> u32 {
        self.next_num
    }

    pub fn is_modified(&self) -> bool {
        !self.overlay.is_empty()
    }

    pub fn object(&self, id: ObjId) -> Result<Rc<Object>> {
        match self.overlay.get(&id.num) {
            Some(Slot::Object { generation, object }) if *generation == id.generation => Ok(object.clone()),
            Some(_) => Ok(Rc::new(Object::Null)),
            None => self.source.object(id),
        }
    }

    pub fn resolve(&self, object: &Object) -> Result<Object> {
        match object {
            Object::Ref(id) => Ok((*self.object(*id)?).clone()),
            other => Ok(other.clone()),
        }
    }

    pub fn decode_stream(&self, stream: &Stream) -> Result<Decoded> {
        let chain = FilterChain::of(&stream.dict, |object| self.resolve(object).unwrap_or(Object::Null))?;
        chain.decode(&stream.raw)
    }

    pub fn is_live(&self, id: ObjId) -> bool {
        match self.overlay.get(&id.num) {
            Some(Slot::Object { generation, .. }) => *generation == id.generation,
            Some(Slot::Freed { .. }) => false,
            None => self
                .source
                .xref()
                .entry(id.num)
                .is_some_and(|entry| !entry.is_free() && entry.generation() == id.generation),
        }
    }

    pub(super) fn restates_source(&self, num: u32, slot: &Slot) -> bool {
        let Slot::Object { generation, object } = slot else {
            return false;
        };
        let id = ObjId::new(num, *generation);
        let in_source =
            self.source.xref().entry(num).is_some_and(|entry| !entry.is_free() && entry.generation() == *generation);
        in_source && self.source.object(id).is_ok_and(|original| *original == **object)
    }

    pub fn ids(&self) -> Vec<ObjId> {
        let mut ids: BTreeSet<ObjId> = self.source.ids().filter(|id| !self.overlay.contains_key(&id.num)).collect();
        ids.extend(self.overlay.iter().filter_map(|(&num, slot)| match slot {
            Slot::Object { generation, .. } => Some(ObjId::new(num, *generation)),
            Slot::Freed { .. } => None,
        }));
        ids.into_iter().collect()
    }

    pub fn changes(&self) -> Vec<(u32, &Slot)> {
        let mut changes: Vec<(u32, &Slot)> = self.overlay.iter().map(|(&num, slot)| (num, slot)).collect();
        changes.sort_by_key(|(num, _)| *num);
        changes
    }

    pub fn forces_rewrite(&self) -> bool {
        self.history.forces_rewrite()
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn transact<T>(
        &mut self,
        label: &str,
        ui_state: &str,
        body: impl FnOnce(&mut Transaction) -> Result<T>,
    ) -> Result<T> {
        let next_num_before = self.next_num;
        let (value, recorded) = self.run(body)?;
        let slots: Vec<SlotChange> = recorded
            .before
            .into_iter()
            .map(|(num, before)| SlotChange { num, after: self.overlay.get(&num).cloned(), before })
            .filter(|slot| slot.before != slot.after)
            .collect();
        if !slots.is_empty() || recorded.forces_rewrite {
            self.history.record(Change {
                label: label.to_string(),
                slots,
                next_num_before,
                next_num_after: self.next_num,
                forces_rewrite: recorded.forces_rewrite,
                ui_before: ui_state.to_string(),
                ui_after: recorded.ui_after.unwrap_or_else(|| ui_state.to_string()),
            });
            self.advance_revision();
        }
        Ok(value)
    }

    pub fn repair<T>(&mut self, body: impl FnOnce(&mut Transaction) -> Result<T>) -> Result<T> {
        Ok(self.run(body)?.0)
    }

    fn run<T>(&mut self, body: impl FnOnce(&mut Transaction) -> Result<T>) -> Result<(T, Recorded)> {
        let next_num_before = self.next_num;
        let mut transaction = Transaction::new(self);
        let outcome = body(&mut transaction);
        let recorded = transaction.finish();
        match outcome {
            Ok(value) => Ok((value, recorded)),
            Err(error) => {
                for (num, slot) in recorded.before {
                    self.restore(num, slot);
                }
                self.next_num = next_num_before;
                Err(error)
            }
        }
    }

    pub fn undo(&mut self) -> Option<Reverted> {
        let change = self.history.take_undo()?;
        for slot in &change.slots {
            self.restore(slot.num, slot.before.clone());
        }
        self.next_num = change.next_num_before;
        self.advance_revision();
        let reverted =
            Reverted { label: change.label.clone(), touched: change.touched(), ui_state: change.ui_before.clone() };
        self.history.push_undone(change);
        Some(reverted)
    }

    pub fn redo(&mut self) -> Option<Reverted> {
        let change = self.history.take_redo()?;
        for slot in &change.slots {
            self.restore(slot.num, slot.after.clone());
        }
        self.next_num = change.next_num_after;
        self.advance_revision();
        let reverted =
            Reverted { label: change.label.clone(), touched: change.touched(), ui_state: change.ui_after.clone() };
        self.history.push_redone(change);
        Some(reverted)
    }

    fn advance_revision(&mut self) {
        self.revisions_issued += 1;
        self.revision = self.revisions_issued;
    }

    pub fn trial<T>(
        &mut self,
        body: impl FnOnce(&mut Transaction) -> Result<()>,
        inspect: impl FnOnce(&ObjectStore) -> T,
    ) -> Result<T> {
        let (next_num, revision) = (self.next_num, self.revision);
        let ((), recorded) = self.run(body)?;
        self.advance_revision();
        let seen = inspect(self);
        for (num, slot) in recorded.before {
            self.restore(num, slot);
        }
        self.next_num = next_num;
        self.revision = revision;
        Ok(seen)
    }

    fn restore(&mut self, num: u32, slot: Option<Slot>) {
        match slot {
            Some(slot) => self.overlay.insert(num, slot),
            None => self.overlay.remove(&num),
        };
    }
}
