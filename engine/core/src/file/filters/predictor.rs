use crate::error::{Error, Result};
use crate::syntax::Dict;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Tiff,
    Png,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Predictor {
    kind: Kind,
    colors: usize,
    bits_per_component: usize,
    columns: usize,
}

impl Predictor {
    pub fn from_params(params: &Dict) -> Result<Option<Predictor>> {
        let kind = match params.int("Predictor").unwrap_or(1) {
            1 => return Ok(None),
            2 => Kind::Tiff,
            10..=15 => Kind::Png,
            other => return Err(Error::new(format!("unknown predictor {other}"))),
        };
        let positive = |key: &str, default: i64| {
            usize::try_from(params.int(key).unwrap_or(default))
                .ok()
                .filter(|&value| value > 0)
                .ok_or_else(|| Error::new(format!("invalid predictor {key}")))
        };
        let bits_per_component = positive("BitsPerComponent", 8)?;
        if ![1, 2, 4, 8, 16].contains(&bits_per_component) {
            return Err(Error::new("invalid predictor BitsPerComponent"));
        }
        Ok(Some(Predictor {
            kind,
            colors: positive("Colors", 1)?,
            bits_per_component,
            columns: positive("Columns", 1)?,
        }))
    }

    fn row_bytes(&self) -> usize {
        (self.colors * self.bits_per_component * self.columns).div_ceil(8)
    }

    fn pixel_bytes(&self) -> usize {
        (self.colors * self.bits_per_component).div_ceil(8)
    }

    pub fn reverse(&self, data: &[u8]) -> Vec<u8> {
        match self.kind {
            Kind::Png => self.reverse_png(data),
            Kind::Tiff => self.reverse_tiff(data),
        }
    }

    fn reverse_png(&self, data: &[u8]) -> Vec<u8> {
        let row_bytes = self.row_bytes();
        let pixel_bytes = self.pixel_bytes();
        let mut out = Vec::with_capacity(data.len());
        let mut previous = vec![0u8; row_bytes];
        for encoded in data.chunks(row_bytes + 1) {
            let (filter, body) = encoded.split_first().unwrap_or((&0, &[]));
            let mut row = body.to_vec();
            row.resize(row_bytes, 0);
            for index in 0..row_bytes {
                let left = if index >= pixel_bytes { row[index - pixel_bytes] } else { 0 };
                let up = previous[index];
                let upper_left = if index >= pixel_bytes { previous[index - pixel_bytes] } else { 0 };
                let prediction = match filter {
                    1 => left,
                    2 => up,
                    3 => ((left as u16 + up as u16) / 2) as u8,
                    4 => paeth(left, up, upper_left),
                    _ => 0,
                };
                row[index] = row[index].wrapping_add(prediction);
            }
            out.extend_from_slice(&row[..body.len().min(row_bytes)]);
            previous = row;
        }
        out
    }

    fn reverse_tiff(&self, data: &[u8]) -> Vec<u8> {
        let mut out = data.to_vec();
        let width = self.bits_per_component;
        let mask = if width == 16 { u16::MAX as u32 } else { (1u32 << width) - 1 };
        let samples_per_row = self.colors * self.columns;
        for row in out.chunks_mut(self.row_bytes()) {
            let available = row.len() * 8 / width;
            for sample in self.colors..samples_per_row.min(available) {
                let value = read_sample(row, sample, width) + read_sample(row, sample - self.colors, width);
                write_sample(row, sample, width, value & mask);
            }
        }
        out
    }
}

fn paeth(left: u8, up: u8, upper_left: u8) -> u8 {
    let estimate = left as i16 + up as i16 - upper_left as i16;
    let distance = |value: u8| (estimate - value as i16).abs();
    if distance(left) <= distance(up) && distance(left) <= distance(upper_left) {
        left
    } else if distance(up) <= distance(upper_left) {
        up
    } else {
        upper_left
    }
}

fn read_sample(row: &[u8], index: usize, width: usize) -> u32 {
    (0..width).fold(0, |value, bit| {
        let position = index * width + bit;
        value << 1 | ((row[position / 8] >> (7 - position % 8)) & 1) as u32
    })
}

fn write_sample(row: &mut [u8], index: usize, width: usize, value: u32) {
    for bit in 0..width {
        let position = index * width + bit;
        let mask = 1 << (7 - position % 8);
        if (value >> (width - 1 - bit)) & 1 == 1 {
            row[position / 8] |= mask;
        } else {
            row[position / 8] &= !mask;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{Name, Object};

    fn predictor(entries: &[(&str, i64)]) -> Predictor {
        let params: Dict = entries.iter().map(|(key, value)| (Name::from(*key), Object::Int(*value))).collect();
        Predictor::from_params(&params).unwrap().unwrap()
    }

    #[test]
    fn reverses_png_up_rows_as_used_by_xref_streams() {
        let png = predictor(&[("Predictor", 12), ("Columns", 3)]);
        let encoded = [2, 1, 0, 16, 2, 0, 0, 1, 2, 0, 1, 2];
        assert_eq!(png.reverse(&encoded), vec![1, 0, 16, 1, 0, 17, 1, 1, 19]);
    }

    #[test]
    fn reverses_png_sub_average_and_paeth() {
        let png = predictor(&[("Predictor", 15), ("Columns", 4)]);
        let encoded = [1, 10, 5, 5, 5, 3, 4, 4, 4, 4, 4, 1, 1, 1, 1];
        assert_eq!(png.reverse(&encoded), vec![10, 15, 20, 25, 9, 16, 22, 27, 10, 17, 23, 28]);
    }

    #[test]
    fn reverses_tiff_differences_per_component() {
        let tiff = predictor(&[("Predictor", 2), ("Colors", 2), ("Columns", 3)]);
        assert_eq!(tiff.reverse(&[10, 20, 1, 2, 1, 2]), vec![10, 20, 11, 22, 12, 24]);
        let bits = predictor(&[("Predictor", 2), ("BitsPerComponent", 4), ("Columns", 4)]);
        assert_eq!(bits.reverse(&[0x31, 0x11]), vec![0x34, 0x56]);
    }

    #[test]
    fn treats_predictor_one_as_absent_and_rejects_unknown_ones() {
        assert_eq!(Predictor::from_params(&Dict::new()).unwrap(), None);
        let params: Dict = [(Name::from("Predictor"), Object::Int(7))].into_iter().collect();
        assert!(Predictor::from_params(&params).is_err());
    }
}
