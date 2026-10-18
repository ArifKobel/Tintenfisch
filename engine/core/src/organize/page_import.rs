use std::collections::{HashMap, HashSet};

use crate::error::Result;
use crate::pages::{Page, PageTree};
use crate::store::{ObjectStore, Transaction};
use crate::syntax::{Dict, Name, ObjId, Object, Stream};

pub struct PageImporter<'s> {
    source: &'s ObjectStore,
    copies: HashMap<ObjId, ObjId>,
    tree_nodes: HashSet<ObjId>,
    left_behind: HashSet<ObjId>,
    pending: Vec<(ObjId, ObjId)>,
    root: ObjId,
}

impl<'s> PageImporter<'s> {
    pub fn new(source: &'s ObjectStore, root: ObjId) -> Self {
        Self {
            source,
            copies: HashMap::new(),
            tree_nodes: HashSet::new(),
            left_behind: HashSet::new(),
            pending: Vec::new(),
            root,
        }
    }

    pub fn import(&mut self, transaction: &mut Transaction, pages: &[Page]) -> Result<Vec<ObjId>> {
        let tree = PageTree::new(self.source);
        self.tree_nodes = tree.node_ids()?;
        self.left_behind = tree.pages()?.iter().map(|page| page.id.0).collect();
        let ids: Vec<ObjId> = pages.iter().map(|page| self.reserve(transaction, page.id.0)).collect();
        for (page, &id) in pages.iter().zip(&ids) {
            let original = self.source.object(page.id.0)?;
            let mut dict = original.as_dict().cloned().unwrap_or_default();
            dict.insert("Resources", Object::Dict(page.resources.clone()));
            dict.insert("MediaBox", page.media_box.to_object());
            match page.crop_box {
                Some(crop_box) => dict.insert("CropBox", crop_box.to_object()),
                None => {
                    dict.remove("CropBox");
                }
            }
            dict.insert("Rotate", Object::Int(page.rotate as i64));
            dict.remove("Parent");
            let mut copied = self.copy_dict(transaction, &dict);
            copied.insert("Parent", Object::Ref(self.root));
            transaction.put(id, Object::Dict(copied))?;
        }
        while let Some((source, target)) = self.pending.pop() {
            let object = self.source.object(source)?;
            let copied = self.copy(transaction, &object);
            transaction.put(target, copied)?;
        }
        Ok(ids)
    }

    fn reserve(&mut self, transaction: &mut Transaction, source: ObjId) -> ObjId {
        let target = transaction.allocate(Object::Null);
        self.copies.insert(source, target);
        target
    }

    fn copy(&mut self, transaction: &mut Transaction, object: &Object) -> Object {
        match object {
            Object::Ref(id) => self.reference(transaction, *id).map_or(Object::Null, Object::Ref),
            Object::Array(items) => Object::Array(items.iter().map(|item| self.copy(transaction, item)).collect()),
            Object::Dict(dict) => Object::Dict(self.copy_dict(transaction, dict)),
            Object::Stream(stream) => {
                Object::Stream(Stream { dict: self.copy_dict(transaction, &stream.dict), raw: stream.raw.clone() })
            }
            other => other.clone(),
        }
    }

    fn copy_dict(&mut self, transaction: &mut Transaction, dict: &Dict) -> Dict {
        dict.iter().map(|(key, value)| (Name::new(key.as_bytes()), self.copy(transaction, value))).collect()
    }

    fn reference(&mut self, transaction: &mut Transaction, source: ObjId) -> Option<ObjId> {
        if let Some(&copy) = self.copies.get(&source) {
            return Some(copy);
        }
        if self.tree_nodes.contains(&source) {
            return Some(self.root);
        }
        if self.left_behind.contains(&source) {
            return None;
        }
        let target = self.reserve(transaction, source);
        self.pending.push((source, target));
        Some(target)
    }
}
