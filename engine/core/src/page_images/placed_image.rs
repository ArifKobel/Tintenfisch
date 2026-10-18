use crate::content::{ContentStream, Drawings, Segment};
use crate::error::{Error, Result};
use crate::geometry::Matrix;
use crate::page_images::ImageFrame;
use crate::pages::Page;
use crate::render::DisplaySpace;
use crate::store::ObjectStore;
use crate::syntax::{Dict, ObjId};

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedImage {
    pub scope: Option<ObjId>,
    pub ordinal: usize,
    pub matrix: Matrix,
    pub resources: Dict,
    pub frame: ImageFrame,
}

impl PlacedImage {
    pub fn on_page(store: &ObjectStore, page: &Page) -> Result<Vec<PlacedImage>> {
        let space = DisplaySpace::of(page)?;
        let drawings = Drawings::of(store, page)?;
        Ok(drawings
            .images
            .into_iter()
            .map(|draw| PlacedImage {
                scope: draw.scope,
                ordinal: draw.ordinal,
                frame: ImageFrame::of(&draw.matrix.then(&space.user_to_display())),
                matrix: draw.matrix,
                resources: draw.resources,
            })
            .collect())
    }

    pub fn nth(store: &ObjectStore, page: &Page, index: usize) -> Result<PlacedImage> {
        PlacedImage::on_page(store, page)?
            .into_iter()
            .nth(index)
            .ok_or_else(|| Error::new(format!("page {} has no image {index}", page.id)))
    }

    pub fn content(&self, store: &ObjectStore, page: &Page) -> Result<ContentStream> {
        let Some(form) = self.scope else { return ContentStream::load(store, page) };
        let object = store.object(form)?;
        let stream = object.as_stream().ok_or_else(|| Error::new("a form XObject is not a stream"))?;
        let data = store.decode_stream(stream)?.data;
        Ok(ContentStream::parse(vec![Segment { stream: form, data }]))
    }
}
