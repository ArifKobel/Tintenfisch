use std::collections::{BTreeMap, BTreeSet};

use crate::error::{Error, Result};

const KEPT_TABLES: [&[u8; 4]; 12] =
    [b"OS/2", b"cmap", b"cvt ", b"fpgm", b"glyf", b"head", b"hhea", b"hmtx", b"loca", b"maxp", b"name", b"prep"];
const MORE_COMPONENTS: u16 = 0x20;
const ARGUMENTS_ARE_WORDS: u16 = 0x01;
const HAS_SCALE: u16 = 0x08;
const HAS_XY_SCALE: u16 = 0x40;
const HAS_TWO_BY_TWO: u16 = 0x80;

fn u16_at(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*data.get(offset)?, *data.get(offset + 1)?]))
}

fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *data.get(offset)?,
        *data.get(offset + 1)?,
        *data.get(offset + 2)?,
        *data.get(offset + 3)?,
    ]))
}

pub fn subset(font: &[u8], glyphs: &BTreeSet<u16>) -> Result<Vec<u8>> {
    let tables = read_tables(font)?;
    let table = |tag: &[u8; 4]| {
        tables.get(tag).copied().ok_or_else(|| Error::new(format!("font without {}", String::from_utf8_lossy(tag))))
    };
    let head = table(b"head")?;
    let long_offsets = u16_at(head, 50).ok_or_else(|| Error::new("short head table"))? == 1;
    let glyph_count = u16_at(table(b"maxp")?, 4).ok_or_else(|| Error::new("short maxp table"))? as usize;
    let loca = table(b"loca")?;
    let glyf = table(b"glyf")?;
    let offset = |glyph: usize| -> usize {
        if long_offsets {
            u32_at(loca, glyph * 4).unwrap_or(0) as usize
        } else {
            u16_at(loca, glyph * 2).unwrap_or(0) as usize * 2
        }
    };
    let outline = |glyph: usize| glyf.get(offset(glyph)..offset(glyph + 1).max(offset(glyph))).unwrap_or_default();

    let mut kept: BTreeSet<u16> = glyphs.iter().copied().filter(|&glyph| (glyph as usize) < glyph_count).collect();
    kept.insert(0);
    let mut pending: Vec<u16> = kept.iter().copied().collect();
    while let Some(glyph) = pending.pop() {
        for component in components(outline(glyph as usize)) {
            if (component as usize) < glyph_count && kept.insert(component) {
                pending.push(component);
            }
        }
    }

    let mut new_glyf = Vec::new();
    let mut new_loca = Vec::with_capacity((glyph_count + 1) * 4);
    for glyph in 0..glyph_count {
        new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
        if kept.contains(&(glyph as u16)) {
            new_glyf.extend_from_slice(outline(glyph));
            while new_glyf.len() % 4 != 0 {
                new_glyf.push(0);
            }
        }
    }
    new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
    let mut new_head = head.to_vec();
    new_head[8..12].copy_from_slice(&[0; 4]);
    new_head[50..52].copy_from_slice(&1u16.to_be_bytes());

    let mut output: BTreeMap<[u8; 4], Vec<u8>> = BTreeMap::new();
    for tag in KEPT_TABLES {
        let data = match tag {
            b"glyf" => new_glyf.clone(),
            b"loca" => new_loca.clone(),
            b"head" => new_head.clone(),
            _ => match tables.get(tag) {
                Some(data) => data.to_vec(),
                None => continue,
            },
        };
        output.insert(*tag, data);
    }
    let mut file = write_tables(&output);
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&file));
    if let Some(head_offset) = head_offset(&file) {
        file[head_offset + 8..head_offset + 12].copy_from_slice(&adjustment.to_be_bytes());
    }
    Ok(file)
}

fn read_tables(font: &[u8]) -> Result<BTreeMap<[u8; 4], &[u8]>> {
    let count = u16_at(font, 4).ok_or_else(|| Error::new("not a TrueType font"))? as usize;
    let mut tables = BTreeMap::new();
    for index in 0..count {
        let record = 12 + index * 16;
        let tag: [u8; 4] = font
            .get(record..record + 4)
            .and_then(|tag| tag.try_into().ok())
            .ok_or_else(|| Error::new("truncated table directory"))?;
        let offset = u32_at(font, record + 8).unwrap_or(0) as usize;
        let length = u32_at(font, record + 12).unwrap_or(0) as usize;
        let data = font.get(offset..offset + length).ok_or_else(|| Error::new("table outside the font"))?;
        tables.insert(tag, data);
    }
    Ok(tables)
}

fn components(outline: &[u8]) -> Vec<u16> {
    let is_composite = outline.len() >= 10 && (u16_at(outline, 0).unwrap_or(0) as i16) < 0;
    if !is_composite {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut position = 10;
    while let (Some(flags), Some(glyph)) = (u16_at(outline, position), u16_at(outline, position + 2)) {
        found.push(glyph);
        position += 4 + if flags & ARGUMENTS_ARE_WORDS != 0 { 4 } else { 2 };
        position += if flags & HAS_TWO_BY_TWO != 0 {
            8
        } else if flags & HAS_XY_SCALE != 0 {
            4
        } else if flags & HAS_SCALE != 0 {
            2
        } else {
            0
        };
        if flags & MORE_COMPONENTS == 0 {
            break;
        }
    }
    found
}

fn checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn write_tables(tables: &BTreeMap<[u8; 4], Vec<u8>>) -> Vec<u8> {
    let count = tables.len() as u16;
    let power = 1u16 << (15 - count.leading_zeros().min(15));
    let search_range = power * 16;
    let mut file = Vec::new();
    file.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    file.extend_from_slice(&count.to_be_bytes());
    file.extend_from_slice(&search_range.to_be_bytes());
    file.extend_from_slice(&(power.trailing_zeros() as u16).to_be_bytes());
    file.extend_from_slice(&(count * 16 - search_range).to_be_bytes());
    let mut offset = 12 + tables.len() * 16;
    let mut bodies = Vec::new();
    for (tag, data) in tables {
        file.extend_from_slice(tag);
        file.extend_from_slice(&checksum(data).to_be_bytes());
        file.extend_from_slice(&(offset as u32).to_be_bytes());
        file.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut padded = data.clone();
        while padded.len() % 4 != 0 {
            padded.push(0);
        }
        offset += padded.len();
        bodies.extend(padded);
    }
    file.extend(bodies);
    file
}

fn head_offset(file: &[u8]) -> Option<usize> {
    let count = u16_at(file, 4)? as usize;
    (0..count)
        .map(|index| 12 + index * 16)
        .find(|&record| &file[record..record + 4] == b"head")
        .and_then(|record| u32_at(file, record + 8))
        .map(|offset| offset as usize)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ttf_parser::{Face, GlyphId};

    use super::*;

    fn liberation() -> Vec<u8> {
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../public/fonts/LiberationSans-Regular.ttf"))
            .unwrap()
    }

    #[test]
    fn keeps_the_requested_outlines_under_their_original_ids() {
        let font = liberation();
        let original = Face::parse(&font, 0).unwrap();
        let wanted = original.glyph_index('Ω').unwrap();
        let other = original.glyph_index('B').unwrap();
        let subset = subset(&font, &BTreeSet::from([wanted.0])).unwrap();
        assert!(subset.len() * 5 < font.len());
        let face = Face::parse(&subset, 0).unwrap();
        assert_eq!(face.number_of_glyphs(), original.number_of_glyphs());
        assert_eq!(face.glyph_bounding_box(wanted), original.glyph_bounding_box(wanted));
        assert_eq!(face.glyph_bounding_box(other), None);
        assert_eq!(face.glyph_hor_advance(GlyphId(other.0)), original.glyph_hor_advance(other));
        assert_eq!(checksum(&subset), 0xB1B0_AFBA);
    }

    #[test]
    fn keeps_the_components_of_composite_glyphs() {
        let font = liberation();
        let original = Face::parse(&font, 0).unwrap();
        let accented = original.glyph_index('Å').unwrap();
        let subset = subset(&font, &BTreeSet::from([accented.0])).unwrap();
        let face = Face::parse(&subset, 0).unwrap();
        assert_eq!(face.glyph_bounding_box(accented), original.glyph_bounding_box(accented));
    }
}
