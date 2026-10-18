use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JpegHeader {
    pub width: usize,
    pub height: usize,
    pub components: usize,
    pub adobe: bool,
    pub orientation: u16,
}

impl JpegHeader {
    pub fn read(data: &[u8]) -> Result<JpegHeader> {
        if !data.starts_with(&[0xFF, 0xD8]) {
            return Err(Error::new("not a JPEG file"));
        }
        let mut header = JpegHeader { width: 0, height: 0, components: 0, adobe: false, orientation: 1 };
        let mut at = 2;
        while at + 4 <= data.len() {
            if data[at] != 0xFF {
                return Err(Error::new("JPEG markers are broken"));
            }
            let marker = data[at + 1];
            if marker == 0xFF {
                at += 1;
                continue;
            }
            let length = u16::from_be_bytes([data[at + 2], data[at + 3]]) as usize;
            let segment =
                data.get(at + 4..at + 2 + length).ok_or_else(|| Error::new("JPEG segment runs past the file"))?;
            match marker {
                0xC0..=0xCF if ![0xC4, 0xC8, 0xCC].contains(&marker) => {
                    let [_, h0, h1, w0, w1, components, ..] = *segment else {
                        return Err(Error::new("JPEG frame header is too short"));
                    };
                    header.height = u16::from_be_bytes([h0, h1]) as usize;
                    header.width = u16::from_be_bytes([w0, w1]) as usize;
                    header.components = components as usize;
                }
                0xE1 => {
                    if let Some(orientation) = segment.strip_prefix(b"Exif\0\0").and_then(exif_orientation) {
                        header.orientation = orientation;
                    }
                }
                0xEE if segment.starts_with(b"Adobe") => header.adobe = true,
                0xDA => break,
                _ => {}
            }
            at += 2 + length;
        }
        if header.width == 0 || header.height == 0 || ![1, 3, 4].contains(&header.components) {
            return Err(Error::unsupported("a JPEG without a usable frame header"));
        }
        Ok(header)
    }
}

fn exif_orientation(tiff: &[u8]) -> Option<u16> {
    let little = match tiff.get(..4)? {
        b"II*\0" => true,
        b"MM\0*" => false,
        _ => return None,
    };
    let read16 = |at: usize| {
        tiff.get(at..at + 2).map(|bytes| {
            if little { u16::from_le_bytes([bytes[0], bytes[1]]) } else { u16::from_be_bytes([bytes[0], bytes[1]]) }
        })
    };
    let read32 = |at: usize| {
        tiff.get(at..at + 4).map(|bytes| {
            let bytes = [bytes[0], bytes[1], bytes[2], bytes[3]];
            if little { u32::from_le_bytes(bytes) } else { u32::from_be_bytes(bytes) }
        })
    };
    let directory = read32(4)? as usize;
    let entries = read16(directory)? as usize;
    (0..entries)
        .map(|index| directory + 2 + index * 12)
        .find(|&entry| read16(entry) == Some(0x0112))
        .and_then(|entry| read16(entry + 8))
        .filter(|orientation| (1..=8).contains(orientation))
}
