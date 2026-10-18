use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::error::{Error, Result};
use crate::fonts::{Font, FontLibrary};
use crate::store::ObjectStore;
use crate::syntax::{ObjId, Object};

type LoadedFonts = HashMap<ObjId, (Rc<Object>, Rc<Font>)>;

#[derive(Default)]
pub struct FontCache {
    fonts: RefCell<LoadedFonts>,
}

impl FontCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn font(&self, object: &Object, store: &ObjectStore, library: &FontLibrary) -> Result<Rc<Font>> {
        let Some(id) = object.as_reference() else {
            return Self::load(object, store, library);
        };
        let source = store.object(id)?;
        if let Some((cached_source, font)) = self.fonts.borrow().get(&id)
            && Rc::ptr_eq(cached_source, &source)
        {
            return Ok(font.clone());
        }
        let font = Self::load(&source, store, library)?;
        self.fonts.borrow_mut().insert(id, (source, font.clone()));
        Ok(font)
    }

    fn load(object: &Object, store: &ObjectStore, library: &FontLibrary) -> Result<Rc<Font>> {
        let resolved = store.resolve(object)?;
        let dict = resolved.as_dict().ok_or_else(|| Error::new("font is not a dictionary"))?;
        Ok(Rc::new(Font::load(dict, store, library)?))
    }
}
