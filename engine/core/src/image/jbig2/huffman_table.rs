use crate::error::{Error, Result};
use crate::image::bit_reader::BitReader;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reach {
    Range,
    Lower,
    Upper,
    OutOfBand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Line {
    prefix: u8,
    range: u8,
    low: i64,
    reach: Reach,
}

const fn range(prefix: u8, range: u8, low: i64) -> Line {
    Line { prefix, range, low, reach: Reach::Range }
}

const fn lower(prefix: u8, low: i64) -> Line {
    Line { prefix, range: 32, low, reach: Reach::Lower }
}

const fn upper(prefix: u8, low: i64) -> Line {
    Line { prefix, range: 32, low, reach: Reach::Upper }
}

const fn out_of_band(prefix: u8) -> Line {
    Line { prefix, range: 0, low: 0, reach: Reach::OutOfBand }
}

const TABLE_B1: &[Line] = &[range(1, 4, 0), range(2, 8, 16), range(3, 16, 272), upper(3, 65808)];
const TABLE_B2: &[Line] =
    &[range(1, 0, 0), range(2, 0, 1), range(3, 0, 2), range(4, 3, 3), range(5, 6, 11), upper(6, 75), out_of_band(6)];
const TABLE_B3: &[Line] = &[
    range(8, 8, -256),
    range(1, 0, 0),
    range(2, 0, 1),
    range(3, 0, 2),
    range(4, 3, 3),
    range(5, 6, 11),
    lower(8, -257),
    upper(7, 75),
    out_of_band(6),
];
const TABLE_B4: &[Line] =
    &[range(1, 0, 1), range(2, 0, 2), range(3, 0, 3), range(4, 3, 4), range(5, 6, 12), upper(5, 76)];
const TABLE_B5: &[Line] = &[
    range(7, 8, -255),
    range(1, 0, 1),
    range(2, 0, 2),
    range(3, 0, 3),
    range(4, 3, 4),
    range(5, 6, 12),
    lower(7, -256),
    upper(6, 76),
];
const TABLE_B6: &[Line] = &[
    range(5, 10, -2048),
    range(4, 9, -1024),
    range(4, 8, -512),
    range(4, 7, -256),
    range(5, 6, -128),
    range(5, 5, -64),
    range(4, 5, -32),
    range(2, 7, 0),
    range(3, 7, 128),
    range(3, 8, 256),
    range(4, 9, 512),
    range(4, 10, 1024),
    lower(6, -2049),
    upper(6, 2048),
];
const TABLE_B7: &[Line] = &[
    range(4, 9, -1024),
    range(3, 8, -512),
    range(4, 7, -256),
    range(5, 6, -128),
    range(5, 5, -64),
    range(4, 5, -32),
    range(4, 5, 0),
    range(5, 5, 32),
    range(5, 6, 64),
    range(4, 7, 128),
    range(3, 8, 256),
    range(3, 9, 512),
    range(3, 10, 1024),
    lower(5, -1025),
    upper(5, 2048),
];
const TABLE_B8: &[Line] = &[
    range(8, 3, -15),
    range(9, 1, -7),
    range(8, 1, -5),
    range(9, 0, -3),
    range(7, 0, -2),
    range(4, 0, -1),
    range(2, 1, 0),
    range(5, 0, 2),
    range(6, 0, 3),
    range(3, 4, 4),
    range(6, 1, 20),
    range(4, 4, 22),
    range(4, 5, 38),
    range(5, 6, 70),
    range(5, 7, 134),
    range(6, 7, 262),
    range(7, 8, 390),
    range(6, 10, 646),
    lower(9, -16),
    upper(9, 1670),
    out_of_band(2),
];
const TABLE_B9: &[Line] = &[
    range(8, 4, -31),
    range(9, 2, -15),
    range(8, 2, -11),
    range(9, 1, -7),
    range(7, 1, -5),
    range(4, 1, -3),
    range(3, 1, -1),
    range(3, 1, 1),
    range(5, 1, 3),
    range(6, 1, 5),
    range(3, 5, 7),
    range(6, 2, 39),
    range(4, 5, 43),
    range(4, 6, 75),
    range(5, 7, 139),
    range(5, 8, 267),
    range(6, 8, 523),
    range(7, 9, 779),
    range(6, 11, 1291),
    lower(9, -32),
    upper(9, 3339),
    out_of_band(2),
];
const TABLE_B10: &[Line] = &[
    range(7, 4, -21),
    range(8, 0, -5),
    range(7, 0, -4),
    range(5, 0, -3),
    range(2, 2, -2),
    range(5, 0, 2),
    range(6, 0, 3),
    range(7, 0, 4),
    range(8, 0, 5),
    range(2, 6, 6),
    range(5, 5, 70),
    range(6, 5, 102),
    range(6, 6, 134),
    range(6, 7, 198),
    range(6, 8, 326),
    range(6, 9, 582),
    range(6, 10, 1094),
    range(7, 11, 2118),
    lower(8, -22),
    upper(8, 4166),
    out_of_band(2),
];
const TABLE_B11: &[Line] = &[
    range(1, 0, 1),
    range(2, 1, 2),
    range(4, 0, 4),
    range(4, 1, 5),
    range(5, 1, 7),
    range(5, 2, 9),
    range(6, 2, 13),
    range(7, 2, 17),
    range(7, 3, 21),
    range(7, 4, 29),
    range(7, 5, 45),
    range(7, 6, 77),
    upper(7, 141),
];
const TABLE_B12: &[Line] = &[
    range(1, 0, 1),
    range(2, 0, 2),
    range(3, 1, 3),
    range(5, 0, 5),
    range(5, 1, 6),
    range(6, 1, 8),
    range(7, 0, 10),
    range(7, 1, 11),
    range(7, 2, 13),
    range(7, 3, 17),
    range(7, 4, 25),
    range(8, 5, 41),
    upper(8, 73),
];
const TABLE_B13: &[Line] = &[
    range(1, 0, 1),
    range(3, 0, 2),
    range(4, 0, 3),
    range(5, 0, 4),
    range(4, 1, 5),
    range(3, 3, 7),
    range(6, 1, 15),
    range(6, 2, 17),
    range(6, 3, 21),
    range(6, 4, 29),
    range(6, 5, 45),
    range(7, 6, 77),
    upper(7, 141),
];
const TABLE_B14: &[Line] = &[range(3, 0, -2), range(3, 0, -1), range(1, 0, 0), range(3, 0, 1), range(3, 0, 2)];
const TABLE_B15: &[Line] = &[
    range(7, 4, -24),
    range(6, 2, -8),
    range(5, 1, -4),
    range(4, 0, -2),
    range(3, 0, -1),
    range(1, 0, 0),
    range(3, 0, 1),
    range(4, 0, 2),
    range(5, 1, 3),
    range(6, 2, 5),
    range(7, 4, 9),
    lower(7, -25),
    upper(7, 25),
];

const STANDARD: [&[Line]; 15] = [
    TABLE_B1, TABLE_B2, TABLE_B3, TABLE_B4, TABLE_B5, TABLE_B6, TABLE_B7, TABLE_B8, TABLE_B9, TABLE_B10, TABLE_B11,
    TABLE_B12, TABLE_B13, TABLE_B14, TABLE_B15,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HuffmanTable {
    codes: Vec<(u32, Line)>,
}

impl HuffmanTable {
    pub fn standard(number: usize) -> HuffmanTable {
        HuffmanTable::from_lines(STANDARD[number - 1].to_vec())
    }

    pub fn of_lengths(lengths: &[u8]) -> HuffmanTable {
        HuffmanTable::from_lines(
            lengths.iter().enumerate().map(|(value, length)| range(*length, 0, value as i64)).collect(),
        )
    }

    pub fn from_segment(data: &[u8]) -> Result<HuffmanTable> {
        let short = || Error::new("a JBIG2 code table segment is cut short");
        let flags = *data.first().ok_or_else(short)?;
        let number = |at: usize| {
            data.get(at..at + 4).map(|bytes| i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i64)
        };
        let (low, high) = (number(1).ok_or_else(short)?, number(5).ok_or_else(short)?);
        let (prefix_bits, range_bits) = (((flags >> 1) & 7) as usize + 1, ((flags >> 4) & 7) as usize + 1);
        let mut reader = BitReader::new(&data[9..]);
        let mut lines = Vec::new();
        let mut current = low;
        while current < high {
            let prefix = reader.read(prefix_bits) as u8;
            let bits = reader.read(range_bits) as u8;
            if bits > 32 {
                return Err(Error::new("a JBIG2 code table has a range longer than 32 bits"));
            }
            lines.push(range(prefix, bits, current));
            current += 1i64 << bits;
            if reader.is_at_end() {
                return Err(short());
            }
        }
        lines.push(lower(reader.read(prefix_bits) as u8, low - 1));
        lines.push(upper(reader.read(prefix_bits) as u8, high));
        if flags & 1 != 0 {
            lines.push(out_of_band(reader.read(prefix_bits) as u8));
        }
        Ok(HuffmanTable::from_lines(lines))
    }

    fn from_lines(lines: Vec<Line>) -> HuffmanTable {
        let longest = lines.iter().map(|line| line.prefix as usize).max().unwrap_or(0);
        let mut counts = vec![0u32; longest + 1];
        for line in &lines {
            counts[line.prefix as usize] += 1;
        }
        counts[0] = 0;
        let mut next = vec![0u32; longest + 1];
        for length in 1..=longest {
            next[length] = (next[length - 1] + counts[length - 1]) << 1;
        }
        let codes = lines
            .into_iter()
            .filter(|line| line.prefix > 0)
            .map(|line| {
                let code = next[line.prefix as usize];
                next[line.prefix as usize] += 1;
                (code, line)
            })
            .collect();
        HuffmanTable { codes }
    }

    pub fn decode(&self, reader: &mut BitReader) -> Result<Option<i64>> {
        let mut code = 0u32;
        for length in 1..=32u8 {
            code = code << 1 | reader.read(1);
            let found = self.codes.iter().find(|(candidate, line)| line.prefix == length && *candidate == code);
            if let Some((_, line)) = found {
                let offset = reader.read(line.range as usize) as i64;
                return Ok(match line.reach {
                    Reach::Range | Reach::Upper => Some(line.low + offset),
                    Reach::Lower => Some(line.low - offset),
                    Reach::OutOfBand => None,
                });
            }
            if reader.is_at_end() {
                break;
            }
        }
        Err(Error::new("a JBIG2 Huffman code is not in its table"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN_CODES: [&[u32]; 15] = [
        &[0x0, 0x2, 0x6, 0x7],
        &[0x0, 0x2, 0x6, 0xe, 0x1e, 0x3e, 0x3f],
        &[0xfe, 0x0, 0x2, 0x6, 0xe, 0x1e, 0xff, 0x7e, 0x3e],
        &[0x0, 0x2, 0x6, 0xe, 0x1e, 0x1f],
        &[0x7e, 0x0, 0x2, 0x6, 0xe, 0x1e, 0x7f, 0x3e],
        &[0x1c, 0x8, 0x9, 0xa, 0x1d, 0x1e, 0xb, 0x0, 0x2, 0x3, 0xc, 0xd, 0x3e, 0x3f],
        &[0x8, 0x0, 0x9, 0x1a, 0x1b, 0xa, 0xb, 0x1c, 0x1d, 0xc, 0x1, 0x2, 0x3, 0x1e, 0x1f],
        &[
            0xfc, 0x1fc, 0xfd, 0x1fd, 0x7c, 0xa, 0x0, 0x1a, 0x3a, 0x4, 0x3b, 0xb, 0xc, 0x1b, 0x1c, 0x3c, 0x7d, 0x3d,
            0x1fe, 0x1ff, 0x1,
        ],
        &[
            0xfc, 0x1fc, 0xfd, 0x1fd, 0x7c, 0xa, 0x2, 0x3, 0x1a, 0x3a, 0x4, 0x3b, 0xb, 0xc, 0x1b, 0x1c, 0x3c, 0x7d,
            0x3d, 0x1fe, 0x1ff, 0x0,
        ],
        &[
            0x7a, 0xfc, 0x7b, 0x18, 0x0, 0x19, 0x36, 0x7c, 0xfd, 0x1, 0x1a, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x7d,
            0xfe, 0xff, 0x2,
        ],
        &[0x0, 0x2, 0xc, 0xd, 0x1c, 0x1d, 0x3c, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e, 0x7f],
        &[0x0, 0x2, 0x6, 0x1c, 0x1d, 0x3c, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e, 0xfe, 0xff],
        &[0x0, 0x4, 0xc, 0x1c, 0xd, 0x5, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x7e, 0x7f],
        &[0x4, 0x5, 0x0, 0x6, 0x7],
        &[0x7c, 0x3c, 0x1c, 0xc, 0x4, 0x0, 0x5, 0xd, 0x1d, 0x3d, 0x7d, 0x7e, 0x7f],
    ];

    #[test]
    fn the_standard_tables_get_the_prefix_codes_annex_b_lists() {
        for (number, expected) in KNOWN_CODES.iter().enumerate() {
            let codes: Vec<u32> = HuffmanTable::standard(number + 1).codes.iter().map(|(code, _)| *code).collect();
            assert_eq!(&codes, expected, "table B.{}", number + 1);
        }
    }

    #[test]
    fn the_ranges_of_each_standard_table_follow_each_other_without_gaps() {
        for (number, lines) in STANDARD.iter().enumerate() {
            let mut ranges: Vec<&Line> = lines.iter().filter(|line| line.reach == Reach::Range).collect();
            ranges.sort_by_key(|line| line.low);
            for pair in ranges.windows(2) {
                assert_eq!(pair[0].low + (1 << pair[0].range), pair[1].low, "table B.{}", number + 1);
            }
            let last = ranges.last().unwrap();
            if let Some(upper) = lines.iter().find(|line| line.reach == Reach::Upper) {
                assert_eq!(upper.low, last.low + (1 << last.range), "table B.{}", number + 1);
            }
            if let Some(lower) = lines.iter().find(|line| line.reach == Reach::Lower) {
                assert_eq!(lower.low, ranges[0].low - 1, "table B.{}", number + 1);
            }
        }
    }

    #[test]
    fn decodes_ranges_both_open_ends_and_out_of_band() {
        let table = HuffmanTable::standard(3);
        let mut bits = Vec::new();
        let mut push = |code: u32, length: usize| bits.extend((0..length).rev().map(|bit| (code >> bit) & 1));
        push(0x1e, 5);
        push(5, 6);
        push(0xff, 8);
        push(3, 32);
        push(0x7e, 7);
        push(9, 32);
        push(0x3e, 6);
        push(0xfe, 8);
        push(1, 8);
        let bytes: Vec<u8> = bits
            .chunks(8)
            .map(|chunk| chunk.iter().enumerate().fold(0u8, |byte, (index, bit)| byte | (*bit as u8) << (7 - index)))
            .collect();
        let mut reader = BitReader::new(&bytes);
        let values: Vec<Option<i64>> = (0..5).map(|_| table.decode(&mut reader).unwrap()).collect();
        assert_eq!(values, [Some(16), Some(-260), Some(84), None, Some(-255)]);
    }

    #[test]
    fn reads_a_code_table_segment() {
        let mut bits = Vec::new();
        let mut push = |value: u32, length: usize| bits.extend((0..length).rev().map(|bit| (value >> bit) & 1));
        for (prefix, range) in [(1, 1), (2, 0), (3, 2)] {
            push(prefix, 3);
            push(range, 2);
        }
        push(4, 3);
        push(4, 3);
        push(3, 3);
        let mut data = vec![1 | (2 << 1) | (1 << 4)];
        data.extend(10i32.to_be_bytes());
        data.extend(17i32.to_be_bytes());
        data.extend(
            bits.chunks(8).map(|chunk| {
                chunk.iter().enumerate().fold(0u8, |byte, (index, bit)| byte | (*bit as u8) << (7 - index))
            }),
        );
        let table = HuffmanTable::from_segment(&data).unwrap();
        let lines: Vec<(u8, u8, i64, Reach)> =
            table.codes.iter().map(|(_, line)| (line.prefix, line.range, line.low, line.reach)).collect();
        assert_eq!(
            lines,
            [
                (1, 1, 10, Reach::Range),
                (2, 0, 12, Reach::Range),
                (3, 2, 13, Reach::Range),
                (4, 32, 9, Reach::Lower),
                (4, 32, 17, Reach::Upper),
                (3, 0, 0, Reach::OutOfBand),
            ]
        );
    }
}
