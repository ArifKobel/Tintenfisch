pub struct PacketBits<'d> {
    data: &'d [u8],
    position: usize,
    byte: u8,
    left: u8,
}

impl<'d> PacketBits<'d> {
    pub fn new(data: &'d [u8]) -> Self {
        Self { data, position: 0, byte: 0, left: 0 }
    }

    pub fn bit(&mut self) -> u32 {
        if self.left == 0 {
            let stuffed = self.byte == 0xff;
            self.byte = self.data.get(self.position).copied().unwrap_or(0xff);
            self.position += 1;
            self.left = if stuffed { 7 } else { 8 };
        }
        self.left -= 1;
        ((self.byte >> self.left) & 1) as u32
    }

    pub fn bits(&mut self, count: u32) -> u32 {
        (0..count).fold(0, |value, _| value << 1 | self.bit())
    }

    pub fn align(&mut self) {
        self.left = 0;
        if self.byte == 0xff {
            self.position += 1;
            self.byte = 0;
        }
    }

    pub fn position(&self) -> usize {
        self.position
    }
}
