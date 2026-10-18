pub struct BitReader<'d> {
    data: &'d [u8],
    position: usize,
}

impl<'d> BitReader<'d> {
    pub fn new(data: &'d [u8]) -> Self {
        Self { data, position: 0 }
    }

    pub fn peek(&self, count: usize) -> u32 {
        (0..count).fold(0, |value, offset| {
            let position = self.position + offset;
            let bit = self.data.get(position / 8).map_or(0, |byte| (byte >> (7 - position % 8)) & 1);
            value << 1 | bit as u32
        })
    }

    pub fn consume(&mut self, count: usize) {
        self.position += count;
    }

    pub fn read(&mut self, count: usize) -> u32 {
        let value = self.peek(count);
        self.consume(count);
        value
    }

    pub fn align(&mut self) {
        self.position = self.position.div_ceil(8) * 8;
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn rest(&self) -> &'d [u8] {
        self.data.get(self.position.div_ceil(8)..).unwrap_or_default()
    }

    pub fn is_at_end(&self) -> bool {
        self.position >= self.data.len() * 8
    }
}
