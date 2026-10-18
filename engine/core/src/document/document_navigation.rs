use crate::annotations::{annotation_list, write_annotation_list};
use crate::document::Document;
use crate::error::{Error, Result};
use crate::geometry::Rect;
use crate::navigation::{OutlineItem, OutlineTree, PageLink, Target};
use crate::pages::PageId;
use crate::render::DisplaySpace;
use crate::syntax::{Dict, Name, ObjId, Object, encode_text};

const PRINT: i64 = 4;

impl Document {
    pub fn outline(&self) -> Result<Vec<OutlineItem>> {
        OutlineTree::load(&self.store)?.items(&self.store)
    }

    pub fn add_bookmark(
        &mut self,
        parent: Option<ObjId>,
        index: usize,
        title: &str,
        target: Option<&Target>,
    ) -> Result<ObjId> {
        let mut tree = OutlineTree::load(&self.store)?;
        self.store.transact("Add bookmark", "", |transaction| {
            let mut dict: Dict = [(Name::from("Title"), Object::String(encode_text(title)))].into_iter().collect();
            if let Some(target) = target {
                target.write(&mut dict);
            }
            let id = transaction.allocate(Object::Dict(dict.clone()));
            tree.insert(parent, index, id, dict)?;
            tree.write(transaction)?;
            Ok(id)
        })
    }

    pub fn rename_bookmark(&mut self, id: ObjId, title: &str) -> Result<()> {
        self.edit_outline("Rename bookmark", |tree| tree.rename(id, title))
    }

    pub fn retarget_bookmark(&mut self, id: ObjId, target: &Target) -> Result<()> {
        self.edit_outline("Change bookmark destination", |tree| tree.retarget(id, target))
    }

    pub fn move_bookmark(&mut self, id: ObjId, parent: Option<ObjId>, index: usize) -> Result<()> {
        self.edit_outline("Move bookmark", |tree| tree.move_to(id, parent, index))
    }

    pub fn delete_bookmark(&mut self, id: ObjId) -> Result<()> {
        self.edit_outline("Delete bookmark", |tree| tree.remove(id))
    }

    pub fn links(&self, page: PageId) -> Result<Vec<PageLink>> {
        PageLink::on_page(&self.store, &self.page(page)?)
    }

    pub fn add_link(&mut self, page: PageId, rect: Rect, target: &Target) -> Result<ObjId> {
        let page = self.page(page)?;
        let rect = DisplaySpace::of(&page)?.rect_to_user(rect);
        self.store.transact("Add link", "", |transaction| {
            let mut dict: Dict = [
                ("Type", Object::Name(Name::from("Annot"))),
                ("Subtype", Object::Name(Name::from("Link"))),
                ("Rect", rect.to_object()),
                ("Border", Object::Array(vec![Object::Int(0), Object::Int(0), Object::Int(0)])),
                ("F", Object::Int(PRINT)),
                ("P", Object::Ref(page.id.0)),
            ]
            .into_iter()
            .map(|(key, value)| (Name::from(key), value))
            .collect();
            target.write(&mut dict);
            let id = transaction.allocate(Object::Dict(dict));
            let mut annotations = annotation_list(transaction, page.id.0)?;
            annotations.push(Object::Ref(id));
            write_annotation_list(transaction, page.id.0, annotations)?;
            Ok(id)
        })
    }

    pub fn update_link(&mut self, page: PageId, id: ObjId, rect: Rect, target: &Target) -> Result<()> {
        let page = self.page(page)?;
        let rect = DisplaySpace::of(&page)?.rect_to_user(rect);
        self.store.transact("Change link", "", |transaction| {
            let mut dict = transaction.object(id)?.as_dict().cloned().ok_or_else(|| Error::new("the link is gone"))?;
            if !dict.name("Subtype").is_some_and(|subtype| subtype.is("Link")) {
                return Err(Error::new("the annotation is not a link"));
            }
            dict.insert("Rect", rect.to_object());
            target.write(&mut dict);
            transaction.put(id, Object::Dict(dict))
        })
    }

    pub fn top_to_display(&self, page: PageId, top: f64) -> Result<f64> {
        let space = DisplaySpace::of(&self.page(page)?)?;
        Ok(space.user_to_display().apply(0.0, top).1)
    }

    pub fn top_to_user(&self, page: PageId, y: f64) -> Result<f64> {
        let space = DisplaySpace::of(&self.page(page)?)?;
        Ok(space.point_to_user((0.0, y)).1)
    }

    fn edit_outline(&mut self, label: &str, change: impl FnOnce(&mut OutlineTree) -> Result<()>) -> Result<()> {
        let mut tree = OutlineTree::load(&self.store)?;
        change(&mut tree)?;
        self.store.transact(label, "", |transaction| tree.write(transaction))
    }
}
