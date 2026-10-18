use std::rc::Rc;

use tiny_skia::Path;

use crate::error::{Error, Result};
use crate::fonts::{CidCmap, FallbackFace, FontDescriptor, FontLibrary, FontProgram};
use crate::store::ObjectStore;
use crate::syntax::{Dict, Object};

enum CidToGid {
    Identity,
    Table(Vec<u16>),
}

pub struct CidFont {
    program: Option<FontProgram>,
    cmap: CidCmap,
    cid_to_gid: CidToGid,
    default_width: f64,
    widths: Vec<(u32, u32, f64)>,
}

impl CidFont {
    pub fn load(font: &Dict, store: &ObjectStore, library: &FontLibrary) -> Result<CidFont> {
        let descendants = store.resolve(font.get("DescendantFonts").unwrap_or(&Object::Null))?;
        let descendant = descendants
            .as_array()
            .and_then(|items| items.first())
            .map(|first| store.resolve(first))
            .transpose()?
            .and_then(|descendant| descendant.as_dict().cloned())
            .ok_or_else(|| Error::new("Type0 font without a descendant font"))?;
        let cmap = match font.get("Encoding").map(|encoding| store.resolve(encoding)).transpose()? {
            Some(Object::Name(name)) => CidCmap::predefined(&name),
            Some(Object::Stream(stream)) => CidCmap::parse(&store.decode_stream(&stream)?.data),
            _ => CidCmap::identity(false),
        };
        let descriptor = FontDescriptor::read(&descendant, store)?;
        let program = match descriptor.program {
            Some(program) => Some(program),
            None => {
                let base_font = font
                    .name("BaseFont")
                    .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                    .unwrap_or_default();
                library.face(FallbackFace::for_font(&base_font, descriptor.flags)).and_then(FontProgram::parse)
            }
        };
        let cid_to_gid = match descendant.get("CIDToGIDMap").map(|map| store.resolve(map)).transpose()? {
            Some(Object::Stream(stream)) => {
                let data = store.decode_stream(&stream)?.data;
                CidToGid::Table(data.as_chunks::<2>().0.iter().map(|&pair| u16::from_be_bytes(pair)).collect())
            }
            _ => CidToGid::Identity,
        };
        let default_width = descendant.get("DW").and_then(Object::as_number).unwrap_or(1000.0) / 1000.0;
        let widths = read_widths(&descendant, store)?;
        Ok(CidFont { program, cmap, cid_to_gid, default_width, widths })
    }

    pub fn next_code(&self, bytes: &[u8]) -> (u32, usize) {
        self.cmap.next_code(bytes)
    }

    pub fn cid(&self, code: u32, length: usize) -> u32 {
        self.cmap.cid(code, length)
    }

    pub fn glyph(&self, cid: u32) -> Option<u16> {
        let cid = u16::try_from(cid).ok()?;
        match &self.cid_to_gid {
            CidToGid::Table(table) => table.get(cid as usize).copied(),
            CidToGid::Identity => self.program.as_ref()?.glyph_for_cid(cid),
        }
    }

    pub fn width(&self, cid: u32) -> f64 {
        self.widths
            .iter()
            .find(|(first, last, _)| (*first..=*last).contains(&cid))
            .map_or(self.default_width, |(_, _, width)| *width)
    }

    pub fn identity_code_for(&self, character: char) -> Option<(u32, usize)> {
        if !self.cmap.is_identity() || !matches!(self.cid_to_gid, CidToGid::Identity) {
            return None;
        }
        let glyph = self.program.as_ref()?.glyph_for_char(character)?;
        Some((glyph as u32, 2))
    }

    pub fn is_vertical(&self) -> bool {
        self.cmap.vertical
    }

    pub fn outline(&self, glyph: u16) -> Option<Rc<Path>> {
        self.program.as_ref()?.outline(glyph)
    }
}

fn read_widths(descendant: &Dict, store: &ObjectStore) -> Result<Vec<(u32, u32, f64)>> {
    let listed = store.resolve(descendant.get("W").unwrap_or(&Object::Null))?;
    let items = listed.as_array().unwrap_or_default();
    let mut widths = Vec::new();
    let mut index = 0;
    while index < items.len() {
        let Some(first) = items[index].as_int().map(|first| first.max(0) as u32) else {
            index += 1;
            continue;
        };
        match store.resolve(items.get(index + 1).unwrap_or(&Object::Null))? {
            Object::Array(list) => {
                for (offset, width) in list.iter().enumerate() {
                    let cid = first + offset as u32;
                    widths.push((cid, cid, width.as_number().unwrap_or(0.0) / 1000.0));
                }
                index += 2;
            }
            Object::Int(last) => {
                let width = items.get(index + 2).and_then(Object::as_number).unwrap_or(0.0);
                widths.push((first, last.max(0) as u32, width / 1000.0));
                index += 3;
            }
            _ => index += 1,
        }
    }
    Ok(widths)
}
