use crate::image::jpx::packet_bits::PacketBits;

const UNKNOWN: u32 = u32::MAX;

pub struct TagTree {
    levels: Vec<(usize, usize)>,
    values: Vec<Vec<u32>>,
    lows: Vec<Vec<u32>>,
}

impl TagTree {
    pub fn new(width: usize, height: usize) -> Self {
        let mut levels = vec![(width, height)];
        while levels.last().is_some_and(|(width, height)| *width > 1 || *height > 1) {
            let (width, height) = *levels.last().unwrap_or(&(1, 1));
            levels.push((width.div_ceil(2), height.div_ceil(2)));
        }
        let values = levels.iter().map(|(width, height)| vec![UNKNOWN; width * height]).collect();
        let lows = levels.iter().map(|(width, height)| vec![0; width * height]).collect();
        Self { levels, values, lows }
    }

    pub fn below(&mut self, bits: &mut PacketBits, x: usize, y: usize, threshold: u32) -> bool {
        let mut low = 0;
        for level in (0..self.levels.len()).rev() {
            let width = self.levels[level].0;
            let index = (y >> level) * width + (x >> level);
            if low > self.lows[level][index] {
                self.lows[level][index] = low;
            } else {
                low = self.lows[level][index];
            }
            while low < threshold && low < self.values[level][index] {
                if bits.bit() == 1 {
                    self.values[level][index] = low;
                } else {
                    low += 1;
                }
            }
            self.lows[level][index] = low;
        }
        self.values[0][y * self.levels[0].0 + x] < threshold
    }

    pub fn value(&mut self, bits: &mut PacketBits, x: usize, y: usize) -> u32 {
        let mut threshold = 1;
        while !self.below(bits, x, y, threshold) {
            threshold += 1;
        }
        threshold - 1
    }
}
