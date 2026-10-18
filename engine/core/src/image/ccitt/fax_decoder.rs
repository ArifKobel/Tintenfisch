use std::collections::HashMap;

use crate::image::bit_reader::BitReader;
use crate::image::ccitt::fax_codes::{BLACK_RUNS, WHITE_RUNS};
use crate::image::ccitt::fax_params::FaxParams;

const EOL: u32 = 1;
const EOL_BITS: usize = 12;
const LONGEST_RUN_CODE: u8 = 13;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    White,
    Black,
}

impl Color {
    fn other(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

enum Mode {
    Pass,
    Horizontal,
    Vertical(i64),
}

pub struct FaxDecoder {
    params: FaxParams,
    white: HashMap<(u8, u16), u16>,
    black: HashMap<(u8, u16), u16>,
}

impl FaxDecoder {
    pub fn new(params: FaxParams) -> Self {
        let table = |runs: &[(u8, u16, u16)]| runs.iter().map(|(length, code, run)| ((*length, *code), *run)).collect();
        Self { params, white: table(&WHITE_RUNS), black: table(&BLACK_RUNS) }
    }

    pub fn decode(&self, data: &[u8]) -> (Vec<u8>, usize) {
        self.decode_from(&mut BitReader::new(data))
    }

    pub fn decode_measured(&self, data: &[u8]) -> (Vec<u8>, usize) {
        let mut reader = BitReader::new(data);
        let (pixels, _) = self.decode_from(&mut reader);
        if reader.peek(2 * EOL_BITS) == EOL << EOL_BITS | EOL {
            reader.consume(2 * EOL_BITS);
        }
        reader.align();
        (pixels, reader.position() / 8)
    }

    fn decode_from(&self, reader: &mut BitReader) -> (Vec<u8>, usize) {
        let columns = self.params.columns;
        let mut changes: Vec<Vec<usize>> = Vec::new();
        let mut reference: Vec<usize> = Vec::new();
        while self.params.rows.is_none_or(|rows| changes.len() < rows) {
            if self.params.byte_aligned && (self.params.k < 0 || !self.params.end_of_line) {
                reader.align();
            }
            Self::skip_end_of_line(reader);
            if reader.is_at_end() {
                break;
            }
            let one_dimensional = match self.params.k {
                k if k < 0 => false,
                0 => true,
                _ => reader.read(1) == 1,
            };
            let row = if one_dimensional { self.row_1d(reader) } else { self.row_2d(reader, &reference) };
            let Some(row) = row else { break };
            reference = row.clone();
            changes.push(row);
        }
        let rows = self.params.rows.unwrap_or(changes.len());
        let row_bytes = columns.div_ceil(8);
        let (paper, ink) = if self.params.black_is_1 { (0x00, 0xff) } else { (0xff, 0x00) };
        let mut pixels = vec![paper; row_bytes * rows];
        for (row, transitions) in changes.iter().enumerate().take(rows) {
            let line = &mut pixels[row * row_bytes..(row + 1) * row_bytes];
            for pair in transitions.chunks(2) {
                let (start, end) = (pair[0], pair.get(1).copied().unwrap_or(columns));
                for x in start..end.min(columns) {
                    let mask = 0x80u8 >> (x % 8);
                    line[x / 8] = (line[x / 8] & !mask) | (ink & mask);
                }
            }
        }
        (pixels, rows)
    }

    fn skip_end_of_line(reader: &mut BitReader) {
        while reader.peek(EOL_BITS) == 0 && !reader.is_at_end() {
            reader.consume(1);
        }
        if reader.peek(EOL_BITS) == EOL {
            reader.consume(EOL_BITS);
        }
    }

    fn row_1d(&self, reader: &mut BitReader) -> Option<Vec<usize>> {
        let columns = self.params.columns;
        let mut changes = Vec::new();
        let (mut position, mut color) = (0, Color::White);
        while position < columns {
            position = (position + self.run(reader, color)?).min(columns);
            changes.push(position);
            color = color.other();
        }
        Some(changes)
    }

    fn row_2d(&self, reader: &mut BitReader, reference: &[usize]) -> Option<Vec<usize>> {
        let columns = self.params.columns;
        let mut changes: Vec<usize> = Vec::new();
        let (mut a0, mut color): (i64, Color) = (-1, Color::White);
        while a0 < columns as i64 {
            let (b1, b2) = changing_elements(reference, a0, color, columns);
            match mode(reader)? {
                Mode::Pass => a0 = b2 as i64,
                Mode::Horizontal => {
                    let start = a0.max(0) as usize;
                    let a1 = (start + self.run(reader, color)?).min(columns);
                    let a2 = (a1 + self.run(reader, color.other())?).min(columns);
                    changes.extend([a1, a2]);
                    a0 = a2 as i64;
                }
                Mode::Vertical(offset) => {
                    let a1 = (b1 as i64 + offset).clamp(0, columns as i64);
                    if changes.last().is_some_and(|last| (*last as i64) > a1) {
                        return None;
                    }
                    changes.push(a1 as usize);
                    a0 = a1;
                    color = color.other();
                }
            }
        }
        Some(changes)
    }

    fn run(&self, reader: &mut BitReader, color: Color) -> Option<usize> {
        let table = match color {
            Color::White => &self.white,
            Color::Black => &self.black,
        };
        let mut total = 0;
        loop {
            let run = (2..=LONGEST_RUN_CODE).find_map(|length| {
                table.get(&(length, reader.peek(length as usize) as u16)).map(|run| (length, *run as usize))
            });
            let (length, run) = run?;
            reader.consume(length as usize);
            total += run;
            if run < 64 {
                return Some(total);
            }
        }
    }
}

fn changing_elements(reference: &[usize], a0: i64, color: Color, columns: usize) -> (usize, usize) {
    let parity = match color {
        Color::White => 0,
        Color::Black => 1,
    };
    let index = (0..reference.len()).find(|index| index % 2 == parity && reference[*index] as i64 > a0);
    match index {
        Some(index) => (reference[index], reference.get(index + 1).copied().unwrap_or(columns)),
        None => (columns, columns),
    }
}

fn mode(reader: &mut BitReader) -> Option<Mode> {
    let codes: [(usize, u32, Mode); 9] = [
        (1, 0b1, Mode::Vertical(0)),
        (3, 0b011, Mode::Vertical(1)),
        (3, 0b010, Mode::Vertical(-1)),
        (3, 0b001, Mode::Horizontal),
        (4, 0b0001, Mode::Pass),
        (6, 0b000011, Mode::Vertical(2)),
        (6, 0b000010, Mode::Vertical(-2)),
        (7, 0b0000011, Mode::Vertical(3)),
        (7, 0b0000010, Mode::Vertical(-3)),
    ];
    let (length, _, mode) = codes.into_iter().find(|(length, code, _)| reader.peek(*length) == *code)?;
    reader.consume(length);
    Some(mode)
}
