use crate::image::jpx::code_block::{CodeBlock, CodeBlockData, Orientation, segment_capacity};
use crate::image::jpx::codestream::{ComponentSize, ImageSize, ProgressionChange, Tile};
use crate::image::jpx::packet_bits::PacketBits;
use crate::image::jpx::tag_tree::TagTree;
use crate::image::jpx::wavelet::Wavelet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Area {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

impl Area {
    fn width(&self) -> usize {
        (self.x1 - self.x0) as usize
    }

    fn height(&self) -> usize {
        (self.y1 - self.y0) as usize
    }

    fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    fn reduced(&self, shift: u32) -> Area {
        let scale = 1u64 << shift;
        let up = |value: u32| (value as u64).div_ceil(scale) as u32;
        Area { x0: up(self.x0), y0: up(self.y0), x1: up(self.x1), y1: up(self.y1) }
    }
}

struct Piece {
    passes: u32,
    length: usize,
    continues: bool,
}

struct Block {
    area: Area,
    data: CodeBlockData,
    passes: u32,
    length_bits: u32,
}

struct Precinct {
    blocks: Vec<usize>,
    across: usize,
    inclusion: TagTree,
    planes: TagTree,
}

struct Subband {
    orientation: Orientation,
    area: Area,
    band: usize,
    blocks: Vec<Block>,
    precincts: Vec<Precinct>,
}

struct Resolution {
    area: Area,
    precinct_exponents: (u8, u8),
    precincts_across: u32,
    precincts_down: u32,
    subbands: Vec<Subband>,
}

struct TileComponent {
    area: Area,
    levels: u8,
    resolutions: Vec<Resolution>,
}

pub struct DecodedTile {
    pub components: Vec<(u32, u32, usize, usize, Vec<f64>)>,
}

pub struct TileDecoder<'t> {
    size: &'t ImageSize,
    tile: &'t Tile,
}

impl<'t> TileDecoder<'t> {
    pub fn new(size: &'t ImageSize, tile: &'t Tile) -> Self {
        Self { size, tile }
    }

    pub fn decode(&self) -> DecodedTile {
        let (x0, y0, x1, y1) = self.size.tile_area(self.tile.index);
        let tile_area = Area { x0, y0, x1, y1 };
        let mut components: Vec<TileComponent> = self
            .size
            .components
            .iter()
            .enumerate()
            .map(|(index, component)| self.layout(index, component, tile_area))
            .collect();
        self.read_packets(&mut components, tile_area);
        let decoded = components.iter().enumerate().map(|(index, component)| self.samples(index, component)).collect();
        DecodedTile { components: decoded }
    }

    fn layout(&self, index: usize, component: &ComponentSize, tile: Area) -> TileComponent {
        let coding = &self.tile.coding[index];
        let area = Area {
            x0: tile.x0.div_ceil(component.step_x),
            y0: tile.y0.div_ceil(component.step_y),
            x1: tile.x1.div_ceil(component.step_x),
            y1: tile.y1.div_ceil(component.step_y),
        };
        let levels = coding.levels;
        let resolutions = (0..=levels as usize)
            .map(|resolution| {
                let reduction = levels as u32 - resolution as u32;
                let resolution_area = area.reduced(reduction);
                let (px, py) = coding.precinct(resolution);
                let span = |low: u32, high: u32, exponent: u8| {
                    if high <= low { 0 } else { high.div_ceil(1 << exponent) - (low >> exponent) }
                };
                let precincts_across = span(resolution_area.x0, resolution_area.x1, px);
                let precincts_down = span(resolution_area.y0, resolution_area.y1, py);
                let orientations: &[(Orientation, u32, u32)] = if resolution == 0 {
                    &[(Orientation::Ll, 0, 0)]
                } else {
                    &[(Orientation::Hl, 1, 0), (Orientation::Lh, 0, 1), (Orientation::Hh, 1, 1)]
                };
                let subbands = orientations
                    .iter()
                    .enumerate()
                    .map(|(position, (orientation, offset_x, offset_y))| {
                        let band_area = if resolution == 0 {
                            area.reduced(levels as u32)
                        } else {
                            let shift = levels as u32 - resolution as u32 + 1;
                            let half = 1u32 << (shift - 1);
                            let up = |value: u32, offset: u32| {
                                let shifted = value as i64 - (half * offset) as i64;
                                if shifted <= 0 { 0 } else { (shifted as u64).div_ceil(1u64 << shift) as u32 }
                            };
                            Area {
                                x0: up(area.x0, *offset_x),
                                y0: up(area.y0, *offset_y),
                                x1: up(area.x1, *offset_x),
                                y1: up(area.y1, *offset_y),
                            }
                        };
                        let band = if resolution == 0 { 0 } else { 1 + (resolution - 1) * 3 + position };
                        self.subband(
                            *orientation,
                            band_area,
                            band,
                            resolution,
                            coding.block_width,
                            coding.block_height,
                            (px, py),
                            (precincts_across, precincts_down),
                            resolution_area,
                        )
                    })
                    .collect();
                Resolution {
                    area: resolution_area,
                    precinct_exponents: (px, py),
                    precincts_across,
                    precincts_down,
                    subbands,
                }
            })
            .collect();
        TileComponent { area, levels, resolutions }
    }

    #[allow(clippy::too_many_arguments)]
    fn subband(
        &self,
        orientation: Orientation,
        area: Area,
        band: usize,
        resolution: usize,
        block_width: u8,
        block_height: u8,
        (px, py): (u8, u8),
        (across, down): (u32, u32),
        resolution_area: Area,
    ) -> Subband {
        let lower = if resolution == 0 { 0 } else { 1 };
        let block_x = block_width.min(px.saturating_sub(lower)) as u32;
        let block_y = block_height.min(py.saturating_sub(lower)) as u32;
        let precinct_x = px.saturating_sub(lower) as u32;
        let precinct_y = py.saturating_sub(lower) as u32;
        let first_precinct_x = resolution_area.x0 >> px;
        let first_precinct_y = resolution_area.y0 >> py;
        let mut blocks = Vec::new();
        let mut precincts = Vec::new();
        for row in 0..down {
            for column in 0..across {
                let precinct = Area {
                    x0: ((first_precinct_x + column) << precinct_x).max(area.x0),
                    y0: ((first_precinct_y + row) << precinct_y).max(area.y0),
                    x1: ((first_precinct_x + column + 1) << precinct_x).min(area.x1),
                    y1: ((first_precinct_y + row + 1) << precinct_y).min(area.y1),
                };
                let (first_column, first_row) = (precinct.x0 >> block_x, precinct.y0 >> block_y);
                let (blocks_across, blocks_down) = if precinct.is_empty() {
                    (0, 0)
                } else {
                    (precinct.x1.div_ceil(1 << block_x) - first_column, precinct.y1.div_ceil(1 << block_y) - first_row)
                };
                let mut members = Vec::new();
                for block_row in 0..blocks_down {
                    for block_column in 0..blocks_across {
                        let (column_index, row_index) = (first_column + block_column, first_row + block_row);
                        let block_area = Area {
                            x0: (column_index << block_x).max(precinct.x0),
                            y0: (row_index << block_y).max(precinct.y0),
                            x1: ((column_index + 1) << block_x).min(precinct.x1),
                            y1: ((row_index + 1) << block_y).min(precinct.y1),
                        };
                        members.push(blocks.len());
                        blocks.push(Block {
                            area: block_area,
                            data: CodeBlockData::default(),
                            passes: 0,
                            length_bits: 3,
                        });
                    }
                }
                precincts.push(Precinct {
                    blocks: members,
                    across: blocks_across as usize,
                    inclusion: TagTree::new(blocks_across.max(1) as usize, blocks_down.max(1) as usize),
                    planes: TagTree::new(blocks_across.max(1) as usize, blocks_down.max(1) as usize),
                });
            }
        }
        Subband { orientation, area, band, blocks, precincts }
    }

    fn packet_order(&self, components: &[TileComponent], tile: Area) -> Vec<(u16, usize, usize, usize)> {
        let progression = self.tile.progression;
        let layers = progression.layers;
        let most_resolutions = components.iter().map(|component| component.resolutions.len()).max().unwrap_or(0);
        let changes: Vec<ProgressionChange> = if self.tile.changes.is_empty() {
            vec![ProgressionChange {
                resolution_start: 0,
                component_start: 0,
                layer_end: layers,
                resolution_end: most_resolutions as u8,
                component_end: components.len() as u16,
                order: progression.order,
            }]
        } else {
            self.tile.changes.clone()
        };
        let mut next_layer: Vec<Vec<Vec<u16>>> = components
            .iter()
            .map(|component| {
                component
                    .resolutions
                    .iter()
                    .map(|resolution| vec![0; (resolution.precincts_across * resolution.precincts_down) as usize])
                    .collect()
            })
            .collect();
        let mut order = Vec::new();
        for change in changes {
            let mut candidates = Vec::new();
            for (component_index, component) in components.iter().enumerate() {
                if component_index < change.component_start as usize || component_index >= change.component_end as usize
                {
                    continue;
                }
                let size = &self.size.components[component_index];
                for (resolution_index, resolution) in component.resolutions.iter().enumerate() {
                    if resolution_index < change.resolution_start as usize
                        || resolution_index >= change.resolution_end as usize
                    {
                        continue;
                    }
                    let reduction = component.levels as u32 - resolution_index as u32;
                    let (px, py) = resolution.precinct_exponents;
                    for precinct in 0..(resolution.precincts_across * resolution.precincts_down) as usize {
                        let (column, row) = (
                            precinct as u32 % resolution.precincts_across,
                            precinct as u32 / resolution.precincts_across,
                        );
                        let absolute_x = (resolution.area.x0 >> px) + column;
                        let absolute_y = (resolution.area.y0 >> py) + row;
                        let x = ((absolute_x as u64) << (px as u32 + reduction)) * size.step_x as u64;
                        let y = ((absolute_y as u64) << (py as u32 + reduction)) * size.step_y as u64;
                        let (x, y) = (x.max(tile.x0 as u64), y.max(tile.y0 as u64));
                        candidates.push((component_index, resolution_index, precinct, x, y));
                    }
                }
            }
            let key = |&(component, resolution, _, x, y): &(usize, usize, usize, u64, u64)| match change.order {
                2 => (resolution as u64, y, x, component as u64),
                3 => (y, x, component as u64, resolution as u64),
                4 => (component as u64, y, x, resolution as u64),
                _ => (0, 0, 0, 0),
            };
            if change.order >= 2 {
                candidates.sort_by_key(key);
                for (component, resolution, precinct, _, _) in candidates {
                    let layer = &mut next_layer[component][resolution][precinct];
                    while *layer < change.layer_end.min(layers) {
                        order.push((*layer, resolution, component, precinct));
                        *layer += 1;
                    }
                }
            } else {
                let mut sorted: Vec<(u16, usize, usize, usize)> = Vec::new();
                for layer in 0..change.layer_end.min(layers) {
                    for &(component, resolution, precinct, _, _) in &candidates {
                        if next_layer[component][resolution][precinct] <= layer {
                            sorted.push((layer, resolution, component, precinct));
                        }
                    }
                }
                if change.order == 1 {
                    sorted.sort_by_key(|&(layer, resolution, component, precinct)| {
                        (resolution, layer, component, precinct)
                    });
                } else {
                    sorted.sort_by_key(|&(layer, resolution, component, precinct)| {
                        (layer, resolution, component, precinct)
                    });
                }
                for (layer, resolution, component, precinct) in sorted {
                    next_layer[component][resolution][precinct] = layer + 1;
                    order.push((layer, resolution, component, precinct));
                }
            }
        }
        order
    }

    fn read_packets(&self, components: &mut [TileComponent], tile: Area) {
        let data = &self.tile.data;
        let mut position = 0;
        for (layer, resolution, component, precinct) in self.packet_order(components, tile) {
            if position >= data.len() {
                break;
            }
            if self.tile.progression.start_of_packet && data.get(position..position + 2) == Some(&[0xff, 0x91]) {
                position += 6;
            }
            let mut bits = PacketBits::new(&data[position..]);
            let mut contributions: Vec<(usize, usize, Vec<Piece>)> = Vec::new();
            let subbands = &mut components[component].resolutions[resolution].subbands;
            if bits.bit() == 1 {
                for (subband_index, subband) in subbands.iter_mut().enumerate() {
                    let precinct = &mut subband.precincts[precinct];
                    for (member, &block_index) in precinct.blocks.iter().enumerate() {
                        let (x, y) = (member % precinct.across.max(1), member / precinct.across.max(1));
                        let block = &mut subband.blocks[block_index];
                        let included = if block.data.included {
                            bits.bit() == 1
                        } else {
                            precinct.inclusion.below(&mut bits, x, y, layer as u32 + 1)
                        };
                        if !included {
                            continue;
                        }
                        if !block.data.included {
                            block.data.included = true;
                            block.data.missing_planes = precinct.planes.value(&mut bits, x, y);
                        }
                        let passes = coding_passes(&mut bits);
                        while bits.bit() == 1 {
                            block.length_bits += 1;
                        }
                        let style = self.tile.coding[component].block_style;
                        let mut pieces = Vec::new();
                        let mut remaining = passes;
                        let mut held = block.data.segments.last().map_or(0, |(_, held)| *held);
                        let mut start = block.passes - held;
                        let mut first = block.passes;
                        while remaining > 0 {
                            let continues = first > 0 && held < segment_capacity(style, start);
                            if !continues {
                                start = first;
                                held = 0;
                            }
                            let chunk = (segment_capacity(style, start) - held).min(remaining);
                            let length = bits.bits(block.length_bits + chunk.ilog2());
                            pieces.push(Piece { passes: chunk, length: length as usize, continues });
                            held += chunk;
                            remaining -= chunk;
                            first += chunk;
                        }
                        contributions.push((subband_index, block_index, pieces));
                    }
                }
            }
            bits.align();
            position += bits.position();
            if self.tile.progression.end_of_packet_header && data.get(position..position + 2) == Some(&[0xff, 0x92]) {
                position += 2;
            }
            for (subband_index, block_index, pieces) in contributions {
                let block = &mut subbands[subband_index].blocks[block_index];
                for piece in pieces {
                    let end = (position + piece.length).min(data.len());
                    let bytes = &data[position.min(end)..end];
                    position = end;
                    match block.data.segments.last_mut() {
                        Some(segment) if piece.continues => {
                            segment.0.extend_from_slice(bytes);
                            segment.1 += piece.passes;
                        }
                        _ => block.data.segments.push((bytes.to_vec(), piece.passes)),
                    }
                    block.passes += piece.passes;
                }
            }
        }
    }

    fn samples(&self, index: usize, component: &TileComponent) -> (u32, u32, usize, usize, Vec<f64>) {
        let coding = &self.tile.coding[index];
        let quantization = &self.tile.quantization[index];
        let depth = self.size.components[index].depth as i32;
        let shift = self.tile.region_shift[index] as u32;
        let wavelet = if coding.reversible { Wavelet::Reversible } else { Wavelet::Irreversible };
        let mut bands: Vec<Vec<Vec<f64>>> = Vec::new();
        for resolution in &component.resolutions {
            let mut decoded = Vec::new();
            for subband in &resolution.subbands {
                let (exponent, mantissa) = quantization.step(subband.band);
                let magnitude_bits = quantization.guard_bits as u32 + exponent as u32 - 1 + shift;
                let gain = match subband.orientation {
                    Orientation::Ll => 0,
                    Orientation::Hl | Orientation::Lh => 1,
                    Orientation::Hh => 2,
                };
                let step = if quantization.style == 0 {
                    1.0
                } else {
                    2f64.powi(depth + gain - exponent as i32) * (1.0 + mantissa as f64 / 2048.0)
                };
                let mut values = vec![0.0; subband.area.width() * subband.area.height()];
                for block in &subband.blocks {
                    let (width, height) = (block.area.width(), block.area.height());
                    if width == 0 || height == 0 {
                        continue;
                    }
                    let coefficients = CodeBlock::new(width, height, subband.orientation, coding.block_style).decode(
                        &block.data,
                        magnitude_bits,
                        shift,
                    );
                    for y in 0..height {
                        for x in 0..width {
                            let value = coefficients[y * width + x];
                            let (band_x, band_y) = (
                                (block.area.x0 - subband.area.x0) as usize + x,
                                (block.area.y0 - subband.area.y0) as usize + y,
                            );
                            values[band_y * subband.area.width() + band_x] = value * step;
                        }
                    }
                }
                decoded.push(values);
            }
            bands.push(decoded);
        }
        let first = &component.resolutions[0];
        let mut current = bands[0][0].clone();
        let (mut width, mut height) = (first.area.width(), first.area.height());
        for (resolution_index, resolution) in component.resolutions.iter().enumerate().skip(1) {
            let area = resolution.area;
            let (new_width, new_height) = (area.width(), area.height());
            let mut next = vec![0.0; new_width * new_height];
            let subbands = &resolution.subbands;
            let (u0, v0) = (area.x0 as usize, area.y0 as usize);
            for y in 0..new_height {
                for x in 0..new_width {
                    let (u, v) = (u0 + x, v0 + y);
                    let (odd_x, odd_y) = (u % 2 == 1, v % 2 == 1);
                    let band_x = u / 2 - if odd_x { u0 / 2 } else { u0.div_ceil(2) };
                    let band_y = v / 2 - if odd_y { v0 / 2 } else { v0.div_ceil(2) };
                    let (values, band_width) = match (odd_x, odd_y) {
                        (false, false) => (&current, width),
                        (true, false) => (&bands[resolution_index][0], subbands[0].area.width()),
                        (false, true) => (&bands[resolution_index][1], subbands[1].area.width()),
                        (true, true) => (&bands[resolution_index][2], subbands[2].area.width()),
                    };
                    next[y * new_width + x] = values.get(band_y * band_width + band_x).copied().unwrap_or(0.0);
                }
            }
            for row in next.chunks_mut(new_width.max(1)) {
                wavelet.synthesize(row, area.x0 as usize);
            }
            let mut column = vec![0.0; new_height];
            for x in 0..new_width {
                for y in 0..new_height {
                    column[y] = next[y * new_width + x];
                }
                wavelet.synthesize(&mut column, area.y0 as usize);
                for y in 0..new_height {
                    next[y * new_width + x] = column[y];
                }
            }
            current = next;
            width = new_width;
            height = new_height;
        }
        (component.area.x0, component.area.y0, width, height, current)
    }
}

fn coding_passes(bits: &mut PacketBits) -> u32 {
    if bits.bit() == 0 {
        return 1;
    }
    if bits.bit() == 0 {
        return 2;
    }
    let two = bits.bits(2);
    if two < 3 {
        return 3 + two;
    }
    let five = bits.bits(5);
    if five < 31 {
        return 6 + five;
    }
    37 + bits.bits(7)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_counts_read_every_codeword_length_of_table_b4() {
        let cases: [(&str, u32); 8] = [
            ("0", 1),
            ("10", 2),
            ("1100", 3),
            ("1110", 5),
            ("111100000", 6),
            ("111111110", 36),
            ("1111111110000000", 37),
            ("1111111111111110", 163),
        ];
        for (code, passes) in cases {
            let mut bytes = Vec::new();
            let (mut byte, mut room) = (0u8, 8);
            for bit in code.bytes().map(|digit| digit - b'0') {
                room -= 1;
                byte |= bit << room;
                if room == 0 {
                    bytes.push(byte);
                    room = if byte == 0xff { 7 } else { 8 };
                    byte = 0;
                }
            }
            bytes.push(byte);
            assert_eq!(coding_passes(&mut PacketBits::new(&bytes)), passes, "{code}");
        }
    }
}
