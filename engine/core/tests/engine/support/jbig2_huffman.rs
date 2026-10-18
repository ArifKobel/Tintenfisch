use crate::support::jbig2_writer::{
    Context, MqEncoder, Picture, Placed, TextLayout, TextSink, encode_refinement, encode_text, instance_refinement,
    refinement_contexts, region_information, size_change,
};

#[derive(Default)]
pub struct BitWriter {
    bytes: Vec<u8>,
    used: u32,
}

impl BitWriter {
    pub fn put(&mut self, value: u64, bits: u32) {
        for shift in (0..bits).rev() {
            if self.used.is_multiple_of(8) {
                self.bytes.push(0);
            }
            let bit = (value >> shift & 1) as u8;
            *self.bytes.last_mut().unwrap() |= bit << (7 - self.used % 8);
            self.used += 1;
        }
    }

    pub fn align(&mut self) {
        self.used = self.used.div_ceil(8) * 8;
    }

    pub fn append(&mut self, bytes: &[u8]) {
        self.align();
        self.bytes.extend_from_slice(bytes);
        self.used += 8 * bytes.len() as u32;
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Range,
    Lower,
    Upper,
    OutOfBand,
}

#[derive(Clone, Copy, Debug)]
struct CodeLine {
    kind: Kind,
    prefix: u32,
    range: u32,
    low: i64,
    code: u32,
}

#[derive(Clone, Debug)]
pub struct CodeTable {
    lines: Vec<CodeLine>,
}

const R: Kind = Kind::Range;
const LO: Kind = Kind::Lower;
const UP: Kind = Kind::Upper;
const OOB: Kind = Kind::OutOfBand;

type StandardLine = (Kind, u32, u32, i64, u32);

#[rustfmt::skip]
const STANDARD: [&[StandardLine]; 15] = [
    &[(R, 1, 4, 0, 0x0), (R, 2, 8, 16, 0x2), (R, 3, 16, 272, 0x6), (UP, 3, 32, 65808, 0x7)],
    &[(R, 1, 0, 0, 0x0), (R, 2, 0, 1, 0x2), (R, 3, 0, 2, 0x6), (R, 4, 3, 3, 0xe), (R, 5, 6, 11, 0x1e), (UP, 6, 32, 75, 0x3e), (OOB, 6, 0, 0, 0x3f)],
    &[(R, 8, 8, -256, 0xfe), (R, 1, 0, 0, 0x0), (R, 2, 0, 1, 0x2), (R, 3, 0, 2, 0x6), (R, 4, 3, 3, 0xe), (R, 5, 6, 11, 0x1e), (LO, 8, 32, -257, 0xff), (UP, 7, 32, 75, 0x7e), (OOB, 6, 0, 0, 0x3e)],
    &[(R, 1, 0, 1, 0x0), (R, 2, 0, 2, 0x2), (R, 3, 0, 3, 0x6), (R, 4, 3, 4, 0xe), (R, 5, 6, 12, 0x1e), (UP, 5, 32, 76, 0x1f)],
    &[(R, 7, 8, -255, 0x7e), (R, 1, 0, 1, 0x0), (R, 2, 0, 2, 0x2), (R, 3, 0, 3, 0x6), (R, 4, 3, 4, 0xe), (R, 5, 6, 12, 0x1e), (LO, 7, 32, -256, 0x7f), (UP, 6, 32, 76, 0x3e)],
    &[(R, 5, 10, -2048, 0x1c), (R, 4, 9, -1024, 0x8), (R, 4, 8, -512, 0x9), (R, 4, 7, -256, 0xa), (R, 5, 6, -128, 0x1d), (R, 5, 5, -64, 0x1e), (R, 4, 5, -32, 0xb), (R, 2, 7, 0, 0x0), (R, 3, 7, 128, 0x2), (R, 3, 8, 256, 0x3), (R, 4, 9, 512, 0xc), (R, 4, 10, 1024, 0xd), (LO, 6, 32, -2049, 0x3e), (UP, 6, 32, 2048, 0x3f)],
    &[(R, 4, 9, -1024, 0x8), (R, 3, 8, -512, 0x0), (R, 4, 7, -256, 0x9), (R, 5, 6, -128, 0x1a), (R, 5, 5, -64, 0x1b), (R, 4, 5, -32, 0xa), (R, 4, 5, 0, 0xb), (R, 5, 5, 32, 0x1c), (R, 5, 6, 64, 0x1d), (R, 4, 7, 128, 0xc), (R, 3, 8, 256, 0x1), (R, 3, 9, 512, 0x2), (R, 3, 10, 1024, 0x3), (LO, 5, 32, -1025, 0x1e), (UP, 5, 32, 2048, 0x1f)],
    &[(R, 8, 3, -15, 0xfc), (R, 9, 1, -7, 0x1fc), (R, 8, 1, -5, 0xfd), (R, 9, 0, -3, 0x1fd), (R, 7, 0, -2, 0x7c), (R, 4, 0, -1, 0xa), (R, 2, 1, 0, 0x0), (R, 5, 0, 2, 0x1a), (R, 6, 0, 3, 0x3a), (R, 3, 4, 4, 0x4), (R, 6, 1, 20, 0x3b), (R, 4, 4, 22, 0xb), (R, 4, 5, 38, 0xc), (R, 5, 6, 70, 0x1b), (R, 5, 7, 134, 0x1c), (R, 6, 7, 262, 0x3c), (R, 7, 8, 390, 0x7d), (R, 6, 10, 646, 0x3d), (LO, 9, 32, -16, 0x1fe), (UP, 9, 32, 1670, 0x1ff), (OOB, 2, 0, 0, 0x1)],
    &[(R, 8, 4, -31, 0xfc), (R, 9, 2, -15, 0x1fc), (R, 8, 2, -11, 0xfd), (R, 9, 1, -7, 0x1fd), (R, 7, 1, -5, 0x7c), (R, 4, 1, -3, 0xa), (R, 3, 1, -1, 0x2), (R, 3, 1, 1, 0x3), (R, 5, 1, 3, 0x1a), (R, 6, 1, 5, 0x3a), (R, 3, 5, 7, 0x4), (R, 6, 2, 39, 0x3b), (R, 4, 5, 43, 0xb), (R, 4, 6, 75, 0xc), (R, 5, 7, 139, 0x1b), (R, 5, 8, 267, 0x1c), (R, 6, 8, 523, 0x3c), (R, 7, 9, 779, 0x7d), (R, 6, 11, 1291, 0x3d), (LO, 9, 32, -32, 0x1fe), (UP, 9, 32, 3339, 0x1ff), (OOB, 2, 0, 0, 0x0)],
    &[(R, 7, 4, -21, 0x7a), (R, 8, 0, -5, 0xfc), (R, 7, 0, -4, 0x7b), (R, 5, 0, -3, 0x18), (R, 2, 2, -2, 0x0), (R, 5, 0, 2, 0x19), (R, 6, 0, 3, 0x36), (R, 7, 0, 4, 0x7c), (R, 8, 0, 5, 0xfd), (R, 2, 6, 6, 0x1), (R, 5, 5, 70, 0x1a), (R, 6, 5, 102, 0x37), (R, 6, 6, 134, 0x38), (R, 6, 7, 198, 0x39), (R, 6, 8, 326, 0x3a), (R, 6, 9, 582, 0x3b), (R, 6, 10, 1094, 0x3c), (R, 7, 11, 2118, 0x7d), (LO, 8, 32, -22, 0xfe), (UP, 8, 32, 4166, 0xff), (OOB, 2, 0, 0, 0x2)],
    &[(R, 1, 0, 1, 0x0), (R, 2, 1, 2, 0x2), (R, 4, 0, 4, 0xc), (R, 4, 1, 5, 0xd), (R, 5, 1, 7, 0x1c), (R, 5, 2, 9, 0x1d), (R, 6, 2, 13, 0x3c), (R, 7, 2, 17, 0x7a), (R, 7, 3, 21, 0x7b), (R, 7, 4, 29, 0x7c), (R, 7, 5, 45, 0x7d), (R, 7, 6, 77, 0x7e), (UP, 7, 32, 141, 0x7f)],
    &[(R, 1, 0, 1, 0x0), (R, 2, 0, 2, 0x2), (R, 3, 1, 3, 0x6), (R, 5, 0, 5, 0x1c), (R, 5, 1, 6, 0x1d), (R, 6, 1, 8, 0x3c), (R, 7, 0, 10, 0x7a), (R, 7, 1, 11, 0x7b), (R, 7, 2, 13, 0x7c), (R, 7, 3, 17, 0x7d), (R, 7, 4, 25, 0x7e), (R, 8, 5, 41, 0xfe), (UP, 8, 32, 73, 0xff)],
    &[(R, 1, 0, 1, 0x0), (R, 3, 0, 2, 0x4), (R, 4, 0, 3, 0xc), (R, 5, 0, 4, 0x1c), (R, 4, 1, 5, 0xd), (R, 3, 3, 7, 0x5), (R, 6, 1, 15, 0x3a), (R, 6, 2, 17, 0x3b), (R, 6, 3, 21, 0x3c), (R, 6, 4, 29, 0x3d), (R, 6, 5, 45, 0x3e), (R, 7, 6, 77, 0x7e), (UP, 7, 32, 141, 0x7f)],
    &[(R, 3, 0, -2, 0x4), (R, 3, 0, -1, 0x5), (R, 1, 0, 0, 0x0), (R, 3, 0, 1, 0x6), (R, 3, 0, 2, 0x7)],
    &[(R, 7, 4, -24, 0x7c), (R, 6, 2, -8, 0x3c), (R, 5, 1, -4, 0x1c), (R, 4, 0, -2, 0xc), (R, 3, 0, -1, 0x4), (R, 1, 0, 0, 0x0), (R, 3, 0, 1, 0x5), (R, 4, 0, 2, 0xd), (R, 5, 1, 3, 0x1d), (R, 6, 2, 5, 0x3d), (R, 7, 4, 9, 0x7d), (LO, 7, 32, -25, 0x7e), (UP, 7, 32, 25, 0x7f)],
];

impl CodeTable {
    pub fn standard(number: usize) -> CodeTable {
        let lines = STANDARD[number - 1]
            .iter()
            .map(|&(kind, prefix, range, low, code)| CodeLine { kind, prefix, range, low, code })
            .collect();
        CodeTable { lines }
    }

    pub fn of_lengths(lengths: &[u32]) -> CodeTable {
        let lines = lengths.iter().enumerate().map(|(value, prefix)| (Kind::Range, *prefix, 0, value as i64)).collect();
        CodeTable::assigned(lines)
    }

    fn assigned(lines: Vec<(Kind, u32, u32, i64)>) -> CodeTable {
        let mut coded: Vec<CodeLine> = Vec::new();
        let mut code = 0u32;
        let longest = lines.iter().map(|line| line.1).max().unwrap_or(0);
        for length in 1..=longest {
            for &(kind, prefix, range, low) in lines.iter().filter(|line| line.1 == length) {
                coded.push(CodeLine { kind, prefix, range, low, code });
                code += 1;
            }
            code <<= 1;
        }
        let order =
            |line: &CodeLine| lines.iter().position(|other| other.0 == line.kind && other.3 == line.low).unwrap();
        coded.sort_by_key(order);
        CodeTable { lines: coded }
    }

    pub fn put(&self, writer: &mut BitWriter, value: Option<i64>) {
        let find = |kind: Kind| self.lines.iter().find(|line| line.kind == kind);
        let (line, offset) = match value {
            None => (find(Kind::OutOfBand).expect("the table has no out of band value"), 0),
            Some(value) => match self
                .lines
                .iter()
                .find(|line| line.kind == Kind::Range && value >= line.low && value < line.low + (1 << line.range))
            {
                Some(line) => (line, (value - line.low) as u64),
                None => {
                    let lower = find(Kind::Lower).filter(|line| value <= line.low);
                    match lower {
                        Some(line) => (line, (line.low - value) as u64),
                        None => {
                            let line =
                                find(Kind::Upper).filter(|line| value >= line.low).expect("value outside the table");
                            (line, (value - line.low) as u64)
                        }
                    }
                }
            },
        };
        writer.put(line.code as u64, line.prefix);
        writer.put(offset, line.range);
    }
}

pub struct CustomTable {
    pub low: i64,
    pub ranges: Vec<(u32, u32)>,
    pub lower: u32,
    pub upper: u32,
    pub out_of_band: Option<u32>,
}

impl CustomTable {
    pub fn high(&self) -> i64 {
        self.ranges.iter().fold(self.low, |low, (_, range)| low + (1 << range))
    }

    pub fn segment_data(&self, prefix_bits: u32, range_bits: u32) -> Vec<u8> {
        let mut data =
            vec![self.out_of_band.is_some() as u8 | ((prefix_bits - 1) << 1) as u8 | ((range_bits - 1) << 4) as u8];
        data.extend((self.low as i32).to_be_bytes());
        data.extend((self.high() as i32).to_be_bytes());
        let mut writer = BitWriter::default();
        for (prefix, range) in &self.ranges {
            writer.put(*prefix as u64, prefix_bits);
            writer.put(*range as u64, range_bits);
        }
        writer.put(self.lower as u64, prefix_bits);
        writer.put(self.upper as u64, prefix_bits);
        if let Some(prefix) = self.out_of_band {
            writer.put(prefix as u64, prefix_bits);
        }
        data.extend(writer.finish());
        data
    }

    pub fn codes(&self) -> CodeTable {
        let mut lines = Vec::new();
        let mut low = self.low;
        for (prefix, range) in &self.ranges {
            lines.push((Kind::Range, *prefix, *range, low));
            low += 1 << range;
        }
        lines.push((Kind::Lower, self.lower, 32, self.low - 1));
        lines.push((Kind::Upper, self.upper, 32, self.high()));
        if let Some(prefix) = self.out_of_band {
            lines.push((Kind::OutOfBand, prefix, 0, 0));
        }
        CodeTable::assigned(lines)
    }
}

pub struct TextCodes {
    pub first_s: CodeTable,
    pub s_delta: CodeTable,
    pub strip_delta: CodeTable,
    pub refinement_width: CodeTable,
    pub refinement_height: CodeTable,
    pub refinement_x: CodeTable,
    pub refinement_y: CodeTable,
    pub refinement_size: CodeTable,
}

impl TextCodes {
    pub fn aggregates() -> TextCodes {
        TextCodes {
            first_s: CodeTable::standard(6),
            s_delta: CodeTable::standard(8),
            strip_delta: CodeTable::standard(11),
            refinement_width: CodeTable::standard(15),
            refinement_height: CodeTable::standard(15),
            refinement_x: CodeTable::standard(15),
            refinement_y: CodeTable::standard(15),
            refinement_size: CodeTable::standard(1),
        }
    }
}

pub enum IdCodes {
    Fixed(u32),
    Coded(CodeTable),
}

pub struct HuffmanSink<'a> {
    pub writer: &'a mut BitWriter,
    pub codes: &'a TextCodes,
    pub ids: &'a IdCodes,
    pub refine: bool,
    pub contexts: &'a mut Vec<Context>,
}

impl TextSink for HuffmanSink<'_> {
    fn strip_delta(&mut self, delta: i64) {
        self.codes.strip_delta.put(self.writer, Some(delta));
    }

    fn first_s(&mut self, delta: i64) {
        self.codes.first_s.put(self.writer, Some(delta));
    }

    fn s_delta(&mut self, delta: Option<i64>) {
        self.codes.s_delta.put(self.writer, delta);
    }

    fn instance_t(&mut self, t: i64, strips: i64) {
        self.writer.put(t as u64, strips.trailing_zeros());
    }

    fn symbol_id(&mut self, id: usize) {
        match self.ids {
            IdCodes::Fixed(bits) => self.writer.put(id as u64, *bits),
            IdCodes::Coded(table) => table.put(self.writer, Some(id as i64)),
        }
    }

    fn refinement(&mut self, symbol: &Picture, refined: Option<(&Picture, i64, i64)>, template: u8) {
        if !self.refine {
            assert!(refined.is_none());
            return;
        }
        let Some((picture, x, y)) = refined else {
            self.writer.put(0, 1);
            return;
        };
        let (width_delta, height_delta) = size_change(symbol, picture);
        self.writer.put(1, 1);
        self.codes.refinement_width.put(self.writer, Some(width_delta));
        self.codes.refinement_height.put(self.writer, Some(height_delta));
        self.codes.refinement_x.put(self.writer, Some(x));
        self.codes.refinement_y.put(self.writer, Some(y));
        let mut encoder = MqEncoder::new();
        encode_refinement(&mut encoder, self.contexts, &instance_refinement(symbol, picture, x, y, template));
        let bytes = encoder.finish();
        self.codes.refinement_size.put(self.writer, Some(bytes.len() as i64));
        self.writer.append(&bytes);
    }
}

pub fn symbol_id_table(writer: &mut BitWriter, lengths: &[u32]) -> CodeTable {
    let runs = CodeTable::of_lengths(&[6; 35]);
    for _ in 0..35 {
        writer.put(6, 4);
    }
    let mut index = 0;
    while index < lengths.len() {
        let length = lengths[index];
        let same = lengths[index..].iter().take_while(|other| **other == length).count();
        if length == 0 && same >= 11 {
            let count = same.min(138);
            runs.put(writer, Some(34));
            writer.put(count as u64 - 11, 7);
            index += count;
        } else if length == 0 && same >= 3 {
            let count = same.min(10);
            runs.put(writer, Some(33));
            writer.put(count as u64 - 3, 3);
            index += count;
        } else if index > 0 && lengths[index - 1] == length && same >= 3 {
            let count = same.min(6);
            runs.put(writer, Some(32));
            writer.put(count as u64 - 3, 2);
            index += count;
        } else {
            runs.put(writer, Some(length as i64));
            index += 1;
        }
    }
    writer.align();
    CodeTable::of_lengths(lengths)
}

pub fn text_huffman_flags(first_s: u16, s_delta: u16, strip_delta: u16, refinement: [u16; 4], size: u16) -> u16 {
    first_s
        | s_delta << 2
        | strip_delta << 4
        | refinement[0] << 6
        | refinement[1] << 8
        | refinement[2] << 10
        | refinement[3] << 12
        | size << 14
}

pub struct HuffmanTextRegion<'a> {
    pub width: usize,
    pub height: usize,
    pub symbols: &'a [Picture],
    pub placed: &'a [Placed<'a>],
    pub layout: &'a TextLayout,
    pub refine: bool,
    pub huffman_flags: u16,
    pub codes: &'a TextCodes,
    pub id_lengths: &'a [u32],
}

pub fn huffman_text_region(region: &HuffmanTextRegion) -> Vec<u8> {
    let layout = region.layout;
    let mut data = region_information(region.width, region.height, 0, 0, 0);
    let flags: u16 = 1
        | (region.refine as u16) << 1
        | (layout.strips.trailing_zeros() as u16) << 2
        | (layout.corner as u16) << 4
        | (layout.transposed as u16) << 6
        | ((layout.s_offset & 0x1f) as u16) << 10
        | 1 << 15;
    data.extend(flags.to_be_bytes());
    data.extend(region.huffman_flags.to_be_bytes());
    data.extend((region.placed.len() as u32).to_be_bytes());
    let mut writer = BitWriter::default();
    let ids = IdCodes::Coded(symbol_id_table(&mut writer, region.id_lengths));
    let mut contexts = refinement_contexts(1);
    let mut sink = HuffmanSink {
        writer: &mut writer,
        codes: region.codes,
        ids: &ids,
        refine: region.refine,
        contexts: &mut contexts,
    };
    encode_text(&mut sink, region.symbols, region.placed, layout, 1);
    data.extend(writer.finish());
    data
}

pub enum Collective<'a> {
    Stored,
    Mmr(&'a [u8]),
}

pub fn huffman_symbol_dictionary(
    new: &[Picture],
    collectives: &[Collective],
    exported: &[bool],
    table_flags: u16,
    (height_codes, width_codes): (&CodeTable, &CodeTable),
) -> Vec<u8> {
    let flags: u16 = 1 | table_flags;
    let mut data = flags.to_be_bytes().to_vec();
    data.extend((exported.iter().filter(|flag| **flag).count() as u32).to_be_bytes());
    data.extend((new.len() as u32).to_be_bytes());
    let mut writer = BitWriter::default();
    let size_codes = CodeTable::standard(1);
    let mut height = 0i64;
    let mut index = 0;
    let mut class = 0;
    while index < new.len() {
        let class_height = new[index].height as i64;
        height_codes.put(&mut writer, Some(class_height - height));
        height = class_height;
        let mut width = 0i64;
        let start = index;
        while index < new.len() && new[index].height as i64 == height {
            width_codes.put(&mut writer, Some(new[index].width as i64 - width));
            width = new[index].width as i64;
            index += 1;
        }
        width_codes.put(&mut writer, None);
        match collectives[class] {
            Collective::Stored => {
                size_codes.put(&mut writer, Some(0));
                writer.append(&stored_bitmap(&collective_picture(&new[start..index])));
            }
            Collective::Mmr(coded) => {
                size_codes.put(&mut writer, Some(coded.len() as i64));
                writer.append(coded);
            }
        }
        class += 1;
    }
    export_runs(&mut writer, exported);
    data.extend(writer.finish());
    data
}

pub fn collective_picture(symbols: &[Picture]) -> Picture {
    let mut collective = Picture::blank(symbols.iter().map(|symbol| symbol.width).sum(), symbols[0].height);
    let mut left = 0;
    for symbol in symbols {
        collective.stamp(symbol, left, 0);
        left += symbol.width as i64;
    }
    collective
}

pub fn stored_bitmap(picture: &Picture) -> Vec<u8> {
    let mut writer = BitWriter::default();
    for y in 0..picture.height as i64 {
        for x in 0..picture.width as i64 {
            writer.put(picture.get(x, y) as u64, 1);
        }
        writer.align();
    }
    writer.finish()
}

fn export_runs(writer: &mut BitWriter, exported: &[bool]) {
    let runs = CodeTable::standard(1);
    let mut flag = false;
    let mut index = 0;
    while index < exported.len() {
        let run = exported[index..].iter().take_while(|exporting| **exporting == flag).count();
        runs.put(writer, Some(run as i64));
        index += run;
        flag = !flag;
    }
}

pub enum RefinedSymbol<'a> {
    Refined { picture: &'a Picture, of: usize, offset: (i64, i64) },
    Aggregate { picture: &'a Picture, parts: Vec<Placed<'a>> },
}

impl RefinedSymbol<'_> {
    fn picture(&self) -> &Picture {
        match self {
            RefinedSymbol::Refined { picture, .. } | RefinedSymbol::Aggregate { picture, .. } => picture,
        }
    }
}

pub fn huffman_refined_dictionary(known: &[Picture], new: &[RefinedSymbol], exported: &[bool]) -> Vec<u8> {
    let flags: u16 = 1 | 2 | 1 << 12;
    let mut data = flags.to_be_bytes().to_vec();
    data.extend((exported.iter().filter(|flag| **flag).count() as u32).to_be_bytes());
    data.extend((new.len() as u32).to_be_bytes());
    let id_bits = usize::BITS - (known.len() + new.len()).saturating_sub(1).leading_zeros();
    let (heights, widths, counts) = (CodeTable::standard(4), CodeTable::standard(2), CodeTable::standard(1));
    let codes = TextCodes::aggregates();
    let ids = IdCodes::Fixed(id_bits);
    let mut contexts = refinement_contexts(1);
    let mut symbols: Vec<Picture> = known.to_vec();
    let mut writer = BitWriter::default();
    let mut height = 0i64;
    let mut index = 0;
    while index < new.len() {
        let class_height = new[index].picture().height as i64;
        heights.put(&mut writer, Some(class_height - height));
        height = class_height;
        let mut width = 0i64;
        while index < new.len() && new[index].picture().height as i64 == height {
            let picture = new[index].picture();
            widths.put(&mut writer, Some(picture.width as i64 - width));
            width = picture.width as i64;
            match &new[index] {
                RefinedSymbol::Refined { picture, of, offset } => {
                    counts.put(&mut writer, Some(1));
                    writer.put(*of as u64, id_bits);
                    codes.refinement_x.put(&mut writer, Some(offset.0));
                    codes.refinement_y.put(&mut writer, Some(offset.1));
                    let mut encoder = MqEncoder::new();
                    let refinement = crate::support::jbig2_writer::Refinement {
                        picture,
                        reference: &symbols[*of],
                        offset: *offset,
                        template: 1,
                        typical_prediction: false,
                        adaptive: [(-1, -1); 2],
                    };
                    encode_refinement(&mut encoder, &mut contexts, &refinement);
                    let bytes = encoder.finish();
                    codes.refinement_size.put(&mut writer, Some(bytes.len() as i64));
                    writer.append(&bytes);
                }
                RefinedSymbol::Aggregate { parts, .. } => {
                    counts.put(&mut writer, Some(parts.len() as i64));
                    let mut sink = HuffmanSink {
                        writer: &mut writer,
                        codes: &codes,
                        ids: &ids,
                        refine: true,
                        contexts: &mut contexts,
                    };
                    encode_text(&mut sink, &symbols, parts, &TextLayout::plain(), 1);
                }
            }
            symbols.push(picture.clone());
            index += 1;
        }
        widths.put(&mut writer, None);
    }
    export_runs(&mut writer, exported);
    data.extend(writer.finish());
    data
}

pub fn halftone_mmr_region(
    width: usize,
    height: usize,
    (grid_width, grid_height): (usize, usize),
    (x, y): (i32, i32),
    (step_x, step_y): (u16, u16),
    planes: &[Vec<u8>],
) -> Vec<u8> {
    let mut data = region_information(width, height, 0, 0, 0);
    data.push(0x01);
    data.extend((grid_width as u32).to_be_bytes());
    data.extend((grid_height as u32).to_be_bytes());
    data.extend(x.to_be_bytes());
    data.extend(y.to_be_bytes());
    data.extend(step_x.to_be_bytes());
    data.extend(step_y.to_be_bytes());
    for plane in planes {
        data.extend(plane);
    }
    data
}
