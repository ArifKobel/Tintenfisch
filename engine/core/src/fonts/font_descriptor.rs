use std::rc::Rc;

use crate::error::Result;
use crate::fonts::FontProgram;
use crate::store::ObjectStore;
use crate::syntax::{Dict, Object};

const SYMBOLIC: u32 = 1 << 2;

pub struct FontDescriptor {
    pub flags: u32,
    pub missing_width: f64,
    pub program: Option<FontProgram>,
}

impl FontDescriptor {
    pub fn read(font: &Dict, store: &ObjectStore) -> Result<FontDescriptor> {
        let descriptor = match font.get("FontDescriptor") {
            Some(object) => store.resolve(object)?,
            None => Object::Null,
        };
        let descriptor = descriptor.as_dict().cloned().unwrap_or_default();
        let mut program = None;
        for key in ["FontFile2", "FontFile3"] {
            if let Some(reference) = descriptor.get(key) {
                let file = store.resolve(reference)?;
                if let Some(stream) = file.as_stream() {
                    let data: Rc<[u8]> = Rc::from(store.decode_stream(stream)?.data);
                    program = FontProgram::parse(data);
                }
            }
        }
        if program.is_none()
            && let Some(reference) = descriptor.get("FontFile")
            && let Some(stream) = store.resolve(reference)?.as_stream()
        {
            let length1 = store.resolve(stream.dict.get("Length1").unwrap_or(&Object::Null))?.as_int().unwrap_or(0);
            let data: Rc<[u8]> = Rc::from(store.decode_stream(stream)?.data);
            program = FontProgram::parse_type1(data, length1.max(0) as usize);
        }
        Ok(FontDescriptor {
            flags: descriptor.int("Flags").unwrap_or(0) as u32,
            missing_width: descriptor.get("MissingWidth").and_then(Object::as_number).unwrap_or(0.0),
            program,
        })
    }

    pub fn is_symbolic(&self) -> bool {
        self.flags & SYMBOLIC != 0
    }
}
