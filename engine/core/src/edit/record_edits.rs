use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::edit::show_writer::PlacedGlyph;
use crate::fonts::Font;
use crate::text::{GlyphOrigin, ShowRecord};

#[derive(Default)]
pub struct RecordEdits {
    placed: HashMap<GlyphOrigin, Option<f64>>,
    inserted: BTreeMap<usize, Vec<(usize, Vec<PlacedGlyph>)>>,
    touched: BTreeSet<usize>,
}

impl RecordEdits {
    pub fn delete(&mut self, origin: GlyphOrigin) {
        self.placed.insert(origin, None);
        self.touched.insert(origin.ordinal);
    }

    pub fn place(&mut self, origin: GlyphOrigin, x: f64) {
        self.placed.insert(origin, Some(x));
        self.touched.insert(origin.ordinal);
    }

    pub fn insert(&mut self, ordinal: usize, before_glyph: usize, glyphs: Vec<PlacedGlyph>) {
        self.inserted.entry(ordinal).or_default().push((before_glyph, glyphs));
        self.touched.insert(ordinal);
    }

    pub fn touch(&mut self, ordinal: usize) {
        self.touched.insert(ordinal);
    }

    pub fn touched(&self) -> impl Iterator<Item = usize> + '_ {
        self.touched.iter().copied()
    }

    pub fn glyphs(&self, record: &ShowRecord) -> Vec<PlacedGlyph> {
        let insertions = self.inserted.get(&record.ordinal).map(Vec::as_slice).unwrap_or_default();
        let inserted_before = |index: usize| {
            insertions.iter().filter(move |(before, _)| *before == index).flat_map(|(_, glyphs)| glyphs.iter().cloned())
        };
        let mut glyphs = Vec::new();
        for (index, glyph) in record.glyphs.iter().enumerate() {
            glyphs.extend(inserted_before(index));
            let origin = GlyphOrigin { form: record.form, ordinal: record.ordinal, index };
            let x = match self.placed.get(&origin) {
                Some(None) => continue,
                Some(Some(x)) => *x,
                None => glyph.x,
            };
            glyphs.push(PlacedGlyph {
                bytes: Font::encode_code(glyph.code, glyph.byte_length),
                font: None,
                x,
                advance: glyph.advance,
                color: None,
            });
        }
        glyphs.extend(
            insertions
                .iter()
                .filter(|(before, _)| *before >= record.glyphs.len())
                .flat_map(|(_, glyphs)| glyphs.iter().cloned()),
        );
        glyphs
    }
}
