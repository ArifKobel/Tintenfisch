use std::collections::HashSet;

use crate::document::Document;
use crate::error::{Error, Result};
use crate::geometry::Rect;
use crate::organize::{DeadLinks, PageImporter, PageList, blank_document};
use crate::pages::{PageId, PageTree, normalized_rotation};
use crate::syntax::{Dict, Object};

impl Document {
    pub fn rotate_pages(&mut self, pages: &[PageId], quarter_turns: i64) -> Result<()> {
        self.store.transact("Rotate pages", "", |transaction| {
            for page in PageTree::new(transaction.store()).pages()?.into_iter().filter(|page| pages.contains(&page.id))
            {
                let mut dict = transaction.object(page.id.0)?.as_dict().cloned().unwrap_or_default();
                let rotate = normalized_rotation(page.rotate as i64 + 90 * quarter_turns);
                dict.insert("Rotate", Object::Int(rotate as i64));
                transaction.put(page.id.0, Object::Dict(dict))?;
            }
            Ok(())
        })
    }

    pub fn delete_pages(&mut self, pages: &[PageId]) -> Result<()> {
        self.store.transact("Delete pages", "", |transaction| {
            let mut list = PageList::flatten(transaction)?;
            let gone: HashSet<_> = pages.iter().map(|page| page.0).filter(|page| list.pages().contains(page)).collect();
            if gone.len() == list.pages().len() {
                return Err(Error::unsupported("a document keeps at least one page"));
            }
            list.remove(&gone);
            list.write(transaction)?;
            DeadLinks::prune(transaction, &gone, list.pages())?;
            for page in &gone {
                transaction.free(*page)?;
            }
            Ok(())
        })
    }

    pub fn move_pages(&mut self, pages: &[PageId], before: Option<PageId>) -> Result<()> {
        self.store.transact("Move pages", "", |transaction| {
            let mut list = PageList::flatten(transaction)?;
            let moving: HashSet<_> = pages.iter().map(|page| page.0).collect();
            let order = list.pages().to_vec();
            let anchor = before.and_then(|before| {
                let from = order.iter().position(|page| *page == before.0)?;
                order[from..].iter().find(|page| !moving.contains(page)).copied()
            });
            let moved: Vec<_> = order.iter().filter(|page| moving.contains(page)).copied().collect();
            list.remove(&moving);
            let at = list.position_before(anchor);
            list.insert(at, &moved);
            list.write(transaction)
        })
    }

    pub fn insert_blank_page(&mut self, before: Option<PageId>, (width, height): (f64, f64)) -> Result<PageId> {
        self.store.transact("Insert page", "", |transaction| {
            let mut list = PageList::flatten(transaction)?;
            let page: Dict = [
                ("Type", Object::Name("Page".into())),
                ("Parent", Object::Ref(list.root())),
                ("MediaBox", Rect::new(0.0, 0.0, width, height).to_object()),
                ("Resources", Object::Dict(Dict::new())),
            ]
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect();
            let id = transaction.allocate(Object::Dict(page));
            let at = list.position_before(before.map(|before| before.0));
            list.insert(at, &[id]);
            list.write(transaction)?;
            Ok(PageId(id))
        })
    }

    pub fn insert_document(&mut self, other: &Document, before: Option<PageId>) -> Result<Vec<PageId>> {
        let pages = PageTree::new(&other.store).pages()?;
        self.store.transact("Insert pages", "", |transaction| {
            let mut list = PageList::flatten(transaction)?;
            let ids = PageImporter::new(&other.store, list.root()).import(transaction, &pages)?;
            let at = list.position_before(before.map(|before| before.0));
            list.insert(at, &ids);
            list.write(transaction)?;
            Ok(ids.into_iter().map(PageId).collect())
        })
    }

    pub fn extract_pages(&self, pages: &[PageId]) -> Result<Vec<u8>> {
        let all = PageTree::new(&self.store).pages()?;
        let chosen: Vec<_> = pages.iter().filter_map(|id| all.iter().find(|page| page.id == *id).cloned()).collect();
        if chosen.is_empty() {
            return Err(Error::unsupported("no pages to extract"));
        }
        let mut extracted = Document::open(blank_document(), "")?;
        extracted.store.transact("Extract pages", "", |transaction| {
            let mut list = PageList::flatten(transaction)?;
            let ids = PageImporter::new(&self.store, list.root()).import(transaction, &chosen)?;
            list.insert(0, &ids);
            list.write(transaction)
        })?;
        extracted.save(true)
    }
}
