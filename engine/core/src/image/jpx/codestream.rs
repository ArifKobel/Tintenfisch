use std::collections::HashMap;

use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComponentSize {
    pub depth: u8,
    pub signed: bool,
    pub step_x: u32,
    pub step_y: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageSize {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub tile_x0: u32,
    pub tile_y0: u32,
    pub components: Vec<ComponentSize>,
}

impl ImageSize {
    pub fn tiles_across(&self) -> u32 {
        (self.x1 - self.tile_x0).div_ceil(self.tile_width)
    }

    pub fn tile_area(&self, index: u32) -> (u32, u32, u32, u32) {
        let (column, row) = (index % self.tiles_across(), index / self.tiles_across());
        (
            (self.tile_x0 + column * self.tile_width).max(self.x0),
            (self.tile_y0 + row * self.tile_height).max(self.y0),
            (self.tile_x0 + (column + 1) * self.tile_width).min(self.x1),
            (self.tile_y0 + (row + 1) * self.tile_height).min(self.y1),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentCoding {
    pub levels: u8,
    pub block_width: u8,
    pub block_height: u8,
    pub block_style: u8,
    pub reversible: bool,
    pub precincts: Vec<(u8, u8)>,
}

impl ComponentCoding {
    pub fn precinct(&self, resolution: usize) -> (u8, u8) {
        self.precincts.get(resolution).copied().unwrap_or((15, 15))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progression {
    pub order: u8,
    pub layers: u16,
    pub multiple_component_transform: bool,
    pub start_of_packet: bool,
    pub end_of_packet_header: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quantization {
    pub style: u8,
    pub guard_bits: u8,
    pub steps: Vec<(u8, u16)>,
}

impl Quantization {
    pub fn step(&self, band: usize) -> (u8, u16) {
        if self.style == 1 {
            let (exponent, mantissa) = self.steps.first().copied().unwrap_or((0, 0));
            let level = if band == 0 { 0 } else { (band - 1) / 3 };
            return ((exponent as i32 - level as i32).max(0) as u8, mantissa);
        }
        self.steps.get(band).copied().unwrap_or((0, 0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionChange {
    pub resolution_start: u8,
    pub component_start: u16,
    pub layer_end: u16,
    pub resolution_end: u8,
    pub component_end: u16,
    pub order: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tile {
    pub index: u32,
    pub progression: Progression,
    pub coding: Vec<ComponentCoding>,
    pub quantization: Vec<Quantization>,
    pub region_shift: Vec<u8>,
    pub changes: Vec<ProgressionChange>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Codestream {
    pub size: ImageSize,
    pub tiles: Vec<Tile>,
}

struct Reader<'d> {
    data: &'d [u8],
    position: usize,
}

impl<'d> Reader<'d> {
    fn byte(&mut self) -> Result<u8> {
        let value = *self.data.get(self.position).ok_or_else(|| Error::new("the JPEG 2000 codestream is cut short"))?;
        self.position += 1;
        Ok(value)
    }

    fn short(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes([self.byte()?, self.byte()?]))
    }

    fn long(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes([self.byte()?, self.byte()?, self.byte()?, self.byte()?]))
    }
}

#[derive(Default)]
struct HeaderChanges {
    progression: Option<Progression>,
    default_coding: Option<ComponentCoding>,
    component_coding: HashMap<usize, ComponentCoding>,
    default_quantization: Option<Quantization>,
    component_quantization: HashMap<usize, Quantization>,
    region_shift: HashMap<usize, u8>,
    changes: Vec<ProgressionChange>,
}

impl HeaderChanges {
    fn apply(
        &self,
        progression: &mut Progression,
        coding: &mut [ComponentCoding],
        quantization: &mut [Quantization],
        shift: &mut [u8],
    ) {
        if let Some(own) = self.progression {
            *progression = own;
        }
        for (component, slot) in coding.iter_mut().enumerate() {
            if let Some(own) = self.component_coding.get(&component).or(self.default_coding.as_ref()) {
                *slot = own.clone();
            }
        }
        for (component, slot) in quantization.iter_mut().enumerate() {
            if let Some(own) = self.component_quantization.get(&component).or(self.default_quantization.as_ref()) {
                *slot = own.clone();
            }
        }
        for (component, slot) in shift.iter_mut().enumerate() {
            if let Some(own) = self.region_shift.get(&component) {
                *slot = *own;
            }
        }
    }
}

impl Codestream {
    pub fn parse(data: &[u8]) -> Result<Codestream> {
        let mut reader = Reader { data, position: 0 };
        if reader.short()? != 0xff4f {
            return Err(Error::new("a JPEG 2000 codestream starts with SOC"));
        }
        if reader.short()? != 0xff51 {
            return Err(Error::new("a JPEG 2000 codestream has SIZ right after SOC"));
        }
        let size = image_size(&mut reader)?;
        let count = size.components.len();
        let mut main = HeaderChanges::default();
        let marker = loop {
            let marker = reader.short()?;
            if marker == 0xff90 || marker == 0xffd9 {
                break marker;
            }
            header_marker(&mut reader, marker, count, &mut main)?;
        };
        let mut progression = Progression {
            order: 0,
            layers: 1,
            multiple_component_transform: false,
            start_of_packet: false,
            end_of_packet_header: false,
        };
        let default_coding = ComponentCoding {
            levels: 5,
            block_width: 6,
            block_height: 6,
            block_style: 0,
            reversible: true,
            precincts: Vec::new(),
        };
        let mut coding = vec![default_coding; count];
        let mut quantization = vec![Quantization { style: 0, guard_bits: 2, steps: Vec::new() }; count];
        let mut shift = vec![0u8; count];
        main.apply(&mut progression, &mut coding, &mut quantization, &mut shift);
        let mut tiles: HashMap<u32, Tile> = HashMap::new();
        let mut marker = marker;
        while marker == 0xff90 {
            let start = reader.position - 2;
            reader.short()?;
            let index = reader.short()? as u32;
            let length = reader.long()? as usize;
            let part = reader.byte()?;
            reader.byte()?;
            let mut changes = HeaderChanges::default();
            loop {
                let next = reader.short()?;
                if next == 0xff93 {
                    break;
                }
                header_marker(&mut reader, next, count, &mut changes)?;
            }
            let end = if length == 0 {
                data.len().saturating_sub(2).max(reader.position)
            } else {
                (start + length).min(data.len())
            };
            let body = data.get(reader.position..end).unwrap_or_default();
            let tile = tiles.entry(index).or_insert_with(|| Tile {
                index,
                progression,
                coding: coding.clone(),
                quantization: quantization.clone(),
                region_shift: shift.clone(),
                changes: main.changes.clone(),
                data: Vec::new(),
            });
            if part == 0 {
                changes.apply(&mut tile.progression, &mut tile.coding, &mut tile.quantization, &mut tile.region_shift);
                if !changes.changes.is_empty() {
                    tile.changes = changes.changes.clone();
                }
            } else {
                tile.changes.extend(changes.changes.iter().copied());
            }
            tile.data.extend_from_slice(body);
            reader.position = end;
            marker = if reader.position + 2 <= data.len() { reader.short()? } else { 0xffd9 };
        }
        let mut tiles: Vec<Tile> = tiles.into_values().collect();
        tiles.sort_by_key(|tile| tile.index);
        Ok(Codestream { size, tiles })
    }
}

fn image_size(reader: &mut Reader) -> Result<ImageSize> {
    reader.short()?;
    reader.short()?;
    let (x1, y1, x0, y0) = (reader.long()?, reader.long()?, reader.long()?, reader.long()?);
    let (tile_width, tile_height, tile_x0, tile_y0) = (reader.long()?, reader.long()?, reader.long()?, reader.long()?);
    let count = reader.short()? as usize;
    let components = (0..count)
        .map(|_| {
            let depth = reader.byte()?;
            Ok(ComponentSize {
                depth: (depth & 0x7f) + 1,
                signed: depth & 0x80 != 0,
                step_x: reader.byte()? as u32,
                step_y: reader.byte()? as u32,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if x1 <= x0
        || y1 <= y0
        || tile_width == 0
        || tile_height == 0
        || components.is_empty()
        || components.iter().any(|component| component.step_x == 0 || component.step_y == 0)
    {
        return Err(Error::new("the JPEG 2000 image size is not usable"));
    }
    Ok(ImageSize { x0, y0, x1, y1, tile_width, tile_height, tile_x0, tile_y0, components })
}

fn component_index(reader: &mut Reader, count: usize) -> Result<usize> {
    Ok(if count < 257 { reader.byte()? as usize } else { reader.short()? as usize })
}

fn coding_parameters(reader: &mut Reader, precincts_defined: bool) -> Result<ComponentCoding> {
    let levels = reader.byte()?;
    let block_width = reader.byte()? + 2;
    let block_height = reader.byte()? + 2;
    let block_style = reader.byte()?;
    let reversible = reader.byte()? == 1;
    let precincts = if precincts_defined {
        (0..=levels).map(|_| reader.byte().map(|size| (size & 0x0f, size >> 4))).collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    Ok(ComponentCoding { levels, block_width, block_height, block_style, reversible, precincts })
}

fn quantization(reader: &mut Reader, length: usize) -> Result<Quantization> {
    let style_byte = reader.byte()?;
    let style = style_byte & 0x1f;
    let guard_bits = style_byte >> 5;
    let steps = if style == 0 {
        (0..length).map(|_| reader.byte().map(|value| (value >> 3, 0))).collect::<Result<Vec<_>>>()?
    } else {
        (0..length / 2)
            .map(|_| reader.short().map(|value| ((value >> 11) as u8, value & 0x7ff)))
            .collect::<Result<Vec<_>>>()?
    };
    Ok(Quantization { style, guard_bits, steps })
}

fn header_marker(reader: &mut Reader, marker: u16, count: usize, changes: &mut HeaderChanges) -> Result<()> {
    let length = reader.short()? as usize;
    let end = reader.position + length - 2;
    match marker {
        0xff52 => {
            let style = reader.byte()?;
            let order = reader.byte()?;
            let layers = reader.short()?;
            let multiple_component_transform = reader.byte()? != 0;
            changes.progression = Some(Progression {
                order,
                layers,
                multiple_component_transform,
                start_of_packet: style & 2 != 0,
                end_of_packet_header: style & 4 != 0,
            });
            changes.default_coding = Some(coding_parameters(reader, style & 1 != 0)?);
        }
        0xff53 => {
            let component = component_index(reader, count)?;
            let style = reader.byte()?;
            changes.component_coding.insert(component, coding_parameters(reader, style & 1 != 0)?);
        }
        0xff5c => {
            changes.default_quantization = Some(quantization(reader, end - reader.position - 1)?);
        }
        0xff5d => {
            let component = component_index(reader, count)?;
            changes.component_quantization.insert(component, quantization(reader, end - reader.position - 1)?);
        }
        0xff5e => {
            let component = component_index(reader, count)?;
            reader.byte()?;
            changes.region_shift.insert(component, reader.byte()?);
        }
        0xff5f => {
            let wide = count >= 257;
            let entry = if wide { 9 } else { 7 };
            for _ in 0..(end - reader.position) / entry {
                let resolution_start = reader.byte()?;
                let component_start = if wide { reader.short()? } else { reader.byte()? as u16 };
                let layer_end = reader.short()?;
                let resolution_end = reader.byte()?;
                let component_end = if wide { reader.short()? } else { reader.byte()? as u16 };
                let order = reader.byte()?;
                changes.changes.push(ProgressionChange {
                    resolution_start,
                    component_start,
                    layer_end,
                    resolution_end,
                    component_end,
                    order,
                });
            }
        }
        0xff60 | 0xff61 => return Err(Error::unsupported("JPEG 2000 packed packet headers are not supported yet")),
        _ => {}
    }
    reader.position = end;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_quantization_lowers_the_exponent_by_one_per_level_above_the_first() {
        let quantization = Quantization { style: 1, guard_bits: 2, steps: vec![(10, 300)] };
        let exponents: Vec<u8> = (0..10).map(|band| quantization.step(band).0).collect();
        assert_eq!(exponents, [10, 10, 10, 10, 9, 9, 9, 8, 8, 8]);
        assert!((0..10).all(|band| quantization.step(band).1 == 300));
        let expounded = Quantization { style: 2, guard_bits: 2, steps: vec![(10, 1), (9, 2), (9, 3), (8, 4)] };
        assert_eq!(expounded.step(2), (9, 3));
    }
}
