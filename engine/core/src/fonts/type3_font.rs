use crate::error::Result;
use crate::fonts::Encoding;
use crate::geometry::Matrix;
use crate::store::ObjectStore;
use crate::syntax::{Dict, ObjId, Object};

pub struct Type3Font {
    pub matrix: Matrix,
    pub resources: Option<Dict>,
    procedures: Dict,
    encoding: Encoding,
    widths: Vec<f64>,
}

impl Type3Font {
    pub fn load(font: &Dict, store: &ObjectStore) -> Result<Type3Font> {
        let matrix = font.get("FontMatrix").and_then(Matrix::from_object).unwrap_or(Matrix::scale(0.001, 0.001));
        let procedures =
            store.resolve(font.get("CharProcs").unwrap_or(&Object::Null))?.as_dict().cloned().unwrap_or_default();
        let resources = match font.get("Resources") {
            Some(resources) => store.resolve(resources)?.as_dict().cloned(),
            None => None,
        };
        let encoding_object = match font.get("Encoding") {
            Some(object) => Some(store.resolve(object)?),
            None => None,
        };
        let encoding = Encoding::read(encoding_object.as_ref(), Encoding::empty());
        let mut widths = vec![0.0; 256];
        let first = font.int("FirstChar").unwrap_or(0).clamp(0, 255) as usize;
        if let Some(listed) = store.resolve(font.get("Widths").unwrap_or(&Object::Null))?.as_array() {
            for (offset, width) in listed.iter().enumerate() {
                if let Some(slot) = widths.get_mut(first + offset) {
                    let glyph_width = store.resolve(width)?.as_number().unwrap_or(0.0);
                    *slot = matrix.apply(glyph_width, 0.0).0 - matrix.e;
                }
            }
        }
        Ok(Type3Font { matrix, resources, procedures, encoding, widths })
    }

    pub fn procedure(&self, code: u8) -> Option<ObjId> {
        let name = self.encoding.name(code)?;
        self.procedures.get(name).and_then(Object::as_reference)
    }

    pub fn encoding(&self) -> &Encoding {
        &self.encoding
    }

    pub fn width(&self, code: u8) -> f64 {
        self.widths[code as usize]
    }
}
