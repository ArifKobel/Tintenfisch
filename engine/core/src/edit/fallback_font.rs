use std::collections::BTreeMap;
use std::fmt::Write;
use std::rc::Rc;

use ttf_parser::{Face, GlyphId, name_id};

use crate::error::{Error, Result};
use crate::fonts::{FallbackFace, FallbackFamily, subset};
use crate::store::Transaction;
use crate::syntax::{Dict, Name, ObjId, Object, Stream};

pub struct FallbackGlyphs {
    stem: String,
    flags: i64,
    data: Rc<[u8]>,
    glyphs: BTreeMap<char, u16>,
}

impl FallbackGlyphs {
    pub fn bundled(face: FallbackFace, data: Rc<[u8]>) -> Self {
        let family = match face.family {
            FallbackFamily::Mono => 1,
            FallbackFamily::Serif => 2,
            FallbackFamily::Sans => 0,
        };
        let flags = family | 32 | if face.italic { 64 } else { 0 };
        Self { stem: face.file_name().trim_end_matches(".ttf").to_string(), flags, data, glyphs: BTreeMap::new() }
    }

    pub fn replacement(data: Rc<[u8]>) -> Self {
        let face = Face::parse(&data, 0).ok();
        let stem = face
            .as_ref()
            .and_then(|face| {
                face.names()
                    .into_iter()
                    .filter(|name| name.name_id == name_id::POST_SCRIPT_NAME)
                    .find_map(|name| name.to_string())
            })
            .map(|name| name.chars().filter(char::is_ascii_alphanumeric).collect::<String>())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "Replacement".to_string());
        let italic = face.as_ref().is_some_and(Face::is_italic);
        Self { stem, flags: 32 | if italic { 64 } else { 0 }, data, glyphs: BTreeMap::new() }
    }

    pub fn has(&self, character: char) -> bool {
        self.parsed().is_ok_and(|face| face.glyph_index(character).is_some_and(|glyph| glyph.0 != 0))
    }

    fn parsed(&self) -> Result<Face<'_>> {
        Face::parse(&self.data, 0).map_err(|_| Error::new("the fallback face cannot be read"))
    }

    pub fn glyph(&mut self, character: char) -> Result<u16> {
        let glyph = self
            .parsed()?
            .glyph_index(character)
            .map(|glyph| glyph.0)
            .ok_or_else(|| Error::unsupported(format!("no font can show {character:?}")))?;
        self.glyphs.insert(character, glyph);
        Ok(glyph)
    }

    pub fn vertical_extent(&self) -> (f64, f64) {
        self.parsed().map_or((0.8, 0.2), |face| {
            let per_em = face.units_per_em() as f64;
            (face.ascender() as f64 / per_em, -(face.descender() as f64) / per_em)
        })
    }

    pub fn advance(&self, glyph: u16) -> f64 {
        self.parsed()
            .map_or(0.0, |face| face.glyph_hor_advance(GlyphId(glyph)).unwrap_or(0) as f64 / face.units_per_em() as f64)
    }

    pub fn embed(&self, transaction: &mut Transaction) -> Result<ObjId> {
        let face = self.parsed()?;
        let per_em = 1000.0 / face.units_per_em() as f64;
        let scaled = |value: i16| Object::Real(value as f64 * per_em);
        let glyph_ids = self.glyphs.values().copied().collect();
        let program = subset(&self.data, &glyph_ids)?;
        let base_font = Name::new(format!("{}+{}", subset_tag(&self.glyphs), self.stem));

        let file = transaction.allocate(Object::Stream(Stream {
            dict: [(Name::from("Length1"), Object::Int(program.len() as i64))].into_iter().collect(),
            raw: Rc::from(program),
        }));
        let bounds = face.global_bounding_box();
        let descriptor: Dict = [
            ("Type", Object::Name(Name::from("FontDescriptor"))),
            ("FontName", Object::Name(base_font.clone())),
            ("Flags", Object::Int(self.flags)),
            (
                "FontBBox",
                Object::Array(vec![
                    scaled(bounds.x_min),
                    scaled(bounds.y_min),
                    scaled(bounds.x_max),
                    scaled(bounds.y_max),
                ]),
            ),
            ("ItalicAngle", Object::Real(face.italic_angle() as f64)),
            ("Ascent", scaled(face.ascender())),
            ("Descent", scaled(face.descender())),
            ("CapHeight", scaled(face.capital_height().unwrap_or(face.ascender()))),
            ("StemV", Object::Int(80)),
            ("FontFile2", Object::Ref(file)),
        ]
        .into_iter()
        .map(|(key, value)| (Name::from(key), value))
        .collect();
        let descriptor = transaction.allocate(Object::Dict(descriptor));

        let widths: Vec<Object> = self
            .glyphs
            .values()
            .flat_map(|&glyph| {
                [Object::Int(glyph as i64), Object::Array(vec![Object::Real(self.advance(glyph) * 1000.0)])]
            })
            .collect();
        let system_info: Dict = [
            (Name::from("Registry"), Object::String(b"Adobe".to_vec())),
            (Name::from("Ordering"), Object::String(b"Identity".to_vec())),
            (Name::from("Supplement"), Object::Int(0)),
        ]
        .into_iter()
        .collect();
        let descendant: Dict = [
            ("Type", Object::Name(Name::from("Font"))),
            ("Subtype", Object::Name(Name::from("CIDFontType2"))),
            ("BaseFont", Object::Name(base_font.clone())),
            ("CIDSystemInfo", Object::Dict(system_info)),
            ("FontDescriptor", Object::Ref(descriptor)),
            ("W", Object::Array(widths)),
            ("CIDToGIDMap", Object::Name(Name::from("Identity"))),
        ]
        .into_iter()
        .map(|(key, value)| (Name::from(key), value))
        .collect();
        let descendant = transaction.allocate(Object::Dict(descendant));
        let to_unicode = transaction
            .allocate(Object::Stream(Stream { dict: Dict::new(), raw: Rc::from(self.to_unicode_cmap().into_bytes()) }));
        let font: Dict = [
            ("Type", Object::Name(Name::from("Font"))),
            ("Subtype", Object::Name(Name::from("Type0"))),
            ("BaseFont", Object::Name(base_font)),
            ("Encoding", Object::Name(Name::from("Identity-H"))),
            ("DescendantFonts", Object::Array(vec![Object::Ref(descendant)])),
            ("ToUnicode", Object::Ref(to_unicode)),
        ]
        .into_iter()
        .map(|(key, value)| (Name::from(key), value))
        .collect();
        Ok(transaction.allocate(Object::Dict(font)))
    }

    fn to_unicode_cmap(&self) -> String {
        let mut cmap = String::from(
            "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def /CMapType 2 def\n1 begincodespacerange <0000> <FFFF> endcodespacerange\n",
        );
        let _ = writeln!(cmap, "{} beginbfchar", self.glyphs.len());
        for (character, glyph) in &self.glyphs {
            let units: String = character.encode_utf16(&mut [0; 2]).iter().map(|unit| format!("{unit:04X}")).collect();
            let _ = writeln!(cmap, "<{glyph:04X}> <{units}>");
        }
        cmap.push_str("endbfchar\nendcmap CMapName currentdict /CMap defineresource pop end end\n");
        cmap
    }
}

fn subset_tag(glyphs: &BTreeMap<char, u16>) -> String {
    let mut seed = glyphs
        .values()
        .fold(0x9E37_79B9u32, |hash, &glyph| hash.rotate_left(5) ^ glyph as u32)
        .wrapping_mul(2_654_435_761);
    (0..6)
        .map(|_| {
            let letter = (b'A' + (seed % 26) as u8) as char;
            seed /= 26;
            letter
        })
        .collect()
}
