use std::collections::{HashMap, HashSet};

use crate::error::{Error, Result};
use crate::navigation::Target;
use crate::navigation::target::catalog;
use crate::organize::catalog_of;
use crate::store::{ObjectStore, Transaction};
use crate::syntax::{Dict, Name, ObjId, Object, decode_text, encode_text};

const MAX_DEPTH: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub struct OutlineItem {
    pub id: ObjId,
    pub title: String,
    pub target: Option<Target>,
    pub open: bool,
    pub children: Vec<OutlineItem>,
}

pub struct OutlineTree {
    root: Option<ObjId>,
    items: HashMap<ObjId, Dict>,
    children: HashMap<Option<ObjId>, Vec<ObjId>>,
}

impl OutlineTree {
    pub fn load(store: &ObjectStore) -> Result<OutlineTree> {
        let mut tree = OutlineTree { root: None, items: HashMap::new(), children: HashMap::new() };
        let root = catalog(store)?.and_then(|catalog| catalog.get("Outlines").and_then(Object::as_reference));
        if let Some(root) = root {
            tree.root = Some(root);
            let mut seen = HashSet::from([root]);
            tree.read_children(store, None, root, 0, &mut seen)?;
        }
        Ok(tree)
    }

    pub fn items(&self, store: &ObjectStore) -> Result<Vec<OutlineItem>> {
        self.items_under(store, None)
    }

    pub fn insert(&mut self, parent: Option<ObjId>, index: usize, id: ObjId, dict: Dict) -> Result<()> {
        self.check_parent(parent)?;
        let siblings = self.children.entry(parent).or_default();
        siblings.insert(index.min(siblings.len()), id);
        self.items.insert(id, dict);
        Ok(())
    }

    pub fn rename(&mut self, id: ObjId, title: &str) -> Result<()> {
        self.item(id)?.insert("Title", Object::String(encode_text(title)));
        Ok(())
    }

    pub fn retarget(&mut self, id: ObjId, target: &Target) -> Result<()> {
        target.write(self.item(id)?);
        Ok(())
    }

    pub fn move_to(&mut self, id: ObjId, parent: Option<ObjId>, index: usize) -> Result<()> {
        self.check_parent(parent)?;
        if parent.is_some_and(|parent| parent == id || self.is_under(parent, id)) {
            return Err(Error::new("a bookmark cannot move under itself"));
        }
        self.detach(id)?;
        let siblings = self.children.entry(parent).or_default();
        siblings.insert(index.min(siblings.len()), id);
        Ok(())
    }

    pub fn remove(&mut self, id: ObjId) -> Result<()> {
        self.detach(id)
    }

    pub fn write(&self, transaction: &mut Transaction) -> Result<()> {
        let root = match self.root {
            Some(root) => root,
            None => {
                let outlines: Dict = [(Name::from("Type"), Object::Name(Name::from("Outlines")))].into_iter().collect();
                let root = transaction.allocate(Object::Dict(outlines));
                let catalog_id = transaction
                    .store()
                    .trailer()
                    .get("Root")
                    .and_then(Object::as_reference)
                    .ok_or_else(|| Error::new("trailer has no /Root"))?;
                let mut catalog = catalog_of(transaction)?;
                catalog.insert("Outlines", Object::Ref(root));
                transaction.put(catalog_id, Object::Dict(catalog))?;
                root
            }
        };
        let shown = self.link(transaction, None, root)?;
        let mut node = transaction.object(root)?.as_dict().cloned().unwrap_or_default();
        let before = node.clone();
        let top = self.children.get(&None).map(Vec::as_slice).unwrap_or_default();
        link_to(&mut node, "First", top.first().copied());
        link_to(&mut node, "Last", top.last().copied());
        node.insert("Count", Object::Int(shown));
        if node != before {
            transaction.put(root, Object::Dict(node))?;
        }
        Ok(())
    }

    fn link(&self, transaction: &mut Transaction, parent: Option<ObjId>, node: ObjId) -> Result<i64> {
        let siblings = self.children.get(&parent).map(Vec::as_slice).unwrap_or_default();
        let mut shown = 0;
        for (index, &id) in siblings.iter().enumerate() {
            let below = self.link(transaction, Some(id), id)?;
            let mut item = self.items[&id].clone();
            let own = self.children.get(&Some(id)).map(Vec::as_slice).unwrap_or_default();
            item.insert("Parent", Object::Ref(node));
            link_to(&mut item, "Prev", index.checked_sub(1).map(|previous| siblings[previous]));
            link_to(&mut item, "Next", siblings.get(index + 1).copied());
            link_to(&mut item, "First", own.first().copied());
            link_to(&mut item, "Last", own.last().copied());
            let open = item.get("Count").and_then(Object::as_int).is_some_and(|count| count > 0);
            if own.is_empty() {
                item.remove("Count");
            } else {
                item.insert("Count", Object::Int(if open { below } else { -below }));
            }
            shown += 1 + if open { below } else { 0 };
            let stored = transaction.object(id)?;
            if stored.as_dict() != Some(&item) {
                transaction.put(id, Object::Dict(item))?;
            }
        }
        Ok(shown)
    }

    fn read_children(
        &mut self,
        store: &ObjectStore,
        parent: Option<ObjId>,
        node: ObjId,
        depth: usize,
        seen: &mut HashSet<ObjId>,
    ) -> Result<()> {
        let dict = store.object(node)?.as_dict().cloned().unwrap_or_default();
        let mut next = dict.get("First").and_then(Object::as_reference);
        let mut ids = Vec::new();
        while let Some(id) = next.filter(|id| depth < MAX_DEPTH && seen.insert(*id)) {
            let item = store.object(id)?.as_dict().cloned().unwrap_or_default();
            next = item.get("Next").and_then(Object::as_reference);
            self.items.insert(id, item);
            ids.push(id);
        }
        for &id in &ids {
            self.read_children(store, Some(id), id, depth + 1, seen)?;
        }
        self.children.insert(parent, ids);
        Ok(())
    }

    fn items_under(&self, store: &ObjectStore, parent: Option<ObjId>) -> Result<Vec<OutlineItem>> {
        let siblings = self.children.get(&parent).map(Vec::as_slice).unwrap_or_default();
        siblings
            .iter()
            .map(|&id| {
                let dict = &self.items[&id];
                Ok(OutlineItem {
                    id,
                    title: dict.get("Title").and_then(Object::as_string).map(decode_text).unwrap_or_default(),
                    target: Target::of(store, dict)?,
                    open: dict.get("Count").and_then(Object::as_int).is_some_and(|count| count > 0),
                    children: self.items_under(store, Some(id))?,
                })
            })
            .collect()
    }

    fn item(&mut self, id: ObjId) -> Result<&mut Dict> {
        self.items.get_mut(&id).ok_or_else(|| Error::new(format!("no bookmark {}", id.num)))
    }

    fn check_parent(&self, parent: Option<ObjId>) -> Result<()> {
        match parent {
            Some(parent) if !self.items.contains_key(&parent) => Err(Error::new(format!("no bookmark {}", parent.num))),
            _ => Ok(()),
        }
    }

    fn detach(&mut self, id: ObjId) -> Result<()> {
        for siblings in self.children.values_mut() {
            if let Some(position) = siblings.iter().position(|sibling| *sibling == id) {
                siblings.remove(position);
                return Ok(());
            }
        }
        Err(Error::new(format!("no bookmark {}", id.num)))
    }

    fn is_under(&self, id: ObjId, ancestor: ObjId) -> bool {
        let mut pending = self.children.get(&Some(ancestor)).cloned().unwrap_or_default();
        while let Some(child) = pending.pop() {
            if child == id {
                return true;
            }
            pending.extend(self.children.get(&Some(child)).cloned().unwrap_or_default());
        }
        false
    }
}

fn link_to(dict: &mut Dict, key: &str, target: Option<ObjId>) {
    match target {
        Some(id) => dict.insert(key, Object::Ref(id)),
        None => {
            dict.remove(key);
        }
    }
}
