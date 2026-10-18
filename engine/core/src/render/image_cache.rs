use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::error::Result;
use crate::image::DecodedImage;
use crate::syntax::{ObjId, Object};

const BUDGET: usize = 192 * 1024 * 1024;

type DecodedImages = HashMap<ObjId, (Rc<Object>, Rc<DecodedImage>)>;

#[derive(Default)]
pub struct ImageCache {
    images: RefCell<DecodedImages>,
    bytes: Cell<usize>,
}

impl ImageCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn image(
        &self,
        id: ObjId,
        source: Rc<Object>,
        load: impl FnOnce() -> Result<DecodedImage>,
    ) -> Result<Rc<DecodedImage>> {
        if let Some((cached_source, image)) = self.images.borrow().get(&id)
            && Rc::ptr_eq(cached_source, &source)
        {
            return Ok(image.clone());
        }
        let image = Rc::new(load()?);
        if self.bytes.get() + image.byte_size() > BUDGET {
            self.images.borrow_mut().clear();
            self.bytes.set(0);
        }
        self.bytes.set(self.bytes.get() + image.byte_size());
        self.images.borrow_mut().insert(id, (source, image.clone()));
        Ok(image)
    }
}
