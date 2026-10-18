use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Jp2Color {
    Rgb,
    Gray,
    Ycc,
    Profile,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    pub entries: Vec<Vec<u32>>,
    pub depths: Vec<u8>,
    pub mapping: Vec<(usize, Option<usize>)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Jp2File<'d> {
    pub codestream: &'d [u8],
    pub color: Option<Jp2Color>,
    pub palette: Option<Palette>,
    pub opacity: Option<usize>,
}

impl<'d> Jp2File<'d> {
    pub fn read(data: &'d [u8]) -> Result<Jp2File<'d>> {
        if data.starts_with(&[0xff, 0x4f]) {
            return Ok(Jp2File { codestream: data, ..Jp2File::default() });
        }
        let mut file = Jp2File::default();
        let mut palette_entries: Option<(Vec<Vec<u32>>, Vec<u8>)> = None;
        let mut mapping = Vec::new();
        for (kind, content) in boxes(data)? {
            match &kind {
                b"jp2h" => {
                    for (kind, content) in boxes(content)? {
                        match &kind {
                            b"colr" if file.color.is_none() => file.color = Some(color(content)),
                            b"pclr" => palette_entries = Some(palette(content)?),
                            b"cmap" => {
                                mapping = content
                                    .as_chunks::<4>()
                                    .0
                                    .iter()
                                    .map(|[high, low, kind, column]| {
                                        (
                                            u16::from_be_bytes([*high, *low]) as usize,
                                            (*kind == 1).then_some(*column as usize),
                                        )
                                    })
                                    .collect();
                            }
                            b"cdef" => {
                                file.opacity = content
                                    .get(2..)
                                    .unwrap_or_default()
                                    .as_chunks::<6>()
                                    .0
                                    .iter()
                                    .find(|entry| u16::from_be_bytes([entry[2], entry[3]]) == 1)
                                    .map(|entry| u16::from_be_bytes([entry[0], entry[1]]) as usize);
                            }
                            _ => {}
                        }
                    }
                }
                b"jp2c" => file.codestream = content,
                _ => {}
            }
        }
        if let Some((entries, depths)) = palette_entries {
            file.palette = Some(Palette { entries, depths, mapping });
        }
        if file.codestream.is_empty() {
            return Err(Error::new("the JPEG 2000 file has no codestream"));
        }
        Ok(file)
    }
}

fn boxes(mut data: &[u8]) -> Result<Vec<([u8; 4], &[u8])>> {
    let mut found = Vec::new();
    while data.len() >= 8 {
        let length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as u64;
        let kind = [data[4], data[5], data[6], data[7]];
        let (header, length) = match length {
            0 => (8, data.len() as u64),
            1 => {
                let wide = data.get(8..16).ok_or_else(|| Error::new("a JP2 box is cut short"))?;
                (16, u64::from_be_bytes(wide.try_into().unwrap_or_default()))
            }
            length => (8, length),
        };
        let end = (length as usize).min(data.len());
        if end < header {
            return Err(Error::new("a JP2 box is shorter than its header"));
        }
        found.push((kind, &data[header..end]));
        data = &data[end..];
    }
    Ok(found)
}

fn color(content: &[u8]) -> Jp2Color {
    match (content.first(), content.get(3..7)) {
        (Some(1), Some([0, 0, 0, 16])) => Jp2Color::Rgb,
        (Some(1), Some([0, 0, 0, 17])) => Jp2Color::Gray,
        (Some(1), Some([0, 0, 0, 18])) => Jp2Color::Ycc,
        _ => Jp2Color::Profile,
    }
}

fn palette(content: &[u8]) -> Result<(Vec<Vec<u32>>, Vec<u8>)> {
    let cut = || Error::new("a JP2 palette is cut short");
    let count = u16::from_be_bytes([*content.first().ok_or_else(cut)?, *content.get(1).ok_or_else(cut)?]) as usize;
    let columns = *content.get(2).ok_or_else(cut)? as usize;
    let depths: Vec<u8> = content.get(3..3 + columns).ok_or_else(cut)?.iter().map(|depth| (depth & 0x7f) + 1).collect();
    let mut position = 3 + columns;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let mut entry = Vec::with_capacity(columns);
        for depth in &depths {
            let bytes = (*depth as usize).div_ceil(8);
            let value = content
                .get(position..position + bytes)
                .ok_or_else(cut)?
                .iter()
                .fold(0u32, |value, byte| value << 8 | *byte as u32);
            position += bytes;
            entry.push(value);
        }
        entries.push(entry);
    }
    Ok((entries, depths))
}
