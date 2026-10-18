use crate::error::{Error, Result};
use crate::image::bit_reader::BitReader;
use crate::image::jbig2::bitmap::Bitmap;
use crate::image::jbig2::huffman_table::HuffmanTable;
use crate::image::jbig2::refinement_region::RefinementRegion;
use crate::image::jbig2::text_source::{TextSource, refinement_offset};
use crate::image::mq_decoder::{MqContext, MqDecoder};

const RUN_CODES: usize = 35;
const REPEAT_PREVIOUS: i64 = 32;
const SHORT_ZEROS: i64 = 33;
const LONG_ZEROS: i64 = 34;

pub struct TextTables {
    pub first_s: HuffmanTable,
    pub s_delta: HuffmanTable,
    pub strip_delta: HuffmanTable,
    pub refinement_width: HuffmanTable,
    pub refinement_height: HuffmanTable,
    pub refinement_x: HuffmanTable,
    pub refinement_y: HuffmanTable,
    pub refinement_size: HuffmanTable,
}

impl TextTables {
    pub fn selected(flags: u16, custom: &mut impl Iterator<Item = HuffmanTable>) -> Result<TextTables> {
        let mut pick = |shift: u16, standard: &[usize]| -> Result<HuffmanTable> {
            let choice = ((flags >> shift) & 3) as usize;
            match standard.get(choice) {
                Some(number) => Ok(HuffmanTable::standard(*number)),
                None if choice == 3 => {
                    custom.next().ok_or_else(|| Error::new("a JBIG2 text region lacks a code table"))
                }
                None => Err(Error::new("a JBIG2 text region selects a table that does not exist")),
            }
        };
        let first_s = pick(0, &[6, 7])?;
        let s_delta = pick(2, &[8, 9, 10])?;
        let strip_delta = pick(4, &[11, 12, 13])?;
        let refinement_width = pick(6, &[14, 15])?;
        let refinement_height = pick(8, &[14, 15])?;
        let refinement_x = pick(10, &[14, 15])?;
        let refinement_y = pick(12, &[14, 15])?;
        let refinement_size = if flags & 0x4000 == 0 {
            HuffmanTable::standard(1)
        } else {
            custom.next().ok_or_else(|| Error::new("a JBIG2 text region lacks a code table"))?
        };
        Ok(TextTables {
            first_s,
            s_delta,
            strip_delta,
            refinement_width,
            refinement_height,
            refinement_x,
            refinement_y,
            refinement_size,
        })
    }

    pub fn for_aggregates() -> TextTables {
        let standard = HuffmanTable::standard;
        TextTables {
            first_s: standard(6),
            s_delta: standard(8),
            strip_delta: standard(11),
            refinement_width: standard(15),
            refinement_height: standard(15),
            refinement_x: standard(15),
            refinement_y: standard(15),
            refinement_size: standard(1),
        }
    }
}

pub enum SymbolIds {
    Fixed(usize),
    Coded(HuffmanTable),
}

impl SymbolIds {
    pub fn read(reader: &mut BitReader, count: usize) -> Result<SymbolIds> {
        let run_lengths: Vec<u8> = (0..RUN_CODES).map(|_| reader.read(4) as u8).collect();
        let runs = HuffmanTable::of_lengths(&run_lengths);
        let mut lengths: Vec<u8> = Vec::with_capacity(count);
        while lengths.len() < count {
            let code = runs.decode(reader)?.ok_or_else(|| Error::new("a JBIG2 symbol ID table is broken"))?;
            let (length, times) = match code {
                REPEAT_PREVIOUS => {
                    let previous =
                        *lengths.last().ok_or_else(|| Error::new("a JBIG2 symbol ID table repeats nothing"))?;
                    (previous, 3 + reader.read(2) as usize)
                }
                SHORT_ZEROS => (0, 3 + reader.read(3) as usize),
                LONG_ZEROS => (0, 11 + reader.read(7) as usize),
                length => (length as u8, 1),
            };
            if lengths.len() + times > count {
                return Err(Error::new("a JBIG2 symbol ID table codes more symbols than there are"));
            }
            lengths.extend(std::iter::repeat_n(length, times));
        }
        reader.align();
        Ok(SymbolIds::Coded(HuffmanTable::of_lengths(&lengths)))
    }

    fn decode(&self, reader: &mut BitReader) -> Result<usize> {
        match self {
            SymbolIds::Fixed(bits) => Ok(reader.read(*bits) as usize),
            SymbolIds::Coded(table) => table
                .decode(reader)?
                .map(|id| id as usize)
                .ok_or_else(|| Error::new("a JBIG2 symbol ID is out of band")),
        }
    }
}

pub struct HuffmanText<'s, 'd> {
    pub reader: &'s mut BitReader<'d>,
    pub tables: &'s TextTables,
    pub ids: &'s SymbolIds,
    pub refinement: &'s mut [MqContext],
}

fn value(table: &HuffmanTable, reader: &mut BitReader) -> Result<i64> {
    table.decode(reader)?.ok_or_else(|| Error::new("a JBIG2 Huffman value is out of band where none may be"))
}

impl TextSource for HuffmanText<'_, '_> {
    fn strip_delta(&mut self) -> Result<i64> {
        value(&self.tables.strip_delta, self.reader)
    }

    fn first_s(&mut self) -> Result<i64> {
        value(&self.tables.first_s, self.reader)
    }

    fn s_delta(&mut self) -> Result<Option<i64>> {
        self.tables.s_delta.decode(self.reader)
    }

    fn instance_t(&mut self, strips: i64) -> Result<i64> {
        Ok(self.reader.read(strips.trailing_zeros() as usize) as i64)
    }

    fn symbol_id(&mut self) -> Result<usize> {
        self.ids.decode(self.reader)
    }

    fn refinement(&mut self, symbol: &Bitmap, template: u8, adaptive: [(i8, i8); 2]) -> Result<Option<Bitmap>> {
        if self.reader.read(1) == 0 {
            return Ok(None);
        }
        let width_delta = value(&self.tables.refinement_width, self.reader)?;
        let height_delta = value(&self.tables.refinement_height, self.reader)?;
        let x = value(&self.tables.refinement_x, self.reader)?;
        let y = value(&self.tables.refinement_y, self.reader)?;
        let size = refined_bytes(&self.tables.refinement_size, self.reader)?;
        let region = RefinementRegion {
            width: (symbol.width as i64 + width_delta).max(0) as usize,
            height: (symbol.height as i64 + height_delta).max(0) as usize,
            template,
            reference: symbol,
            offset: refinement_offset(width_delta, height_delta, x, y),
            typical_prediction: false,
            adaptive,
        };
        let bitmap = region.decode(&mut MqDecoder::new(&self.reader.rest()[..size]), self.refinement);
        self.reader.consume(size * 8);
        Ok(Some(bitmap))
    }
}

pub fn refined_bytes(table: &HuffmanTable, reader: &mut BitReader) -> Result<usize> {
    let size =
        usize::try_from(value(table, reader)?).map_err(|_| Error::new("a JBIG2 refinement has a negative size"))?;
    reader.align();
    if size > reader.rest().len() {
        return Err(Error::new("a JBIG2 refinement runs past its segment"));
    }
    Ok(size)
}
