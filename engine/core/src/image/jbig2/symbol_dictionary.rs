use crate::error::{Error, Result};
use crate::image::bit_reader::BitReader;
use crate::image::ccitt::{FaxDecoder, FaxParams};
use crate::image::jbig2::arithmetic_text::ArithmeticText;
use crate::image::jbig2::bitmap::{Bitmap, Combination};
use crate::image::jbig2::coding_state::CodingState;
use crate::image::jbig2::generic_region::GenericRegion;
use crate::image::jbig2::huffman_table::HuffmanTable;
use crate::image::jbig2::huffman_text::{HuffmanText, SymbolIds, TextTables, refined_bytes};
use crate::image::jbig2::integer_decoder::symbol_code_length;
use crate::image::jbig2::refinement_region::RefinementRegion;
use crate::image::jbig2::text_region::{Corner, TextRegion};
use crate::image::mq_decoder::{MqContext, MqDecoder};

pub struct DictionaryTables {
    pub height_delta: HuffmanTable,
    pub width_delta: HuffmanTable,
    pub bitmap_size: HuffmanTable,
    pub aggregate_count: HuffmanTable,
}

impl DictionaryTables {
    pub fn selected(flags: u16, custom: &mut impl Iterator<Item = HuffmanTable>) -> Result<DictionaryTables> {
        let mut user = || custom.next().ok_or_else(|| Error::new("a JBIG2 symbol dictionary lacks a code table"));
        let height_delta = match (flags >> 2) & 3 {
            0 => HuffmanTable::standard(4),
            1 => HuffmanTable::standard(5),
            3 => user()?,
            _ => return Err(Error::new("a JBIG2 symbol dictionary selects a table that does not exist")),
        };
        let width_delta = match (flags >> 4) & 3 {
            0 => HuffmanTable::standard(2),
            1 => HuffmanTable::standard(3),
            3 => user()?,
            _ => return Err(Error::new("a JBIG2 symbol dictionary selects a table that does not exist")),
        };
        let bitmap_size = if flags & 0x40 == 0 { HuffmanTable::standard(1) } else { user()? };
        let aggregate_count = if flags & 0x80 == 0 { HuffmanTable::standard(1) } else { user()? };
        Ok(DictionaryTables { height_delta, width_delta, bitmap_size, aggregate_count })
    }
}

pub struct SymbolDictionary {
    pub template: u8,
    pub adaptive: Vec<(i64, i64)>,
    pub refine_aggregate: bool,
    pub refinement_template: u8,
    pub refinement_adaptive: [(i8, i8); 2],
    pub exported: usize,
    pub new: usize,
}

impl SymbolDictionary {
    pub fn decode(&self, input: &[Bitmap], decoder: &mut MqDecoder) -> Result<Vec<Bitmap>> {
        let broken = || Error::new("a JBIG2 symbol dictionary ends early");
        let mut state =
            CodingState::new(symbol_code_length(input.len() + self.new), self.template, self.refinement_template);
        let mut symbols: Vec<Bitmap> = input.to_vec();
        let mut height = 0;
        while symbols.len() < input.len() + self.new {
            height += state.height_delta.decode(decoder).ok_or_else(broken)?;
            let mut width = 0;
            while let Some(delta) = state.width_delta.decode(decoder) {
                if symbols.len() == input.len() + self.new {
                    return Err(Error::new("a JBIG2 symbol dictionary defines more symbols than it announces"));
                }
                width += delta;
                let (width, height) =
                    (usize::try_from(width).map_err(|_| broken())?, usize::try_from(height).map_err(|_| broken())?);
                let symbol = if !self.refine_aggregate {
                    let region = GenericRegion {
                        width,
                        height,
                        template: self.template,
                        typical_prediction: false,
                        adaptive: self.adaptive.clone(),
                        skip: None,
                    };
                    region.decode(decoder, &mut state.generic)
                } else {
                    let instances = state.instance_count.decode(decoder).ok_or_else(broken)?;
                    if instances == 1 {
                        let id = state.symbol_id.decode(decoder);
                        let x = state.refinement_x.decode(decoder).ok_or_else(broken)?;
                        let y = state.refinement_y.decode(decoder).ok_or_else(broken)?;
                        let reference = symbols
                            .get(id)
                            .ok_or_else(|| Error::new("a JBIG2 symbol refines a symbol not yet known"))?;
                        let region = RefinementRegion {
                            width,
                            height,
                            template: self.refinement_template,
                            reference,
                            offset: (x, y),
                            typical_prediction: false,
                            adaptive: self.refinement_adaptive,
                        };
                        region.decode(decoder, &mut state.refinement)
                    } else {
                        let region = TextRegion {
                            width,
                            height,
                            instances: usize::try_from(instances).map_err(|_| broken())?,
                            strips: 1,
                            corner: Corner::TopLeft,
                            transposed: false,
                            combination: Combination::Or,
                            default_black: false,
                            s_offset: 0,
                            refine: true,
                            refinement_template: self.refinement_template,
                            refinement_adaptive: self.refinement_adaptive,
                            symbols: &symbols.clone(),
                        };
                        region.decode(&mut ArithmeticText { decoder, state: &mut state })?
                    }
                };
                symbols.push(symbol);
            }
        }
        let export_run = &mut state.export_run;
        self.exported(&symbols, || export_run.decode(decoder).ok_or_else(broken))
    }

    pub fn decode_huffman(
        &self,
        input: &[Bitmap],
        reader: &mut BitReader,
        tables: &DictionaryTables,
        refinement: &mut [MqContext],
    ) -> Result<Vec<Bitmap>> {
        let broken = || Error::new("a JBIG2 symbol dictionary ends early");
        let total = input.len() + self.new;
        let aggregates = TextTables::for_aggregates();
        let ids = SymbolIds::Fixed(symbol_code_length(total) as usize);
        let mut symbols: Vec<Bitmap> = input.to_vec();
        let mut height = 0i64;
        while symbols.len() < total {
            height += tables.height_delta.decode(reader)?.ok_or_else(broken)?;
            let class_height = usize::try_from(height).map_err(|_| broken())?;
            let mut width = 0i64;
            let mut widths = Vec::new();
            while let Some(delta) = tables.width_delta.decode(reader)? {
                if symbols.len() + widths.len() >= total {
                    return Err(Error::new("a JBIG2 symbol dictionary defines more symbols than it announces"));
                }
                width += delta;
                let symbol_width = usize::try_from(width).map_err(|_| broken())?;
                if !self.refine_aggregate {
                    widths.push(symbol_width);
                    continue;
                }
                let count = tables.aggregate_count.decode(reader)?.ok_or_else(broken)?;
                let symbol = if count == 1 {
                    let id = reader.read(symbol_code_length(total) as usize) as usize;
                    let x = aggregates.refinement_x.decode(reader)?.ok_or_else(broken)?;
                    let y = aggregates.refinement_y.decode(reader)?.ok_or_else(broken)?;
                    let size = refined_bytes(&aggregates.refinement_size, reader)?;
                    let reference =
                        symbols.get(id).ok_or_else(|| Error::new("a JBIG2 symbol refines a symbol not yet known"))?;
                    let region = RefinementRegion {
                        width: symbol_width,
                        height: class_height,
                        template: self.refinement_template,
                        reference,
                        offset: (x, y),
                        typical_prediction: false,
                        adaptive: self.refinement_adaptive,
                    };
                    let bitmap = region.decode(&mut MqDecoder::new(&reader.rest()[..size]), refinement);
                    reader.consume(size * 8);
                    bitmap
                } else {
                    let known = symbols.clone();
                    let region = self.aggregate(symbol_width, class_height, count, &known)?;
                    region.decode(&mut HuffmanText { reader, tables: &aggregates, ids: &ids, refinement })?
                };
                symbols.push(symbol);
            }
            if !self.refine_aggregate {
                symbols.extend(collective(reader, &tables.bitmap_size, &widths, class_height)?);
            }
        }
        let runs = HuffmanTable::standard(1);
        self.exported(&symbols, || runs.decode(reader)?.ok_or_else(broken))
    }

    fn aggregate<'s>(&self, width: usize, height: usize, count: i64, symbols: &'s [Bitmap]) -> Result<TextRegion<'s>> {
        Ok(TextRegion {
            width,
            height,
            instances: usize::try_from(count).map_err(|_| Error::new("a JBIG2 aggregate has no parts"))?,
            strips: 1,
            corner: Corner::TopLeft,
            transposed: false,
            combination: Combination::Or,
            default_black: false,
            s_offset: 0,
            refine: true,
            refinement_template: self.refinement_template,
            refinement_adaptive: self.refinement_adaptive,
            symbols,
        })
    }

    fn exported(&self, symbols: &[Bitmap], mut run: impl FnMut() -> Result<i64>) -> Result<Vec<Bitmap>> {
        let mut exported = Vec::with_capacity(self.exported);
        let mut exporting = false;
        let mut index = 0;
        while index < symbols.len() {
            let length = usize::try_from(run()?).map_err(|_| Error::new("a JBIG2 export run is negative"))?;
            let end = (index + length).min(symbols.len());
            if exporting {
                exported.extend_from_slice(&symbols[index..end]);
            }
            index = end;
            exporting = !exporting;
        }
        Ok(exported)
    }
}

fn collective(
    reader: &mut BitReader,
    size_table: &HuffmanTable,
    widths: &[usize],
    height: usize,
) -> Result<Vec<Bitmap>> {
    let size = size_table.decode(reader)?.ok_or_else(|| Error::new("a JBIG2 collective bitmap has no size"))?;
    let size = usize::try_from(size).map_err(|_| Error::new("a JBIG2 collective bitmap has a negative size"))?;
    reader.align();
    let total_width: usize = widths.iter().sum();
    let data = reader.rest();
    let (pixels, used) = if size == 0 {
        let used = total_width.div_ceil(8) * height;
        (data.get(..used).ok_or_else(|| Error::new("a JBIG2 collective bitmap is cut short"))?.to_vec(), used)
    } else {
        let coded = data.get(..size).ok_or_else(|| Error::new("a JBIG2 collective bitmap is cut short"))?;
        let params = FaxParams {
            k: -1,
            columns: total_width,
            rows: Some(height),
            byte_aligned: false,
            end_of_line: false,
            black_is_1: true,
        };
        (FaxDecoder::new(params).decode(coded).0, size)
    };
    reader.consume(used * 8);
    let bitmap = Bitmap::from_packed(total_width, height, pixels);
    let mut left = 0;
    Ok(widths
        .iter()
        .map(|width| {
            let mut symbol = Bitmap::new(*width, height, false);
            symbol.compose_from(&bitmap, left as i64, 0);
            left += width;
            symbol
        })
        .collect())
}
