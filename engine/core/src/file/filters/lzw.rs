use crate::error::{Error, Result};

const CLEAR: u16 = 256;
const END_OF_DATA: u16 = 257;
const MAX_CODE_WIDTH: u32 = 12;

struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl BitReader<'_> {
    fn read(&mut self, width: u32) -> Option<u16> {
        if self.bit + width as usize > self.data.len() * 8 {
            return None;
        }
        let mut value = 0u16;
        for _ in 0..width {
            let byte = self.data[self.bit / 8];
            value = value << 1 | ((byte >> (7 - self.bit % 8)) & 1) as u16;
            self.bit += 1;
        }
        Some(value)
    }
}

pub fn decode(data: &[u8], early_change: bool) -> Result<Vec<u8>> {
    let mut table: Vec<Vec<u8>> = (0..=255u8).map(|byte| vec![byte]).collect();
    table.push(Vec::new());
    table.push(Vec::new());
    let mut reader = BitReader { data, bit: 0 };
    let mut width = 9;
    let mut previous: Option<Vec<u8>> = None;
    let mut out = Vec::new();
    while let Some(code) = reader.read(width) {
        match code {
            CLEAR => {
                table.truncate(258);
                width = 9;
                previous = None;
                continue;
            }
            END_OF_DATA => break,
            _ => {}
        }
        let entry = match (table.get(code as usize), &previous) {
            (Some(entry), _) if !(CLEAR..=END_OF_DATA).contains(&code) => entry.clone(),
            (None, Some(previous)) if code as usize == table.len() => {
                let mut entry = previous.clone();
                entry.push(previous[0]);
                entry
            }
            _ => return Err(Error::new(format!("LZWDecode: invalid code {code}"))),
        };
        out.extend_from_slice(&entry);
        if let Some(mut grown) = previous.take() {
            grown.push(entry[0]);
            table.push(grown);
        }
        previous = Some(entry);
        let threshold = table.len() + usize::from(early_change);
        if threshold >= 1 << width && width < MAX_CODE_WIDTH {
            width += 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_example_from_the_pdf_specification() {
        let encoded = [0x80, 0x0b, 0x60, 0x50, 0x22, 0x0c, 0x0c, 0x85, 0x01];
        assert_eq!(decode(&encoded, true).unwrap(), b"-----A---B");
    }

    #[test]
    fn stops_at_the_end_of_input_without_an_end_code() {
        assert_eq!(decode(&[0x80, 0x0b, 0x60], true).unwrap(), b"-");
    }

    #[test]
    fn rejects_a_code_that_is_not_yet_defined() {
        assert!(decode(&[0xff, 0x80], true).is_err());
    }
}
