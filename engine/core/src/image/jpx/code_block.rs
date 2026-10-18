use crate::image::jpx::packet_bits::PacketBits;
use crate::image::mq_decoder::{MqContext, MqDecoder};

pub const STYLE_BYPASS: u8 = 1;
pub const STYLE_RESET: u8 = 2;
pub const STYLE_TERMINATE_ALL: u8 = 4;
pub const STYLE_VERTICALLY_CAUSAL: u8 = 8;
pub const STYLE_SEGMENTATION: u8 = 32;

const RUN_LENGTH: usize = 17;
const UNIFORM: usize = 18;
const SIGNIFICANT: u8 = 1;
const NEGATIVE: u8 = 2;
const VISITED: u8 = 4;
const REFINED: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Ll,
    Hl,
    Lh,
    Hh,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodeBlockData {
    pub segments: Vec<(Vec<u8>, u32)>,
    pub missing_planes: u32,
    pub included: bool,
}

pub fn segment_capacity(style: u8, first: u32) -> u32 {
    if style & STYLE_TERMINATE_ALL != 0 {
        1
    } else if style & STYLE_BYPASS != 0 {
        if first < 10 {
            10 - first
        } else if (first - 10).is_multiple_of(3) {
            2
        } else {
            1
        }
    } else {
        u32::MAX
    }
}

enum Source<'d> {
    Arithmetic(MqDecoder<'d>),
    Raw(PacketBits<'d>),
}

pub struct CodeBlock {
    width: usize,
    height: usize,
    orientation: Orientation,
    style: u8,
    flags: Vec<u8>,
    magnitudes: Vec<u32>,
    known_plane: Vec<u8>,
    contexts: [MqContext; 19],
}

impl CodeBlock {
    pub fn new(width: usize, height: usize, orientation: Orientation, style: u8) -> Self {
        Self {
            width,
            height,
            orientation,
            style,
            flags: vec![0; width * height],
            magnitudes: vec![0; width * height],
            known_plane: vec![0; width * height],
            contexts: initial_contexts(),
        }
    }

    pub fn decode(mut self, data: &CodeBlockData, magnitude_bits: u32, region_shift: u32) -> Vec<f64> {
        let planes = magnitude_bits.saturating_sub(data.missing_planes);
        if planes > 0 && data.included {
            let last_pass = 1 + 3 * (planes - 1);
            let mut pass = 0u32;
            for (bytes, passes) in &data.segments {
                if pass >= last_pass {
                    break;
                }
                let raw = self.style & STYLE_BYPASS != 0 && pass >= 10 && (pass - 10) % 3 != 2;
                let mut source =
                    if raw { Source::Raw(PacketBits::new(bytes)) } else { Source::Arithmetic(MqDecoder::new(bytes)) };
                for _ in 0..*passes {
                    if pass >= last_pass {
                        break;
                    }
                    let (plane, kind) =
                        if pass == 0 { (planes - 1, 2) } else { (planes - 1 - pass.div_ceil(3), (pass - 1) % 3) };
                    match kind {
                        0 => self.significance_pass(&mut source, plane),
                        1 => self.refinement_pass(&mut source, plane),
                        _ => self.cleanup_pass(&mut source, plane),
                    }
                    if self.style & STYLE_RESET != 0 {
                        self.contexts = initial_contexts();
                    }
                    pass += 1;
                }
            }
        }
        (0..self.width * self.height)
            .map(|index| {
                let (mut magnitude, mut plane) = (self.magnitudes[index], self.known_plane[index] as u32);
                if magnitude == 0 {
                    return 0.0;
                }
                if region_shift > 0 && magnitude >= 1 << region_shift {
                    magnitude >>= region_shift;
                    plane = plane.saturating_sub(region_shift);
                }
                let value = magnitude as f64 + if plane > 0 { (1u64 << (plane - 1)) as f64 } else { 0.0 };
                if self.flags[index] & NEGATIVE != 0 { -value } else { value }
            })
            .collect()
    }

    fn significant(&self, x: i64, y: i64, stripe_end: i64) -> u8 {
        if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
            return 0;
        }
        if self.style & STYLE_VERTICALLY_CAUSAL != 0 && y >= stripe_end {
            return 0;
        }
        self.flags[y as usize * self.width + x as usize] & SIGNIFICANT
    }

    fn sign_of(&self, x: i64, y: i64, stripe_end: i64) -> i32 {
        if self.significant(x, y, stripe_end) == 0 {
            return 0;
        }
        if self.flags[y as usize * self.width + x as usize] & NEGATIVE != 0 { -1 } else { 1 }
    }

    fn zero_context(&self, x: i64, y: i64, stripe_end: i64) -> usize {
        let at = |dx: i64, dy: i64| self.significant(x + dx, y + dy, stripe_end) as u32;
        let horizontal = at(-1, 0) + at(1, 0);
        let vertical = at(0, -1) + at(0, 1);
        let diagonal = at(-1, -1) + at(1, -1) + at(-1, 1) + at(1, 1);
        let (h, v) = match self.orientation {
            Orientation::Hl => (vertical, horizontal),
            _ => (horizontal, vertical),
        };
        if self.orientation == Orientation::Hh {
            let hv = h + v;
            return match (diagonal, hv) {
                (d, _) if d >= 3 => 8,
                (2, hv) if hv >= 1 => 7,
                (2, _) => 6,
                (1, hv) if hv >= 2 => 5,
                (1, 1) => 4,
                (1, _) => 3,
                (_, hv) if hv >= 2 => 2,
                (_, 1) => 1,
                _ => 0,
            };
        }
        match (h, v, diagonal) {
            (2, _, _) => 8,
            (1, v, _) if v >= 1 => 7,
            (1, _, d) if d >= 1 => 6,
            (1, _, _) => 5,
            (_, 2, _) => 4,
            (_, 1, _) => 3,
            (_, _, d) if d >= 2 => 2,
            (_, _, 1) => 1,
            _ => 0,
        }
    }

    fn decode_sign(&mut self, source: &mut Source, x: i64, y: i64, stripe_end: i64) -> bool {
        match source {
            Source::Raw(bits) => bits.bit() == 1,
            Source::Arithmetic(decoder) => {
                let horizontal = (self.sign_of(x - 1, y, stripe_end) + self.sign_of(x + 1, y, stripe_end)).clamp(-1, 1);
                let vertical = (self.sign_of(x, y - 1, stripe_end) + self.sign_of(x, y + 1, stripe_end)).clamp(-1, 1);
                let (context, flip) = match (horizontal, vertical) {
                    (1, 1) => (13, 0),
                    (1, 0) => (12, 0),
                    (1, -1) => (11, 0),
                    (0, 1) => (10, 0),
                    (0, 0) => (9, 0),
                    (0, -1) => (10, 1),
                    (-1, 1) => (11, 1),
                    (-1, 0) => (12, 1),
                    _ => (13, 1),
                };
                decoder.decode(&mut self.contexts[context]) ^ flip == 1
            }
        }
    }

    fn become_significant(&mut self, source: &mut Source, x: i64, y: i64, stripe_end: i64, plane: u32) {
        let negative = self.decode_sign(source, x, y, stripe_end);
        let index = y as usize * self.width + x as usize;
        self.flags[index] |= SIGNIFICANT | if negative { NEGATIVE } else { 0 };
        self.magnitudes[index] |= 1 << plane;
        self.known_plane[index] = plane as u8;
    }

    fn bit(&mut self, source: &mut Source, context: usize) -> u32 {
        match source {
            Source::Raw(bits) => bits.bit(),
            Source::Arithmetic(decoder) => decoder.decode(&mut self.contexts[context]) as u32,
        }
    }

    fn stripes(&self) -> impl Iterator<Item = (i64, i64)> + use<> {
        let height = self.height as i64;
        (0..height).step_by(4).map(move |top| (top, (top + 4).min(height)))
    }

    fn significance_pass(&mut self, source: &mut Source, plane: u32) {
        for (top, end) in self.stripes() {
            for x in 0..self.width as i64 {
                for y in top..end {
                    let index = y as usize * self.width + x as usize;
                    if self.flags[index] & SIGNIFICANT != 0 {
                        continue;
                    }
                    let context = self.zero_context(x, y, end);
                    if context == 0 {
                        continue;
                    }
                    self.flags[index] |= VISITED;
                    if self.bit(source, context) == 1 {
                        self.become_significant(source, x, y, end, plane);
                    }
                }
            }
        }
    }

    fn refinement_pass(&mut self, source: &mut Source, plane: u32) {
        for (top, end) in self.stripes() {
            for x in 0..self.width as i64 {
                for y in top..end {
                    let index = y as usize * self.width + x as usize;
                    if self.flags[index] & (SIGNIFICANT | VISITED) != SIGNIFICANT {
                        continue;
                    }
                    let context = if self.flags[index] & REFINED != 0 {
                        16
                    } else if self.any_neighbour(x, y, end) {
                        15
                    } else {
                        14
                    };
                    if self.bit(source, context) == 1 {
                        self.magnitudes[index] |= 1 << plane;
                    }
                    self.known_plane[index] = plane as u8;
                    self.flags[index] |= REFINED;
                }
            }
        }
    }

    fn any_neighbour(&self, x: i64, y: i64, end: i64) -> bool {
        (-1..=1).any(|dy| (-1..=1).any(|dx| (dx, dy) != (0, 0) && self.significant(x + dx, y + dy, end) == 1))
    }

    fn cleanup_pass(&mut self, source: &mut Source, plane: u32) {
        for (top, end) in self.stripes() {
            for x in 0..self.width as i64 {
                let mut y = top;
                if end - top == 4
                    && (top..end).all(|row| {
                        self.flags[row as usize * self.width + x as usize] & (SIGNIFICANT | VISITED) == 0
                            && self.zero_context(x, row, end) == 0
                    })
                {
                    if self.bit(source, RUN_LENGTH) == 0 {
                        continue;
                    }
                    let first = self.bit(source, UNIFORM) << 1 | self.bit(source, UNIFORM);
                    y = top + first as i64;
                    self.become_significant(source, x, y, end, plane);
                    y += 1;
                }
                while y < end {
                    let index = y as usize * self.width + x as usize;
                    if self.flags[index] & (SIGNIFICANT | VISITED) == 0 {
                        let context = self.zero_context(x, y, end);
                        if self.bit(source, context) == 1 {
                            self.become_significant(source, x, y, end, plane);
                        }
                    }
                    y += 1;
                }
            }
        }
        for flag in &mut self.flags {
            *flag &= !VISITED;
        }
        if self.style & STYLE_SEGMENTATION != 0 {
            for _ in 0..4 {
                self.bit(source, UNIFORM);
            }
        }
    }
}

fn initial_contexts() -> [MqContext; 19] {
    let mut contexts = [MqContext::default(); 19];
    contexts[0].state = 4;
    contexts[RUN_LENGTH].state = 3;
    contexts[UNIFORM].state = 46;
    contexts
}
