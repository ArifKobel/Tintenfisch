use crate::image::mq_decoder::{MqContext, MqDecoder};

pub struct IntegerDecoder {
    contexts: Vec<MqContext>,
}

impl IntegerDecoder {
    pub fn new() -> Self {
        Self { contexts: vec![MqContext::default(); 512] }
    }

    pub fn decode(&mut self, decoder: &mut MqDecoder) -> Option<i64> {
        let mut previous = 1usize;
        let mut bit = |decoder: &mut MqDecoder| {
            let value = decoder.decode(&mut self.contexts[previous]) as usize;
            previous = if previous < 256 { previous << 1 | value } else { ((previous << 1 | value) & 511) | 256 };
            value as i64
        };
        let negative = bit(decoder) == 1;
        let (bits, offset) = if bit(decoder) == 0 {
            (2, 0)
        } else if bit(decoder) == 0 {
            (4, 4)
        } else if bit(decoder) == 0 {
            (6, 20)
        } else if bit(decoder) == 0 {
            (8, 84)
        } else if bit(decoder) == 0 {
            (12, 340)
        } else {
            (32, 4436)
        };
        let value = (0..bits).fold(0i64, |value, _| value << 1 | bit(decoder)) + offset;
        match (negative, value) {
            (true, 0) => None,
            (true, value) => Some(-value),
            (false, value) => Some(value),
        }
    }
}

pub struct SymbolIdDecoder {
    length: u32,
    contexts: Vec<MqContext>,
}

impl SymbolIdDecoder {
    pub fn new(length: u32) -> Self {
        Self { length, contexts: vec![MqContext::default(); 1 << (length + 1)] }
    }

    pub fn decode(&mut self, decoder: &mut MqDecoder) -> usize {
        let mut previous = 1usize;
        for _ in 0..self.length {
            previous = previous << 1 | decoder.decode(&mut self.contexts[previous]) as usize;
        }
        previous - (1 << self.length)
    }
}

pub fn symbol_code_length(count: usize) -> u32 {
    usize::BITS - count.saturating_sub(1).leading_zeros()
}
