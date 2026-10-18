use crate::error::{Error, Result};
use crate::function::pdf_function::{Bounds, numbers, pairs};
use crate::syntax::Dict;

#[derive(Clone, Debug, PartialEq)]
pub struct SampledFunction {
    pub bounds: Bounds,
    size: Vec<usize>,
    bits_per_sample: u32,
    encode: Vec<(f64, f64)>,
    decode: Vec<(f64, f64)>,
    samples: Vec<u8>,
}

impl SampledFunction {
    pub fn read(dict: &Dict, bounds: Bounds, samples: Vec<u8>) -> Result<Self> {
        let range = bounds.range.clone().ok_or_else(|| Error::new("sampled function without /Range"))?;
        let size: Vec<usize> =
            numbers(dict.get("Size")).unwrap_or_default().iter().map(|&value| value.max(1.0) as usize).collect();
        if size.len() != bounds.domain.len() {
            return Err(Error::new("sampled function /Size does not match /Domain"));
        }
        let bits_per_sample = dict.int("BitsPerSample").unwrap_or(8) as u32;
        if ![1, 2, 4, 8, 12, 16, 24, 32].contains(&bits_per_sample) {
            return Err(Error::new("sampled function with invalid /BitsPerSample"));
        }
        let encode =
            pairs(dict.get("Encode")).unwrap_or_else(|| size.iter().map(|&count| (0.0, (count - 1) as f64)).collect());
        let decode = pairs(dict.get("Decode")).unwrap_or_else(|| range.clone());
        Ok(Self { bounds, size, bits_per_sample, encode, decode, samples })
    }

    fn outputs(&self) -> usize {
        self.decode.len()
    }

    fn sample(&self, index: usize) -> f64 {
        let bits = self.bits_per_sample as usize;
        let start = index * bits;
        let mut value: u64 = 0;
        for bit in start..start + bits {
            let byte = self.samples.get(bit / 8).copied().unwrap_or(0);
            value = value << 1 | ((byte >> (7 - bit % 8)) & 1) as u64;
        }
        value as f64 / ((1u64 << bits) - 1) as f64
    }

    pub fn evaluate(&self, inputs: &[f64]) -> Vec<f64> {
        let positions: Vec<f64> = inputs
            .iter()
            .enumerate()
            .map(|(dimension, &input)| {
                let (low, high) = self.bounds.domain[dimension];
                let (encode_low, encode_high) = self.encode[dimension];
                let position = if high > low {
                    encode_low + (input - low) * (encode_high - encode_low) / (high - low)
                } else {
                    encode_low
                };
                position.clamp(0.0, (self.size[dimension] - 1) as f64)
            })
            .collect();
        let mut outputs = vec![0.0; self.outputs()];
        for corner in 0..1usize << positions.len() {
            let mut weight = 1.0;
            let mut offset = 0;
            let mut stride = 1;
            for (dimension, &position) in positions.iter().enumerate() {
                let floor = position.floor();
                let fraction = position - floor;
                let upper = corner >> dimension & 1 == 1;
                let index = (floor as usize + usize::from(upper)).min(self.size[dimension] - 1);
                weight *= if upper { fraction } else { 1.0 - fraction };
                offset += index * stride;
                stride *= self.size[dimension];
            }
            if weight == 0.0 {
                continue;
            }
            for (output, value) in outputs.iter_mut().enumerate() {
                *value += weight * self.sample(offset * self.outputs() + output);
            }
        }
        outputs.iter().zip(&self.decode).map(|(value, (low, high))| low + value * (high - low)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Parser;

    fn read(source: &[u8], samples: Vec<u8>) -> SampledFunction {
        let dict = Parser::new(source, 0).parse_object().unwrap().as_dict().unwrap().clone();
        let bounds = Bounds { domain: pairs(dict.get("Domain")).unwrap(), range: pairs(dict.get("Range")) };
        SampledFunction::read(&dict, bounds, samples).unwrap()
    }

    #[test]
    fn interpolates_between_samples_in_one_dimension() {
        let ramp = read(b"<< /Domain [0 1] /Range [0 1] /Size [3] /BitsPerSample 8 >>", vec![0, 255, 0]);
        assert_eq!(ramp.evaluate(&[0.0]), vec![0.0]);
        assert_eq!(ramp.evaluate(&[0.5]), vec![1.0]);
        assert!((ramp.evaluate(&[0.25])[0] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn interpolates_bilinearly_with_several_outputs() {
        let grid = read(
            b"<< /Domain [0 1 0 1] /Range [0 1 0 1] /Size [2 2] /BitsPerSample 8 >>",
            vec![0, 0, 255, 0, 0, 255, 255, 255],
        );
        let center = grid.evaluate(&[0.5, 0.5]);
        assert!((center[0] - 0.5).abs() < 1e-9 && (center[1] - 0.5).abs() < 1e-9);
        assert_eq!(grid.evaluate(&[1.0, 0.0]), vec![1.0, 0.0]);
    }

    #[test]
    fn reads_samples_narrower_than_a_byte() {
        let bits = read(b"<< /Domain [0 1] /Range [0 1] /Size [4] /BitsPerSample 2 >>", vec![0b00_01_10_11]);
        assert!((bits.evaluate(&[1.0 / 3.0])[0] - 1.0 / 3.0).abs() < 1e-9);
    }
}
