const STATES: [(u32, u8, u8, bool); 47] = [
    (0x5601, 1, 1, true),
    (0x3401, 2, 6, false),
    (0x1801, 3, 9, false),
    (0x0ac1, 4, 12, false),
    (0x0521, 5, 29, false),
    (0x0221, 38, 33, false),
    (0x5601, 7, 6, true),
    (0x5401, 8, 14, false),
    (0x4801, 9, 14, false),
    (0x3801, 10, 14, false),
    (0x3001, 11, 17, false),
    (0x2401, 12, 18, false),
    (0x1c01, 13, 20, false),
    (0x1601, 29, 21, false),
    (0x5601, 15, 14, true),
    (0x5401, 16, 14, false),
    (0x5101, 17, 15, false),
    (0x4801, 18, 16, false),
    (0x3801, 19, 17, false),
    (0x3401, 20, 18, false),
    (0x3001, 21, 19, false),
    (0x2801, 22, 19, false),
    (0x2401, 23, 20, false),
    (0x2201, 24, 21, false),
    (0x1c01, 25, 22, false),
    (0x1801, 26, 23, false),
    (0x1601, 27, 24, false),
    (0x1401, 28, 25, false),
    (0x1201, 29, 26, false),
    (0x1101, 30, 27, false),
    (0x0ac1, 31, 28, false),
    (0x09c1, 32, 29, false),
    (0x08a1, 33, 30, false),
    (0x0521, 34, 31, false),
    (0x0441, 35, 32, false),
    (0x02a1, 36, 33, false),
    (0x0221, 37, 34, false),
    (0x0141, 38, 35, false),
    (0x0111, 39, 36, false),
    (0x0085, 40, 37, false),
    (0x0049, 41, 38, false),
    (0x0025, 42, 39, false),
    (0x0015, 43, 40, false),
    (0x0009, 44, 41, false),
    (0x0005, 45, 42, false),
    (0x0001, 45, 43, false),
    (0x5601, 46, 46, false),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MqContext {
    pub state: u8,
    pub mps: u8,
}

pub struct MqDecoder<'d> {
    data: &'d [u8],
    position: usize,
    c: u32,
    a: u32,
    ct: u32,
}

impl<'d> MqDecoder<'d> {
    pub fn new(data: &'d [u8]) -> Self {
        let mut decoder = Self { data, position: 0, c: 0, a: 0x8000, ct: 0 };
        decoder.c = (decoder.byte(0) as u32) << 16;
        decoder.byte_in();
        decoder.c <<= 7;
        decoder.ct -= 7;
        decoder
    }

    fn byte(&self, position: usize) -> u8 {
        self.data.get(position).copied().unwrap_or(0xff)
    }

    fn byte_in(&mut self) {
        if self.byte(self.position) == 0xff {
            if self.byte(self.position + 1) > 0x8f {
                self.c += 0xff00;
                self.ct = 8;
            } else {
                self.position += 1;
                self.c += (self.byte(self.position) as u32) << 9;
                self.ct = 7;
            }
        } else {
            self.position += 1;
            self.c += (self.byte(self.position) as u32) << 8;
            self.ct = 8;
        }
    }

    pub fn decode(&mut self, context: &mut MqContext) -> u8 {
        let (qe, next_mps, next_lps, switch) = STATES[context.state as usize];
        self.a -= qe;
        let less_probable = |context: &mut MqContext| {
            let symbol = 1 - context.mps;
            if switch {
                context.mps = symbol;
            }
            context.state = next_lps;
            symbol
        };
        let symbol = if (self.c >> 16) < qe {
            let exchanged = self.a < qe;
            self.a = qe;
            if exchanged {
                context.state = next_mps;
                context.mps
            } else {
                less_probable(context)
            }
        } else {
            self.c -= qe << 16;
            if self.a & 0x8000 != 0 {
                return context.mps;
            }
            if self.a < qe {
                less_probable(context)
            } else {
                context.state = next_mps;
                context.mps
            }
        };
        while self.a & 0x8000 == 0 {
            if self.ct == 0 {
                self.byte_in();
            }
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
        }
        symbol
    }
}
