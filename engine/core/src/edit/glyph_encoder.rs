use std::collections::HashSet;

use crate::edit::fallback_font::FallbackGlyphs;
use crate::edit::show_writer::{FontChoice, PlacedGlyph};
use crate::edit::text_style::{FormatFont, GlyphFormat};
use crate::error::{Error, Result};
use crate::fonts::{FallbackFace, Font, FontLibrary};
use crate::syntax::Name;
use crate::text::{RecordedGlyph, ShowRecord, TextState};

#[derive(Clone, PartialEq)]
enum GlyphSource {
    Bundled(FallbackFace),
    Replacement(String, usize),
}

pub struct GlyphEncoder<'a> {
    library: &'a FontLibrary,
    names_in_use: HashSet<Name>,
    fallbacks: Vec<(GlyphSource, Name, FallbackGlyphs)>,
}

impl<'a> GlyphEncoder<'a> {
    pub fn new(library: &'a FontLibrary, names_in_use: HashSet<Name>) -> Self {
        Self { library, names_in_use, fallbacks: Vec::new() }
    }

    pub fn typed(
        &mut self,
        style: &ShowRecord,
        target: &ShowRecord,
        character: char,
        format: Option<&GlyphFormat>,
    ) -> Result<PlacedGlyph> {
        let font = style.font.as_ref().ok_or_else(|| Error::new("the edited text has no font"))?;
        if font.is_vertical() {
            return Err(Error::unsupported("vertical text cannot be edited yet"));
        }
        let size = match format.and_then(|format| format.size) {
            Some(shown) => shown / target.display_scale(),
            None => style.state.size * style.display_scale() / target.display_scale(),
        };
        let color = format.and_then(|format| format.color);
        let (own, own_name) = match format.map(|format| &format.font) {
            Some(FormatFont::Bundled(face)) => return self.bundled(*face, target, character, size, color),
            Some(FormatFont::Document { name, font }) => (font.as_ref(), Some(name.clone())),
            Some(FormatFont::Kept) | None => (font.as_ref(), style.font_name.clone()),
        };
        if let Some((code, length)) = own.encode(character) {
            let bytes = Font::encode_code(code, length);
            let name = own_name.ok_or_else(|| Error::new("the edited text has no font resource"))?;
            return Ok(PlacedGlyph {
                advance: advance_of(own, &target.state, size, &bytes),
                font: switched_font(target, name, size),
                bytes,
                x: 0.0,
                color,
            });
        }
        for index in 0..self.library.replacements(own.base_font()).len() {
            let source = GlyphSource::Replacement(own.base_font().to_string(), index);
            let (name, glyphs) = self.source(source)?;
            if glyphs.has(character) {
                return Self::place(name, glyphs, target, character, size, color);
            }
        }
        self.bundled(own.fallback_face(), target, character, size, color)
    }

    fn bundled(
        &mut self,
        face: FallbackFace,
        target: &ShowRecord,
        character: char,
        size: f64,
        color: Option<[f64; 3]>,
    ) -> Result<PlacedGlyph> {
        let (name, glyphs) = self.fallback(face)?;
        Self::place(name, glyphs, target, character, size, color)
    }

    fn place(
        name: Name,
        glyphs: &mut FallbackGlyphs,
        target: &ShowRecord,
        character: char,
        size: f64,
        color: Option<[f64; 3]>,
    ) -> Result<PlacedGlyph> {
        let glyph = glyphs.glyph(character)?;
        let state = &target.state;
        let advance = (glyphs.advance(glyph) * size + state.char_spacing) * state.horizontal_scale;
        let font = FontChoice { name, size };
        Ok(PlacedGlyph { bytes: glyph.to_be_bytes().to_vec(), font: Some(font), x: 0.0, advance, color })
    }

    pub fn fallback(&mut self, face: FallbackFace) -> Result<(Name, &mut FallbackGlyphs)> {
        self.source(GlyphSource::Bundled(face))
    }

    fn source(&mut self, source: GlyphSource) -> Result<(Name, &mut FallbackGlyphs)> {
        let index = match self.fallbacks.iter().position(|(known, _, _)| *known == source) {
            Some(index) => index,
            None => {
                let glyphs = match &source {
                    GlyphSource::Bundled(face) => FallbackGlyphs::bundled(
                        *face,
                        self.library.face(*face).ok_or_else(|| Error::new("no fallback face is registered"))?,
                    ),
                    GlyphSource::Replacement(base_font, index) => {
                        FallbackGlyphs::replacement(self.library.replacements(base_font)[*index].clone())
                    }
                };
                let name = (0..)
                    .map(|index| Name::new(format!("TfFallback{index}")))
                    .find(|name| !self.names_in_use.contains(name))
                    .unwrap_or_else(|| Name::from("TfFallback"));
                self.names_in_use.insert(name.clone());
                self.fallbacks.push((source, name, glyphs));
                self.fallbacks.len() - 1
            }
        };
        let (_, name, glyphs) = &mut self.fallbacks[index];
        Ok((name.clone(), glyphs))
    }

    pub fn moved(target: &ShowRecord, source: &ShowRecord, glyph: &RecordedGlyph) -> Result<PlacedGlyph> {
        let font = source.font.as_ref().ok_or_else(|| Error::new("the moved text has no font"))?;
        let name = source.font_name.clone().ok_or_else(|| Error::new("the moved text has no font resource"))?;
        let size = source.state.size * source.display_scale() / target.display_scale();
        let bytes = Font::encode_code(glyph.code, glyph.byte_length);
        let advance = advance_of(font, &target.state, size, &bytes);
        let color = (source.fill != target.fill).then(|| source.fill.rgb()).flatten();
        Ok(PlacedGlyph { bytes, font: switched_font(target, name, size), x: 0.0, advance, color })
    }

    pub fn into_fallbacks(self) -> Vec<(Name, FallbackGlyphs)> {
        self.fallbacks.into_iter().map(|(_, name, glyphs)| (name, glyphs)).collect()
    }
}

fn switched_font(target: &ShowRecord, name: Name, size: f64) -> Option<FontChoice> {
    let same = target.font_name.as_ref() == Some(&name) && (size - target.state.size).abs() < 1e-6;
    (!same).then_some(FontChoice { name, size })
}

fn advance_of(font: &Font, state: &TextState, size: f64, bytes: &[u8]) -> f64 {
    font.decode(bytes).first().map_or(0.0, |glyph| {
        let spacing = state.char_spacing + if glyph.is_word_space { state.word_spacing } else { 0.0 };
        (glyph.width * size + spacing) * state.horizontal_scale
    })
}
