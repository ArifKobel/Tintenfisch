use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::image::bit_reader::BitReader;
use crate::image::ccitt::{FaxDecoder, FaxParams};
use crate::image::jbig2::arithmetic_text::ArithmeticText;
use crate::image::jbig2::bitmap::{Bitmap, Combination};
use crate::image::jbig2::coding_state::CodingState;
use crate::image::jbig2::generic_region::GenericRegion;
use crate::image::jbig2::halftone_region::HalftoneRegion;
use crate::image::jbig2::huffman_table::HuffmanTable;
use crate::image::jbig2::huffman_text::{HuffmanText, SymbolIds, TextTables};
use crate::image::jbig2::integer_decoder::symbol_code_length;
use crate::image::jbig2::pattern_dictionary::pattern_dictionary;
use crate::image::jbig2::refinement_region::RefinementRegion;
use crate::image::jbig2::segment::{Segment, SegmentReader};
use crate::image::jbig2::symbol_dictionary::{DictionaryTables, SymbolDictionary};
use crate::image::jbig2::text_region::{Corner, TextRegion};
use crate::image::mq_decoder::MqDecoder;

const UNKNOWN: u32 = u32::MAX;
const DEFAULT_REFINEMENT_ADAPTIVE: [(i8, i8); 2] = [(-1, -1), (-1, -1)];

enum Kept {
    Symbols(Vec<Bitmap>),
    Patterns(Vec<Bitmap>),
    Region(Bitmap),
    Table(HuffmanTable),
}

struct RegionInfo {
    width: usize,
    height: usize,
    left: i64,
    top: i64,
    combination: Combination,
}

pub struct Jbig2Decoder {
    page: Option<Bitmap>,
    default_black: bool,
    height_known: bool,
    kept: HashMap<u32, Kept>,
}

impl Jbig2Decoder {
    pub fn decode(globals: &[u8], data: &[u8]) -> Result<(usize, usize, Vec<u8>)> {
        let mut decoder = Jbig2Decoder { page: None, default_black: false, height_known: true, kept: HashMap::new() };
        for segment in SegmentReader::new(globals).chain(SegmentReader::new(data)) {
            if !decoder.apply(&segment?)? {
                break;
            }
        }
        let page = decoder.page.ok_or_else(|| Error::new("the JBIG2 stream has no page"))?;
        let (width, height) = (page.width, page.height);
        Ok((width, height, page.into_packed().into_iter().map(|byte| !byte).collect()))
    }

    fn apply(&mut self, segment: &Segment) -> Result<bool> {
        let immediate = |kind: u8| kind & 2 != 0;
        match segment.kind {
            0 => {
                let symbols = self.symbol_dictionary(segment)?;
                self.kept.insert(segment.number, Kept::Symbols(symbols));
            }
            4 | 6 | 7 => {
                let (info, region) = self.text_region(segment)?;
                self.place(segment, immediate(segment.kind), info, region)?;
            }
            16 => {
                let patterns = pattern_dictionary(segment.data)?;
                self.kept.insert(segment.number, Kept::Patterns(patterns));
            }
            20 | 22 | 23 => {
                let (info, region) = self.halftone_region(segment)?;
                self.place(segment, immediate(segment.kind), info, region)?;
            }
            36 | 38 | 39 => {
                let (info, region) = generic_region(segment.data)?;
                self.place(segment, immediate(segment.kind), info, region)?;
            }
            40 | 42 | 43 => {
                let (info, region) = self.refinement_region(segment)?;
                self.place(segment, immediate(segment.kind), info, region)?;
            }
            48 => self.start_page(segment.data)?,
            49 | 51 => return Ok(false),
            50 => {
                let end = number(segment.data, 0)? as usize;
                if !self.height_known
                    && let Some(page) = &mut self.page
                {
                    page.grow(end + 1, self.default_black);
                }
            }
            52 | 62 => {}
            53 => {
                self.kept.insert(segment.number, Kept::Table(HuffmanTable::from_segment(segment.data)?));
            }
            kind => return Err(Error::unsupported(format!("JBIG2 segments of type {kind} are not supported"))),
        }
        Ok(true)
    }

    fn place(&mut self, segment: &Segment, immediate: bool, info: RegionInfo, region: Bitmap) -> Result<()> {
        if !immediate {
            self.kept.insert(segment.number, Kept::Region(region));
            return Ok(());
        }
        let page = self.page.as_mut().ok_or_else(|| Error::new("a JBIG2 region comes before its page"))?;
        if !self.height_known {
            page.grow(info.top.max(0) as usize + info.height, self.default_black);
        }
        page.compose(&region, info.left, info.top, info.combination);
        Ok(())
    }

    fn start_page(&mut self, data: &[u8]) -> Result<()> {
        let (width, height) = (number(data, 0)?, number(data, 4)?);
        self.default_black = byte(data, 16)? & 0x04 != 0;
        self.height_known = height != UNKNOWN;
        let height = if self.height_known { height as usize } else { 0 };
        self.page = Some(Bitmap::new(width as usize, height, self.default_black));
        Ok(())
    }

    fn referred<'k>(&'k self, segment: &Segment, pick: impl Fn(&'k Kept) -> Option<&'k Vec<Bitmap>>) -> Vec<Bitmap> {
        segment.referred.iter().filter_map(|number| self.kept.get(number)).filter_map(pick).flatten().cloned().collect()
    }

    fn symbol_dictionary(&self, segment: &Segment) -> Result<Vec<Bitmap>> {
        let data = segment.data;
        let flags = u16::from_be_bytes([byte(data, 0)?, byte(data, 1)?]);
        let huffman = flags & 1 != 0;
        let refine_aggregate = flags & 2 != 0;
        let template = ((flags >> 10) & 3) as u8;
        let refinement_template = ((flags >> 12) & 1) as u8;
        let mut at = 2;
        let adaptive = match huffman {
            true => Vec::new(),
            false => adaptive_pixels(data, &mut at, if template == 0 { 4 } else { 1 })?,
        };
        let refinement_adaptive = if refine_aggregate && refinement_template == 0 {
            let pixels = adaptive_pixels(data, &mut at, 2)?;
            [(pixels[0].0 as i8, pixels[0].1 as i8), (pixels[1].0 as i8, pixels[1].1 as i8)]
        } else {
            DEFAULT_REFINEMENT_ADAPTIVE
        };
        let dictionary = SymbolDictionary {
            template,
            adaptive,
            refine_aggregate,
            refinement_template,
            refinement_adaptive,
            exported: number(data, at)? as usize,
            new: number(data, at + 4)? as usize,
        };
        let input = self.referred(segment, |kept| match kept {
            Kept::Symbols(symbols) => Some(symbols),
            _ => None,
        });
        let coded = data.get(at + 8..).ok_or_else(|| Error::new("a JBIG2 segment is cut short"))?;
        if !huffman {
            return dictionary.decode(&input, &mut MqDecoder::new(coded));
        }
        let tables = DictionaryTables::selected(flags, &mut self.tables(segment).into_iter())?;
        let mut contexts = RefinementRegion::contexts(refinement_template);
        dictionary.decode_huffman(&input, &mut BitReader::new(coded), &tables, &mut contexts)
    }

    fn tables(&self, segment: &Segment) -> Vec<HuffmanTable> {
        segment
            .referred
            .iter()
            .filter_map(|number| match self.kept.get(number) {
                Some(Kept::Table(table)) => Some(table.clone()),
                _ => None,
            })
            .collect()
    }

    fn text_region(&self, segment: &Segment) -> Result<(RegionInfo, Bitmap)> {
        let data = segment.data;
        let info = region_info(data)?;
        let flags = u16::from_be_bytes([byte(data, 17)?, byte(data, 18)?]);
        let huffman = flags & 1 != 0;
        let refine = flags & 2 != 0;
        let refinement_template = (flags >> 15) as u8;
        let mut at = 19;
        let huffman_flags = match huffman {
            true => {
                at += 2;
                u16::from_be_bytes([byte(data, 19)?, byte(data, 20)?])
            }
            false => 0,
        };
        let refinement_adaptive = if refine && refinement_template == 0 {
            let pixels = adaptive_pixels(data, &mut at, 2)?;
            [(pixels[0].0 as i8, pixels[0].1 as i8), (pixels[1].0 as i8, pixels[1].1 as i8)]
        } else {
            DEFAULT_REFINEMENT_ADAPTIVE
        };
        let instances = number(data, at)? as usize;
        let symbols = self.referred(segment, |kept| match kept {
            Kept::Symbols(symbols) => Some(symbols),
            _ => None,
        });
        let offset = ((flags >> 10) & 0x1f) as i64;
        let region = TextRegion {
            width: info.width,
            height: info.height,
            instances,
            strips: 1 << ((flags >> 2) & 3),
            corner: Corner::from_bits(flags >> 4),
            transposed: flags & 0x40 != 0,
            combination: Combination::from_bits(((flags >> 7) & 3) as u8),
            default_black: flags & 0x200 != 0,
            s_offset: if offset >= 16 { offset - 32 } else { offset },
            refine,
            refinement_template,
            refinement_adaptive,
            symbols: &symbols,
        };
        let coded = data.get(at + 4..).ok_or_else(|| Error::new("a JBIG2 segment is cut short"))?;
        let bitmap = if huffman {
            let tables = TextTables::selected(huffman_flags, &mut self.tables(segment).into_iter())?;
            let mut reader = BitReader::new(coded);
            let ids = SymbolIds::read(&mut reader, symbols.len())?;
            let mut contexts = RefinementRegion::contexts(refinement_template);
            region.decode(&mut HuffmanText {
                reader: &mut reader,
                tables: &tables,
                ids: &ids,
                refinement: &mut contexts,
            })?
        } else {
            let mut state = CodingState::new(symbol_code_length(symbols.len()), 0, refinement_template);
            region.decode(&mut ArithmeticText { decoder: &mut MqDecoder::new(coded), state: &mut state })?
        };
        Ok((info, bitmap))
    }

    fn halftone_region(&self, segment: &Segment) -> Result<(RegionInfo, Bitmap)> {
        let data = segment.data;
        let info = region_info(data)?;
        let flags = byte(data, 17)?;
        let patterns = self.referred(segment, |kept| match kept {
            Kept::Patterns(patterns) => Some(patterns),
            _ => None,
        });
        let short = |at: usize| Ok::<i64, Error>(u16::from_be_bytes([byte(data, at)?, byte(data, at + 1)?]) as i64);
        let region = HalftoneRegion {
            width: info.width,
            height: info.height,
            mmr: flags & 1 != 0,
            template: (flags >> 1) & 3,
            skip_outside: flags & 0x08 != 0,
            combination: Combination::from_bits((flags >> 4) & 7),
            default_black: flags & 0x80 != 0,
            grid_width: number(data, 18)? as usize,
            grid_height: number(data, 22)? as usize,
            grid_x: number(data, 26)? as i32 as i64,
            grid_y: number(data, 30)? as i32 as i64,
            step_x: short(34)?,
            step_y: short(36)?,
            patterns: &patterns,
        };
        let bitmap = region.decode(data.get(38..).ok_or_else(|| Error::new("a JBIG2 segment is cut short"))?);
        Ok((info, bitmap))
    }

    fn refinement_region(&self, segment: &Segment) -> Result<(RegionInfo, Bitmap)> {
        let data = segment.data;
        let info = region_info(data)?;
        let flags = byte(data, 17)?;
        let template = flags & 1;
        let mut at = 18;
        let adaptive = if template == 0 {
            let pixels = adaptive_pixels(data, &mut at, 2)?;
            [(pixels[0].0 as i8, pixels[0].1 as i8), (pixels[1].0 as i8, pixels[1].1 as i8)]
        } else {
            DEFAULT_REFINEMENT_ADAPTIVE
        };
        let referred = segment.referred.iter().find_map(|number| match self.kept.get(number) {
            Some(Kept::Region(region)) => Some(region.clone()),
            _ => None,
        });
        let reference = match referred {
            Some(region) => region,
            None => {
                let page = self.page.as_ref().ok_or_else(|| Error::new("a JBIG2 refinement comes before its page"))?;
                let mut under = Bitmap::new(info.width, info.height, false);
                under.compose_from(page, info.left, info.top);
                under
            }
        };
        let region = RefinementRegion {
            width: info.width,
            height: info.height,
            template,
            reference: &reference,
            offset: (0, 0),
            typical_prediction: flags & 2 != 0,
            adaptive,
        };
        let mut contexts = RefinementRegion::contexts(template);
        let bitmap = region.decode(&mut MqDecoder::new(&data[at..]), &mut contexts);
        Ok((info, bitmap))
    }
}

fn region_info(data: &[u8]) -> Result<RegionInfo> {
    Ok(RegionInfo {
        width: number(data, 0)? as usize,
        height: number(data, 4)? as usize,
        left: number(data, 8)? as i32 as i64,
        top: number(data, 12)? as i32 as i64,
        combination: Combination::from_bits(byte(data, 16)?),
    })
}

fn generic_region(data: &[u8]) -> Result<(RegionInfo, Bitmap)> {
    let info = region_info(data)?;
    let flags = byte(data, 17)?;
    let template = (flags >> 1) & 3;
    let region = if flags & 1 != 0 {
        let params = FaxParams {
            k: -1,
            columns: info.width,
            rows: Some(info.height),
            byte_aligned: false,
            end_of_line: false,
            black_is_1: true,
        };
        let (pixels, _) = FaxDecoder::new(params).decode(&data[18..]);
        Bitmap::from_packed(info.width, info.height, pixels)
    } else {
        let mut at = 18;
        let adaptive = adaptive_pixels(data, &mut at, if template == 0 { 4 } else { 1 })?;
        let region = GenericRegion {
            width: info.width,
            height: info.height,
            template,
            typical_prediction: flags & 0x08 != 0,
            adaptive,
            skip: None,
        };
        let mut contexts = GenericRegion::contexts(template);
        region.decode(&mut MqDecoder::new(&data[at..]), &mut contexts)
    };
    Ok((info, region))
}

pub(super) fn adaptive_pixels(data: &[u8], at: &mut usize, count: usize) -> Result<Vec<(i64, i64)>> {
    let pixels = (0..count)
        .map(|index| Ok((byte(data, *at + index * 2)? as i8 as i64, byte(data, *at + index * 2 + 1)? as i8 as i64)))
        .collect();
    *at += count * 2;
    pixels
}

pub(super) fn byte(data: &[u8], at: usize) -> Result<u8> {
    data.get(at).copied().ok_or_else(|| Error::new("a JBIG2 segment is cut short"))
}

pub(super) fn number(data: &[u8], at: usize) -> Result<u32> {
    (at..at + 4).try_fold(0, |value, index| Ok(value << 8 | byte(data, index)? as u32))
}
