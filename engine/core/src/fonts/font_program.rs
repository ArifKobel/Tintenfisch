use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use tiny_skia::{Path, PathBuilder};
use ttf_parser::{Face, GlyphId, OutlineBuilder, PlatformId, cff, name_id};

use crate::fonts::type1_charstring::CharstringInterpreter;
use crate::fonts::type1_program::Type1Program;
use crate::fonts::{from_postscript, meaningful, with_style};

enum Format {
    Sfnt,
    BareCff,
    Type1(Type1Program),
}

pub struct FontProgram {
    data: Rc<[u8]>,
    format: Format,
    outlines: RefCell<HashMap<u16, Option<Rc<Path>>>>,
    cid_glyphs: RefCell<Option<HashMap<u16, u16>>>,
}

impl FontProgram {
    pub fn parse(data: Rc<[u8]>) -> Option<FontProgram> {
        let format = if Face::parse(&data, 0).is_ok() {
            Format::Sfnt
        } else if cff::Table::parse(&data).is_some() {
            Format::BareCff
        } else {
            return None;
        };
        Some(FontProgram { data, format, outlines: RefCell::new(HashMap::new()), cid_glyphs: RefCell::new(None) })
    }

    pub fn parse_type1(data: Rc<[u8]>, length1: usize) -> Option<FontProgram> {
        let program = Type1Program::parse(&data, length1)?;
        if program.charstrings.is_empty() {
            return None;
        }
        Some(FontProgram {
            data,
            format: Format::Type1(program),
            outlines: RefCell::new(HashMap::new()),
            cid_glyphs: RefCell::new(None),
        })
    }

    fn face(&self) -> Option<Face<'_>> {
        Face::parse(&self.data, 0).ok()
    }

    fn cff(&self) -> Option<cff::Table<'_>> {
        cff::Table::parse(&self.data)
    }

    pub fn family_name(&self) -> Option<String> {
        let face = self.face()?;
        let read = |id: u16| {
            face.names()
                .into_iter()
                .filter(|name| name.name_id == id)
                .find_map(|name| name.to_string())
                .filter(|text| meaningful(text).is_some())
        };
        if let Some(family) = read(name_id::TYPOGRAPHIC_FAMILY).or_else(|| read(name_id::FAMILY)) {
            let style = read(name_id::TYPOGRAPHIC_SUBFAMILY).or_else(|| read(name_id::SUBFAMILY));
            return Some(with_style(&family, style.as_deref()));
        }
        read(name_id::POST_SCRIPT_NAME).and_then(|name| from_postscript(&name)).or_else(|| {
            let unique = read(name_id::UNIQUE_ID)?;
            from_postscript(unique.rsplit(';').next()?)
        })
    }

    pub fn weight(&self) -> Option<u16> {
        self.face()?.tables().os2.map(|os2| os2.weight().to_number())
    }

    pub fn is_bare_cff(&self) -> bool {
        matches!(self.format, Format::BareCff)
    }

    pub fn has_named_glyphs(&self) -> bool {
        matches!(self.format, Format::BareCff | Format::Type1(_))
    }

    pub fn glyph_count(&self) -> u16 {
        match &self.format {
            Format::Sfnt => self.face().map_or(0, |face| face.number_of_glyphs()),
            Format::BareCff => self.cff().map_or(0, |table| table.number_of_glyphs()),
            Format::Type1(program) => program.charstrings.len() as u16,
        }
    }

    pub fn glyph_for_char(&self, character: char) -> Option<u16> {
        let face = self.face()?;
        face.glyph_index(character).map(|glyph| glyph.0)
    }

    pub fn glyph_by_name(&self, name: &str) -> Option<u16> {
        match &self.format {
            Format::Sfnt => self.face()?.glyph_index_by_name(name).map(|glyph| glyph.0),
            Format::BareCff => self.cff()?.glyph_index_by_name(name).map(|glyph| glyph.0),
            Format::Type1(program) => program.glyph(name),
        }
    }

    pub fn glyph_for_symbol(&self, code: u8) -> Option<u16> {
        let face = self.face()?;
        let subtables = face.tables().cmap?.subtables;
        let lookup = |platform: PlatformId, encoding: u16, candidates: &[u32]| {
            subtables
                .into_iter()
                .filter(|table| table.platform_id == platform && table.encoding_id == encoding)
                .find_map(|table| {
                    candidates.iter().find_map(|&candidate| table.glyph_index(candidate)).filter(|glyph| glyph.0 != 0)
                })
        };
        let code = code as u32;
        lookup(PlatformId::Windows, 0, &[code, 0xF000 | code, 0xF100 | code, 0xF200 | code])
            .or_else(|| lookup(PlatformId::Macintosh, 0, &[code]))
            .map(|glyph| glyph.0)
    }

    pub fn has_unicode_cmap(&self) -> bool {
        self.face()
            .and_then(|face| face.tables().cmap)
            .is_some_and(|cmap| cmap.subtables.into_iter().any(|table| table.is_unicode()))
    }

    pub fn glyph_for_mac_code(&self, code: u8) -> Option<u16> {
        let face = self.face()?;
        let subtables = face.tables().cmap?.subtables;
        subtables
            .into_iter()
            .filter(|table| table.platform_id == PlatformId::Macintosh && table.encoding_id == 0)
            .find_map(|table| table.glyph_index(code as u32))
            .map(|glyph| glyph.0)
    }

    pub fn glyph_for_builtin_code(&self, code: u8) -> Option<u16> {
        match &self.format {
            Format::Type1(program) => program.builtin_name(code).and_then(|name| program.glyph(name)),
            _ => self.cff()?.glyph_index(code).map(|glyph| glyph.0).filter(|&glyph| glyph != 0),
        }
    }

    pub fn glyph_for_cid(&self, cid: u16) -> Option<u16> {
        if !self.is_bare_cff() {
            return Some(cid);
        }
        let mut cached = self.cid_glyphs.borrow_mut();
        let map = cached.get_or_insert_with(|| {
            let table = self.cff();
            let mut map = HashMap::new();
            if let Some(table) = table {
                for glyph in 0..table.number_of_glyphs() {
                    match table.glyph_cid(GlyphId(glyph)) {
                        Some(cid) => map.insert(cid, glyph),
                        None => map.insert(glyph, glyph),
                    };
                }
            }
            map
        });
        map.get(&cid).copied()
    }

    pub fn advance(&self, glyph: u16) -> Option<f64> {
        match &self.format {
            Format::Sfnt => {
                let face = self.face()?;
                Some(face.glyph_hor_advance(GlyphId(glyph))? as f64 / face.units_per_em() as f64)
            }
            Format::BareCff => {
                let table = self.cff()?;
                Some(table.glyph_width(GlyphId(glyph))? as f64 * table.matrix().sx as f64)
            }
            Format::Type1(program) => {
                let (_, advance) = CharstringInterpreter::new(program).run(glyph);
                Some(program.matrix.apply(advance, 0.0).0 - program.matrix.e)
            }
        }
    }

    pub fn outline(&self, glyph: u16) -> Option<Rc<Path>> {
        if let Some(cached) = self.outlines.borrow().get(&glyph) {
            return cached.clone();
        }
        let outline = self.trace(glyph).map(Rc::new);
        self.outlines.borrow_mut().insert(glyph, outline.clone());
        outline
    }

    fn trace(&self, glyph: u16) -> Option<Path> {
        match &self.format {
            Format::Sfnt => {
                let face = self.face()?;
                let scale = 1.0 / face.units_per_em() as f32;
                let mut builder = EmOutline::new([scale, 0.0, 0.0, scale, 0.0, 0.0]);
                face.outline_glyph(GlyphId(glyph), &mut builder)?;
                builder.finish()
            }
            Format::BareCff => {
                let table = self.cff()?;
                let matrix = table.matrix();
                let mut builder = EmOutline::new([matrix.sx, matrix.ky, matrix.kx, matrix.sy, matrix.tx, matrix.ty]);
                table.outline(GlyphId(glyph), &mut builder).ok()?;
                builder.finish()
            }
            Format::Type1(program) => {
                let (path, _) = CharstringInterpreter::new(program).run(glyph);
                path?.transform(program.matrix.to_skia())
            }
        }
    }
}

struct EmOutline {
    matrix: [f32; 6],
    builder: PathBuilder,
}

impl EmOutline {
    fn new(matrix: [f32; 6]) -> Self {
        Self { matrix, builder: PathBuilder::new() }
    }

    fn map(&self, x: f32, y: f32) -> (f32, f32) {
        let [a, b, c, d, e, f] = self.matrix;
        (a * x + c * y + e, b * x + d * y + f)
    }

    fn finish(self) -> Option<Path> {
        self.builder.finish()
    }
}

impl OutlineBuilder for EmOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.map(x, y);
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.map(x, y);
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x1, y1) = self.map(x1, y1);
        let (x, y) = self.map(x, y);
        self.builder.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (x1, y1) = self.map(x1, y1);
        let (x2, y2) = self.map(x2, y2);
        let (x, y) = self.map(x, y);
        self.builder.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
    }
}
