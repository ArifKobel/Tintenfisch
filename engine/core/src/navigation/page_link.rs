use crate::annotations::Annotation;
use crate::error::Result;
use crate::geometry::Rect;
use crate::navigation::Target;
use crate::pages::Page;
use crate::render::DisplaySpace;
use crate::store::ObjectStore;
use crate::syntax::ObjId;

#[derive(Clone, Debug, PartialEq)]
pub struct PageLink {
    pub id: ObjId,
    pub rect: Rect,
    pub target: Option<Target>,
}

impl PageLink {
    pub fn on_page(store: &ObjectStore, page: &Page) -> Result<Vec<PageLink>> {
        let space = DisplaySpace::of(page)?;
        let mut links = Vec::new();
        for annotation in Annotation::on_page(store, page)? {
            let Some(id) = annotation.id.filter(|_| annotation.subtype.is("Link")) else { continue };
            links.push(PageLink {
                id,
                rect: space.rect_to_display(annotation.rect),
                target: Target::of(store, &annotation.dict)?,
            });
        }
        Ok(links)
    }
}
