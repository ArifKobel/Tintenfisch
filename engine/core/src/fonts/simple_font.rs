use std::rc::Rc;

use tiny_skia::Path;

use crate::error::Result;
use crate::fonts::encoding_tables::mac_roman;
use crate::fonts::glyph_names::unicode_of;
use crate::fonts::{Encoding, FallbackFace, FontDescriptor, FontLibrary, FontProgram};
use crate::store::ObjectStore;
use crate::syntax::{Dict, Object};

pub struct SimpleFont {
    program: Option<FontProgram>,
    glyphs: Vec<Option<u16>>,
    widths: Vec<f64>,
    encoding: Encoding,
    substituted: bool,
}

impl SimpleFont {
    pub fn load(font: &Dict, store: &ObjectStore, library: &FontLibrary) -> Result<SimpleFont> {
        let descriptor = FontDescriptor::read(font, store)?;
        let base_font =
            font.name("BaseFont").map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()).unwrap_or_default();
        let is_truetype = font.name("Subtype").is_some_and(|subtype| subtype.is("TrueType"));
        let symbolic = descriptor.is_symbolic();
        let encoding_object = match font.get("Encoding") {
            Some(object) => Some(store.resolve(object)?),
            None => None,
        };
        let default = match (symbolic, is_truetype) {
            (true, _) => Encoding::empty(),
            (false, true) => Encoding::win_ansi(),
            (false, false) => Encoding::standard(),
        };
        let encoding = Encoding::read(encoding_object.as_ref(), default);
        let explicit_encoding = encoding_object.is_some();

        let missing_width = descriptor.missing_width;
        let (program, substituted) = match descriptor.program {
            Some(program) => (Some(program), false),
            None => {
                let face = FallbackFace::for_font(&base_font, descriptor.flags);
                (library.face(face).and_then(FontProgram::parse), true)
            }
        };
        let mut glyphs = vec![None; 256];
        if let Some(program) = &program {
            for (code, glyph) in glyphs.iter_mut().enumerate() {
                *glyph =
                    map_code(program, &encoding, code as u8, symbolic && !substituted, explicit_encoding, substituted);
            }
        }
        let widths = read_widths(font, store, missing_width, program.as_ref(), &glyphs)?;
        Ok(SimpleFont { program, glyphs, widths, encoding, substituted })
    }

    pub fn glyph(&self, code: u8) -> Option<u16> {
        self.glyphs[code as usize]
    }

    pub fn width(&self, code: u8) -> f64 {
        self.widths[code as usize]
    }

    pub fn outline(&self, glyph: u16) -> Option<Rc<Path>> {
        self.program.as_ref()?.outline(glyph)
    }

    pub fn encoding(&self) -> &Encoding {
        &self.encoding
    }

    pub fn is_substituted(&self) -> bool {
        self.substituted
    }
}

fn map_code(
    program: &FontProgram,
    encoding: &Encoding,
    code: u8,
    symbolic: bool,
    explicit_encoding: bool,
    substituted: bool,
) -> Option<u16> {
    let name = encoding.name(code);
    if program.has_named_glyphs() {
        let by_name = name.filter(|_| explicit_encoding).and_then(|name| program.glyph_by_name(name));
        return by_name
            .or_else(|| program.glyph_for_builtin_code(code))
            .or_else(|| name.and_then(|name| program.glyph_by_name(name)));
    }
    if substituted {
        let character = name.and_then(unicode_of).or_else(|| (code >= 32).then_some(code as char))?;
        return program.glyph_for_char(character).filter(|&glyph| glyph != 0);
    }
    if (symbolic || !program.has_unicode_cmap())
        && let Some(glyph) = program.glyph_for_symbol(code)
    {
        return Some(glyph);
    }
    let by_unicode =
        name.and_then(unicode_of).and_then(|character| program.glyph_for_char(character)).filter(|&glyph| glyph != 0);
    by_unicode
        .or_else(|| {
            let mac_code = name.and_then(|name| mac_roman().iter().position(|entry| *entry == Some(name)))?;
            program.glyph_for_mac_code(mac_code as u8)
        })
        .or_else(|| name.and_then(|name| program.glyph_by_name(name)))
        .or_else(|| program.glyph_for_symbol(code))
}

fn read_widths(
    font: &Dict,
    store: &ObjectStore,
    missing_width: f64,
    program: Option<&FontProgram>,
    glyphs: &[Option<u16>],
) -> Result<Vec<f64>> {
    let mut widths = vec![missing_width / 1000.0; 256];
    let listed = match font.get("Widths") {
        Some(object) => store.resolve(object)?,
        None => Object::Null,
    };
    if let Some(listed) = listed.as_array() {
        let first = font.int("FirstChar").unwrap_or(0).clamp(0, 255) as usize;
        for (offset, width) in listed.iter().enumerate() {
            let width = store.resolve(width)?.as_number().unwrap_or(0.0);
            if let Some(slot) = widths.get_mut(first + offset) {
                *slot = width / 1000.0;
            }
        }
        return Ok(widths);
    }
    if let Some(program) = program {
        for (slot, glyph) in widths.iter_mut().zip(glyphs) {
            if let Some(advance) = glyph.and_then(|glyph| program.advance(glyph)) {
                *slot = advance;
            }
        }
    }
    Ok(widths)
}
