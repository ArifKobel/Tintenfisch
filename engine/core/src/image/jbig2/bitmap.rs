#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combination {
    Or,
    And,
    Xor,
    Xnor,
    Replace,
}

impl Combination {
    pub fn from_bits(bits: u8) -> Combination {
        match bits & 7 {
            1 => Combination::And,
            2 => Combination::Xor,
            3 => Combination::Xnor,
            4 => Combination::Replace,
            _ => Combination::Or,
        }
    }

    fn apply(self, page: u8, region: u8) -> u8 {
        match self {
            Combination::Or => page | region,
            Combination::And => page & region,
            Combination::Xor => page ^ region,
            Combination::Xnor => 1 - (page ^ region),
            Combination::Replace => region,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitmap {
    pub width: usize,
    pub height: usize,
    row_bytes: usize,
    data: Vec<u8>,
}

impl Bitmap {
    pub fn new(width: usize, height: usize, black: bool) -> Bitmap {
        let row_bytes = width.div_ceil(8);
        Bitmap { width, height, row_bytes, data: vec![if black { 0xff } else { 0 }; row_bytes * height] }
    }

    pub fn from_packed(width: usize, height: usize, mut data: Vec<u8>) -> Bitmap {
        let row_bytes = width.div_ceil(8);
        data.resize(row_bytes * height, 0);
        Bitmap { width, height, row_bytes, data }
    }

    pub fn get(&self, x: i64, y: i64) -> u8 {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return 0;
        }
        let (x, y) = (x as usize, y as usize);
        (self.data[y * self.row_bytes + x / 8] >> (7 - x % 8)) & 1
    }

    pub fn set(&mut self, x: usize, y: usize, value: u8) {
        let mask = 0x80 >> (x % 8);
        let byte = &mut self.data[y * self.row_bytes + x / 8];
        *byte = if value != 0 { *byte | mask } else { *byte & !mask };
    }

    pub fn copy_row(&mut self, from: usize, to: usize) {
        let (from, to) = (from * self.row_bytes, to * self.row_bytes);
        self.data.copy_within(from..from + self.row_bytes, to);
    }

    pub fn grow(&mut self, height: usize, black: bool) {
        if height > self.height {
            self.data.resize(self.row_bytes * height, if black { 0xff } else { 0 });
            self.height = height;
        }
    }

    pub fn compose(&mut self, region: &Bitmap, left: i64, top: i64, combination: Combination) {
        for y in 0..region.height as i64 {
            let page_y = top + y;
            if page_y < 0 || page_y as usize >= self.height {
                continue;
            }
            for x in 0..region.width as i64 {
                let page_x = left + x;
                if page_x < 0 || page_x as usize >= self.width {
                    continue;
                }
                let value = combination.apply(self.get(page_x, page_y), region.get(x, y));
                self.set(page_x as usize, page_y as usize, value);
            }
        }
    }

    pub fn compose_from(&mut self, source: &Bitmap, left: i64, top: i64) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.set(x, y, source.get(left + x as i64, top + y as i64));
            }
        }
    }

    pub fn into_packed(self) -> Vec<u8> {
        self.data
    }
}
