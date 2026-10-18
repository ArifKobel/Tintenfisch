use crate::image::ccitt::{FaxDecoder, FaxParams};
use crate::image::jbig2::bitmap::{Bitmap, Combination};
use crate::image::jbig2::generic_region::GenericRegion;
use crate::image::jbig2::integer_decoder::symbol_code_length;
use crate::image::mq_decoder::MqDecoder;

pub struct HalftoneRegion<'p> {
    pub mmr: bool,
    pub width: usize,
    pub height: usize,
    pub template: u8,
    pub skip_outside: bool,
    pub combination: Combination,
    pub default_black: bool,
    pub grid_width: usize,
    pub grid_height: usize,
    pub grid_x: i64,
    pub grid_y: i64,
    pub step_x: i64,
    pub step_y: i64,
    pub patterns: &'p [Bitmap],
}

impl HalftoneRegion<'_> {
    pub fn decode(&self, data: &[u8]) -> Bitmap {
        let mut region = Bitmap::new(self.width, self.height, self.default_black);
        let Some(first) = self.patterns.first() else { return region };
        let (pattern_width, pattern_height) = (first.width as i64, first.height as i64);
        let cell = |row: usize, column: usize| {
            let (row, column) = (row as i64, column as i64);
            (
                (self.grid_x + row * self.step_y + column * self.step_x) >> 8,
                (self.grid_y + row * self.step_x - column * self.step_y) >> 8,
            )
        };
        let skip = self.skip_outside.then(|| {
            let mut skip = Bitmap::new(self.grid_width, self.grid_height, false);
            for row in 0..self.grid_height {
                for column in 0..self.grid_width {
                    let (x, y) = cell(row, column);
                    if x + pattern_width <= 0
                        || x >= self.width as i64
                        || y + pattern_height <= 0
                        || y >= self.height as i64
                    {
                        skip.set(column, row, 1);
                    }
                }
            }
            skip
        });
        let planes = symbol_code_length(self.patterns.len()) as usize;
        let adaptive: Vec<(i64, i64)> = vec![(if self.template <= 1 { 3 } else { 2 }, -1), (-3, -1), (2, -2), (-2, -2)];
        let mut contexts = GenericRegion::contexts(self.template);
        let mut decoder = MqDecoder::new(data);
        let mut remaining = data;
        let mut gray = vec![0usize; self.grid_width * self.grid_height];
        let mut previous: Option<Bitmap> = None;
        for plane in (0..planes).rev() {
            let mut bits = if self.mmr {
                let params = FaxParams {
                    k: -1,
                    columns: self.grid_width,
                    rows: Some(self.grid_height),
                    byte_aligned: false,
                    end_of_line: false,
                    black_is_1: true,
                };
                let (pixels, used) = FaxDecoder::new(params).decode_measured(remaining);
                remaining = remaining.get(used..).unwrap_or_default();
                Bitmap::from_packed(self.grid_width, self.grid_height, pixels)
            } else {
                let generic = GenericRegion {
                    width: self.grid_width,
                    height: self.grid_height,
                    template: self.template,
                    typical_prediction: false,
                    adaptive: adaptive.clone(),
                    skip: skip.as_ref(),
                };
                generic.decode(&mut decoder, &mut contexts)
            };
            for row in 0..self.grid_height {
                for column in 0..self.grid_width {
                    let (x, y) = (column as i64, row as i64);
                    let bit = bits.get(x, y) ^ previous.as_ref().map_or(0, |previous| previous.get(x, y));
                    bits.set(column, row, bit);
                    gray[row * self.grid_width + column] |= (bit as usize) << plane;
                }
            }
            previous = Some(bits);
        }
        for row in 0..self.grid_height {
            for column in 0..self.grid_width {
                let value = gray[row * self.grid_width + column].min(self.patterns.len() - 1);
                let (x, y) = cell(row, column);
                region.compose(&self.patterns[value], x, y, self.combination);
            }
        }
        region
    }
}
