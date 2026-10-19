use std::rc::Rc;

use tiny_skia::Path;

use crate::error::Result;
use crate::fonts::glyph_names::unicode_of;
use crate::fonts::{CidFont, FallbackFace, FontLibrary, FontProgram, SimpleFont, ToUnicode, Type3Font, meaningful};
use crate::store::ObjectStore;
use crate::syntax::{Dict, ObjId, Object};

const DEFAULT_ASCENT: f64 = 0.8;
const DEFAULT_DESCENT: f64 = -0.2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Glyph {
    Outline(u16),
    Procedure(ObjId),
    Missing,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShownGlyph {
    pub code: u32,
    pub byte_length: usize,
    pub glyph: Glyph,
    pub width: f64,
    pub is_word_space: bool,
}

pub enum FontKind {
    Simple(SimpleFont),
    Cid(CidFont),
    Type3(Type3Font),
}

pub struct Font {
    kind: FontKind,
    base_font: String,
    family: Option<String>,
    flags: u32,
    to_unicode: Option<ToUnicode>,
    ascent: f64,
    descent: f64,
}

impl Font {
    pub fn load(font: &Dict, store: &ObjectStore, library: &FontLibrary) -> Result<Font> {
        let kind = match font.name("Subtype").map(|subtype| subtype.as_bytes()) {
            Some(b"Type0") => FontKind::Cid(CidFont::load(font, store, library)?),
            Some(b"Type3") => FontKind::Type3(Type3Font::load(font, store)?),
            _ => FontKind::Simple(SimpleFont::load(font, store, library)?),
        };
        let to_unicode = match font.get("ToUnicode").map(|cmap| store.resolve(cmap)).transpose()? {
            Some(Object::Stream(stream)) => Some(ToUnicode::parse(&store.decode_stream(&stream)?.data)),
            _ => None,
        };
        let (ascent, descent, flags) = descriptor_metrics(font, store)?;
        let base_font =
            font.name("BaseFont").map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()).unwrap_or_default();
        let embedded = match &kind {
            FontKind::Simple(font) => font.embedded_program(),
            FontKind::Cid(font) => font.embedded_program(),
            FontKind::Type3(_) => None,
        };
        let family = embedded.and_then(FontProgram::family_name).or_else(|| {
            let name =
                base_font.split_once('+').filter(|(prefix, _)| prefix.len() == 6).map_or(&*base_font, |(_, name)| name);
            meaningful(name).map(String::from)
        });
        Ok(Font { kind, base_font, family, flags, to_unicode, ascent, descent })
    }

    pub fn kind(&self) -> &FontKind {
        &self.kind
    }

    pub fn decode(&self, bytes: &[u8]) -> Vec<ShownGlyph> {
        match &self.kind {
            FontKind::Simple(font) => bytes
                .iter()
                .map(|&code| ShownGlyph {
                    code: code as u32,
                    byte_length: 1,
                    glyph: font.glyph(code).map_or(Glyph::Missing, Glyph::Outline),
                    width: font.width(code),
                    is_word_space: code == 32,
                })
                .collect(),
            FontKind::Type3(font) => bytes
                .iter()
                .map(|&code| ShownGlyph {
                    code: code as u32,
                    byte_length: 1,
                    glyph: font.procedure(code).map_or(Glyph::Missing, Glyph::Procedure),
                    width: font.width(code),
                    is_word_space: code == 32,
                })
                .collect(),
            FontKind::Cid(font) => {
                let mut glyphs = Vec::new();
                let mut rest = bytes;
                while !rest.is_empty() {
                    let (code, length) = font.next_code(rest);
                    let cid = font.cid(code, length);
                    glyphs.push(ShownGlyph {
                        code,
                        byte_length: length,
                        glyph: font.glyph(cid).map_or(Glyph::Missing, Glyph::Outline),
                        width: font.width(cid),
                        is_word_space: length == 1 && code == 32,
                    });
                    rest = &rest[length..];
                }
                glyphs
            }
        }
    }

    pub fn unicode(&self, code: u32) -> Option<String> {
        if let Some(text) = self.to_unicode.as_ref().and_then(|cmap| cmap.text(code)) {
            return Some(text.to_string());
        }
        let encoding = match &self.kind {
            FontKind::Simple(font) => font.encoding(),
            FontKind::Type3(font) => font.encoding(),
            FontKind::Cid(_) => return None,
        };
        let name = encoding.name(u8::try_from(code).ok()?)?;
        unicode_of(name).map(String::from)
    }

    pub fn encode(&self, character: char) -> Option<(u32, usize)> {
        let text = character.to_string();
        let candidate = match &self.to_unicode {
            Some(cmap) => cmap.code_for(&text),
            None => match &self.kind {
                FontKind::Simple(_) | FontKind::Type3(_) => (0..=255u32)
                    .find(|&code| self.unicode(code).as_deref() == Some(text.as_str()))
                    .map(|code| (code, 1)),
                FontKind::Cid(font) => font.identity_code_for(character),
            },
        }?;
        self.shows_glyph(candidate, character).then_some(candidate)
    }

    fn shows_glyph(&self, (code, length): (u32, usize), character: char) -> bool {
        let bytes = Font::encode_code(code, length);
        self.decode(&bytes).first().is_some_and(|glyph| match glyph.glyph {
            Glyph::Outline(id) => id != 0 && (character.is_whitespace() || self.outline(id).is_some()),
            Glyph::Procedure(_) => true,
            Glyph::Missing => false,
        })
    }

    pub fn encode_code(code: u32, length: usize) -> Vec<u8> {
        code.to_be_bytes()[4 - length.min(4)..].to_vec()
    }

    pub fn outline(&self, glyph: u16) -> Option<Rc<Path>> {
        match &self.kind {
            FontKind::Simple(font) => font.outline(glyph),
            FontKind::Cid(font) => font.outline(glyph),
            FontKind::Type3(_) => None,
        }
    }

    pub fn is_vertical(&self) -> bool {
        matches!(&self.kind, FontKind::Cid(font) if font.is_vertical())
    }

    pub fn embedded_program(&self) -> Option<&FontProgram> {
        match &self.kind {
            FontKind::Simple(font) => font.embedded_program(),
            FontKind::Cid(font) => font.embedded_program(),
            FontKind::Type3(_) => None,
        }
    }

    pub fn base_font(&self) -> &str {
        &self.base_font
    }

    pub fn family_name(&self) -> Option<&str> {
        self.family.as_deref()
    }

    pub fn fallback_face(&self) -> FallbackFace {
        FallbackFace::for_font(&self.base_font, self.flags)
    }

    pub fn vertical_extent(&self) -> (f64, f64) {
        (self.ascent, self.descent)
    }
}

fn descriptor_metrics(font: &Dict, store: &ObjectStore) -> Result<(f64, f64, u32)> {
    let mut holder = font.clone();
    if let Some(descendants) = font.get("DescendantFonts") {
        let descendants = store.resolve(descendants)?;
        if let Some(first) = descendants.as_array().and_then(|items| items.first()) {
            holder = store.resolve(first)?.as_dict().cloned().unwrap_or_default();
        }
    }
    let descriptor = match holder.get("FontDescriptor") {
        Some(descriptor) => store.resolve(descriptor)?.as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    let ascent = descriptor
        .get("Ascent")
        .and_then(Object::as_number)
        .filter(|ascent| *ascent > 0.0)
        .map_or(DEFAULT_ASCENT, |ascent| ascent / 1000.0);
    let descent = descriptor
        .get("Descent")
        .and_then(Object::as_number)
        .filter(|descent| *descent < 0.0)
        .map_or(DEFAULT_DESCENT, |descent| descent / 1000.0);
    Ok((ascent, descent, descriptor.int("Flags").unwrap_or(0) as u32))
}
