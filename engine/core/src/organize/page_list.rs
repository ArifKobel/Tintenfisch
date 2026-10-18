use std::collections::HashSet;

use crate::error::{Error, Result};
use crate::pages::{Page, PageTree};
use crate::store::Transaction;
use crate::syntax::{Dict, Name, ObjId, Object};

const INHERITABLE: [&str; 4] = ["Resources", "MediaBox", "CropBox", "Rotate"];

pub struct PageList {
    root: ObjId,
    pages: Vec<ObjId>,
}

impl PageList {
    pub fn flatten(transaction: &mut Transaction) -> Result<Self> {
        let pages = PageTree::new(transaction.store()).pages()?;
        let root = root_of(transaction)?;
        let flat = pages.iter().all(|page| page.parent == Some(root));
        for page in &pages {
            Self::own_attributes(transaction, page, root)?;
        }
        if !flat {
            let nodes = PageTree::new(transaction.store()).node_ids()?;
            for node in nodes.into_iter().filter(|node| *node != root) {
                transaction.free(node)?;
            }
        }
        let list = Self { root, pages: pages.iter().map(|page| page.id.0).collect() };
        let root_inherits = transaction
            .object(root)?
            .as_dict()
            .is_some_and(|root| INHERITABLE.iter().any(|key| root.get(key).is_some()));
        if !flat || root_inherits {
            list.write(transaction)?;
        }
        Ok(list)
    }

    pub fn root(&self) -> ObjId {
        self.root
    }

    pub fn pages(&self) -> &[ObjId] {
        &self.pages
    }

    pub fn position_before(&self, before: Option<ObjId>) -> usize {
        before.and_then(|before| self.pages.iter().position(|page| *page == before)).unwrap_or(self.pages.len())
    }

    pub fn remove(&mut self, pages: &HashSet<ObjId>) {
        self.pages.retain(|page| !pages.contains(page));
    }

    pub fn insert(&mut self, at: usize, pages: &[ObjId]) {
        let at = at.min(self.pages.len());
        self.pages.splice(at..at, pages.iter().copied());
    }

    pub fn write(&self, transaction: &mut Transaction) -> Result<()> {
        let mut root = transaction
            .object(self.root)?
            .as_dict()
            .cloned()
            .ok_or_else(|| Error::new("the page tree root is not a dictionary"))?;
        for key in INHERITABLE {
            root.remove(key);
        }
        root.insert("Kids", Object::Array(self.pages.iter().map(|page| Object::Ref(*page)).collect()));
        root.insert("Count", Object::Int(self.pages.len() as i64));
        transaction.put(self.root, Object::Dict(root))
    }

    fn own_attributes(transaction: &mut Transaction, page: &Page, root: ObjId) -> Result<()> {
        let original = transaction.object(page.id.0)?;
        let mut dict = original.as_dict().cloned().ok_or_else(|| Error::new("a page is not a dictionary"))?;
        let before = dict.clone();
        dict.insert_missing(Name::from("Resources"), Object::Dict(page.resources.clone()));
        dict.insert_missing(Name::from("MediaBox"), page.media_box.to_object());
        if let Some(crop_box) = page.crop_box {
            dict.insert_missing(Name::from("CropBox"), crop_box.to_object());
        }
        if page.rotate != 0 {
            dict.insert_missing(Name::from("Rotate"), Object::Int(page.rotate as i64));
        }
        dict.insert("Parent", Object::Ref(root));
        if dict != before {
            transaction.put(page.id.0, Object::Dict(dict))?;
        }
        Ok(())
    }
}

pub fn root_of(transaction: &Transaction) -> Result<ObjId> {
    catalog_of(transaction)?
        .get("Pages")
        .and_then(Object::as_reference)
        .ok_or_else(|| Error::new("the catalog has no page tree"))
}

pub fn catalog_of(transaction: &Transaction) -> Result<Dict> {
    let catalog = transaction
        .store()
        .trailer()
        .get("Root")
        .and_then(Object::as_reference)
        .ok_or_else(|| Error::new("trailer has no /Root"))?;
    transaction.object(catalog)?.as_dict().cloned().ok_or_else(|| Error::new("the catalog is not a dictionary"))
}
