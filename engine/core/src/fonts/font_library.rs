use std::collections::HashMap;
use std::rc::Rc;

use crate::fonts::FallbackFace;

#[derive(Default)]
pub struct FontLibrary {
    faces: HashMap<FallbackFace, Rc<[u8]>>,
}

impl FontLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, face: FallbackFace, data: impl Into<Rc<[u8]>>) {
        self.faces.insert(face, data.into());
    }

    pub fn face(&self, wanted: FallbackFace) -> Option<Rc<[u8]>> {
        let regular = FallbackFace { bold: false, italic: false, ..wanted };
        self.faces.get(&wanted).or_else(|| self.faces.get(&regular)).or_else(|| self.faces.values().next()).cloned()
    }
}
