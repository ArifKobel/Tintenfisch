use tiny_skia::{ColorU8, IntSize, Pixmap};

#[derive(Clone, Debug, PartialEq)]
pub struct DecodedImage {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

impl DecodedImage {
    pub fn byte_size(&self) -> usize {
        self.rgba.len()
    }

    pub fn to_pixmap(&self) -> Option<Pixmap> {
        let mut pixmap = Pixmap::from_vec(self.rgba.clone(), IntSize::from_wh(self.width as u32, self.height as u32)?)?;
        for pixel in pixmap.pixels_mut() {
            *pixel = ColorU8::from_rgba(pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()).premultiply();
        }
        Some(pixmap)
    }

    pub fn reduced(&self, factor_x: usize, factor_y: usize) -> DecodedImage {
        let (factor_x, factor_y) = (factor_x.max(1), factor_y.max(1));
        let width = self.width.div_ceil(factor_x);
        let height = self.height.div_ceil(factor_y);
        let mut rgba = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let mut sums = [0u32; 4];
                let mut count = 0u32;
                for source_y in y * factor_y..((y + 1) * factor_y).min(self.height) {
                    for source_x in x * factor_x..((x + 1) * factor_x).min(self.width) {
                        let start = (source_y * self.width + source_x) * 4;
                        let pixel = &self.rgba[start..start + 4];
                        let alpha = pixel[3] as u32;
                        for (sum, &value) in sums.iter_mut().zip(&pixel[..3]) {
                            *sum += value as u32 * alpha;
                        }
                        sums[3] += alpha;
                        count += 1;
                    }
                }
                let target = (y * width + x) * 4;
                for (channel, sum) in rgba[target..target + 3].iter_mut().zip(&sums[..3]) {
                    *channel = sum.checked_div(sums[3]).unwrap_or(0) as u8;
                }
                rgba[target + 3] = (sums[3] / count.max(1)) as u8;
            }
        }
        DecodedImage { width, height, rgba }
    }

    pub fn alpha_at(&self, x: usize, y: usize, width: usize, height: usize) -> u8 {
        let source_x = (x * self.width / width.max(1)).min(self.width - 1);
        let source_y = (y * self.height / height.max(1)).min(self.height - 1);
        self.rgba[(source_y * self.width + source_x) * 4 + 3]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduces_by_averaging_and_weights_color_by_alpha() {
        let image = DecodedImage { width: 2, height: 1, rgba: vec![255, 0, 0, 255, 0, 0, 255, 0] };
        let reduced = image.reduced(2, 1);
        assert_eq!((reduced.width, reduced.height), (1, 1));
        assert_eq!(reduced.rgba, vec![255, 0, 0, 127]);
    }
}
