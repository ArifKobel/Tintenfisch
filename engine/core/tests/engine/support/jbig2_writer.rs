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

pub type Context = (u8, u8);

pub struct MqEncoder {
    a: u32,
    c: u32,
    ct: u32,
    out: Vec<u8>,
}

impl MqEncoder {
    pub fn new() -> Self {
        Self { a: 0x8000, c: 0, ct: 12, out: vec![0] }
    }

    pub fn encode(&mut self, context: &mut Context, bit: u8) {
        let (state, mps) = *context;
        let (qe, next_mps, next_lps, switch) = STATES[state as usize];
        self.a -= qe;
        if bit == mps {
            if self.a & 0x8000 == 0 {
                if self.a < qe {
                    self.a = qe;
                } else {
                    self.c += qe;
                }
                context.0 = next_mps;
                self.renormalize();
            } else {
                self.c += qe;
            }
        } else {
            if self.a < qe {
                self.c += qe;
            } else {
                self.a = qe;
            }
            if switch {
                context.1 = 1 - mps;
            }
            context.0 = next_lps;
            self.renormalize();
        }
    }

    fn renormalize(&mut self) {
        loop {
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
            if self.ct == 0 {
                self.byte_out();
            }
            if self.a & 0x8000 != 0 {
                break;
            }
        }
    }

    fn byte_out(&mut self) {
        let last = self.out.len() - 1;
        if self.out[last] == 0xff {
            self.out.push((self.c >> 20) as u8);
            self.c &= 0xfffff;
            self.ct = 7;
        } else if self.c < 0x800_0000 {
            self.out.push((self.c >> 19) as u8);
            self.c &= 0x7ffff;
            self.ct = 8;
        } else {
            self.out[last] += 1;
            if self.out[last] == 0xff {
                self.c &= 0x7ff_ffff;
                self.out.push((self.c >> 20) as u8);
                self.c &= 0xfffff;
                self.ct = 7;
            } else {
                self.out.push((self.c >> 19) as u8);
                self.c &= 0x7ffff;
                self.ct = 8;
            }
        }
    }

    pub fn finish(mut self) -> Vec<u8> {
        let bound = self.c + self.a;
        self.c |= 0xffff;
        if self.c >= bound {
            self.c -= 0x8000;
        }
        self.c <<= self.ct;
        self.byte_out();
        self.c <<= self.ct;
        self.byte_out();
        if self.out.last() != Some(&0xff) {
            self.out.push(0xff);
        }
        self.out.push(0xac);
        self.out.remove(0);
        self.out
    }
}

pub struct IntegerEncoder {
    contexts: Vec<Context>,
}

impl IntegerEncoder {
    pub fn new() -> Self {
        Self { contexts: vec![(0, 0); 512] }
    }

    pub fn encode(&mut self, encoder: &mut MqEncoder, value: Option<i64>) {
        let mut previous = 1usize;
        let mut bit = |encoder: &mut MqEncoder, bit: u8| {
            encoder.encode(&mut self.contexts[previous], bit);
            let bit = bit as usize;
            previous = if previous < 256 { previous << 1 | bit } else { ((previous << 1 | bit) & 511) | 256 };
        };
        let (negative, magnitude) = match value {
            None => (true, 0),
            Some(value) => (value < 0, value.unsigned_abs()),
        };
        bit(encoder, negative as u8);
        let ranges: [(u64, u64, u32); 6] =
            [(0, 3, 2), (4, 19, 4), (20, 83, 6), (84, 339, 8), (340, 4435, 12), (4436, u64::MAX, 32)];
        let index = ranges.iter().position(|(low, high, _)| magnitude >= *low && magnitude <= *high).unwrap();
        for _ in 0..index {
            bit(encoder, 1);
        }
        if index < 5 {
            bit(encoder, 0);
        }
        let (low, _, bits) = ranges[index];
        for shift in (0..bits).rev() {
            bit(encoder, ((magnitude - low) >> shift & 1) as u8);
        }
    }
}

pub struct IdEncoder {
    length: u32,
    contexts: Vec<Context>,
}

impl IdEncoder {
    pub fn new(count: usize) -> Self {
        let length = usize::BITS - count.saturating_sub(1).leading_zeros();
        Self { length, contexts: vec![(0, 0); 1 << (length + 1)] }
    }

    pub fn encode(&mut self, encoder: &mut MqEncoder, id: usize) {
        let mut previous = 1usize;
        for shift in (0..self.length).rev() {
            let bit = (id >> shift & 1) as u8;
            encoder.encode(&mut self.contexts[previous], bit);
            previous = previous << 1 | bit as usize;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub width: usize,
    pub height: usize,
    pub black: Vec<bool>,
}

impl Picture {
    pub fn blank(width: usize, height: usize) -> Picture {
        Picture { width, height, black: vec![false; width * height] }
    }

    pub fn get(&self, x: i64, y: i64) -> usize {
        let inside = x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height;
        (inside && self.black[y as usize * self.width + x as usize]) as usize
    }

    pub fn set(&mut self, x: i64, y: i64, black: bool) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.black[y as usize * self.width + x as usize] = black;
        }
    }

    pub fn stamp(&mut self, other: &Picture, left: i64, top: i64) {
        for y in 0..other.height as i64 {
            for x in 0..other.width as i64 {
                if other.get(x, y) == 1 {
                    self.set(left + x, top + y, true);
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum RegionCoding<'a> {
    Arithmetic { template: u8, typical_prediction: bool, adaptive: &'a [(i8, i8)] },
    Mmr(&'a [u8]),
}

pub const DEFAULT_ADAPTIVE: [[(i8, i8); 4]; 4] =
    [[(3, -1), (-3, -1), (2, -2), (-2, -2)], [(3, -1); 4], [(2, -1); 4], [(2, -1); 4]];

fn wide(adaptive: &[(i8, i8)]) -> Vec<(i64, i64)> {
    adaptive.iter().map(|(x, y)| (*x as i64, *y as i64)).collect()
}

fn neighbours(template: u8, adaptive: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let at = |index: usize| adaptive[index];
    match template {
        0 => vec![
            (-1, 0),
            (-2, 0),
            (-3, 0),
            (-4, 0),
            at(0),
            (2, -1),
            (1, -1),
            (0, -1),
            (-1, -1),
            (-2, -1),
            at(1),
            at(2),
            (1, -2),
            (0, -2),
            (-1, -2),
            at(3),
        ],
        1 => vec![
            (-1, 0),
            (-2, 0),
            (-3, 0),
            at(0),
            (2, -1),
            (1, -1),
            (0, -1),
            (-1, -1),
            (-2, -1),
            (2, -2),
            (1, -2),
            (0, -2),
            (-1, -2),
        ],
        2 => vec![(-1, 0), (-2, 0), at(0), (1, -1), (0, -1), (-1, -1), (-2, -1), (1, -2), (0, -2), (-1, -2)],
        _ => vec![(-1, 0), (-2, 0), (-3, 0), (-4, 0), at(0), (1, -1), (0, -1), (-1, -1), (-2, -1), (-3, -1)],
    }
}

pub fn generic_contexts(template: u8) -> Vec<Context> {
    vec![(0, 0); 1 << [16, 13, 10, 10][template as usize]]
}

pub fn encode_generic(
    encoder: &mut MqEncoder,
    contexts: &mut [Context],
    picture: &Picture,
    template: u8,
    typical_prediction: bool,
    adaptive: &[(i64, i64)],
    skip: Option<&Picture>,
) {
    let typical_context = [0x9b25, 0x0795, 0x00e5, 0x0195][template as usize];
    let offsets = neighbours(template, adaptive);
    let mut typical = false;
    for y in 0..picture.height as i64 {
        if typical_prediction {
            let same = (0..picture.width as i64).all(|x| picture.get(x, y) == picture.get(x, y - 1));
            encoder.encode(&mut contexts[typical_context], (same != typical) as u8);
            typical = same;
            if same {
                continue;
            }
        }
        for x in 0..picture.width as i64 {
            if skip.is_some_and(|skip| skip.get(x, y) == 1) {
                continue;
            }
            let context = offsets
                .iter()
                .enumerate()
                .fold(0, |context, (bit, (dx, dy))| context | picture.get(x + dx, y + dy) << bit);
            encoder.encode(&mut contexts[context], picture.get(x, y) as u8);
        }
    }
}

pub fn refinement_contexts(template: u8) -> Vec<Context> {
    vec![(0, 0); 1 << [13, 10][template as usize]]
}

type Offsets = Vec<(i64, i64)>;

pub struct Refinement<'a> {
    pub picture: &'a Picture,
    pub reference: &'a Picture,
    pub offset: (i64, i64),
    pub template: u8,
    pub typical_prediction: bool,
    pub adaptive: [(i64, i64); 2],
}

pub fn encode_refinement(encoder: &mut MqEncoder, contexts: &mut [Context], refinement: &Refinement) {
    let Refinement { picture, reference, offset: (dx, dy), template, typical_prediction, adaptive } = *refinement;
    let (own, referenced): (Offsets, Offsets) = if template == 0 {
        (
            vec![(-1, 0), (1, -1), (0, -1), adaptive[0]],
            vec![(1, 1), (0, 1), (-1, 1), (1, 0), (0, 0), (-1, 0), (1, -1), (0, -1), adaptive[1]],
        )
    } else {
        (vec![(-1, 0), (1, -1), (0, -1), (-1, -1)], vec![(1, 1), (0, 1), (1, 0), (0, 0), (-1, 0), (0, -1)])
    };
    let uniform = |x: i64, y: i64| {
        let value = reference.get(x - dx, y - dy);
        (-1..=1).all(|oy| (-1..=1).all(|ox| reference.get(x - dx + ox, y - dy + oy) == value))
    };
    let mut typical = false;
    for y in 0..picture.height as i64 {
        if typical_prediction {
            let predictable =
                (0..picture.width as i64).all(|x| !uniform(x, y) || picture.get(x, y) == reference.get(x - dx, y - dy));
            encoder.encode(&mut contexts[[0x0100, 0x0040][template as usize]], (predictable != typical) as u8);
            typical = predictable;
        }
        for x in 0..picture.width as i64 {
            if typical && uniform(x, y) {
                continue;
            }
            let context = own
                .iter()
                .map(|(ox, oy)| picture.get(x + ox, y + oy))
                .chain(referenced.iter().map(|(ox, oy)| reference.get(x - dx + ox, y - dy + oy)))
                .enumerate()
                .fold(0, |context, (bit, value)| context | value << bit);
            encoder.encode(&mut contexts[context], picture.get(x, y) as u8);
        }
    }
}

fn arithmetic_region(picture: &Picture, template: u8, typical_prediction: bool, adaptive: &[(i8, i8)]) -> Vec<u8> {
    let mut encoder = MqEncoder::new();
    let mut contexts = generic_contexts(template);
    encode_generic(&mut encoder, &mut contexts, picture, template, typical_prediction, &wide(adaptive), None);
    encoder.finish()
}

pub fn segment(number: u32, kind: u8, referred: &[u32], on_page: bool, data: &[u8]) -> Vec<u8> {
    assert!(referred.len() <= 4);
    let mut out = number.to_be_bytes().to_vec();
    out.push(kind);
    out.push((referred.len() as u8) << 5);
    for other in referred {
        match number {
            0..=256 => out.push(*other as u8),
            257..=65536 => out.extend((*other as u16).to_be_bytes()),
            _ => out.extend(other.to_be_bytes()),
        }
    }
    out.push(on_page as u8);
    out.extend((data.len() as u32).to_be_bytes());
    out.extend(data);
    out
}

pub fn page_information(width: usize, height: usize) -> Vec<u8> {
    let mut page = (width as u32).to_be_bytes().to_vec();
    page.extend((height as u32).to_be_bytes());
    page.extend([0; 8]);
    page.extend([0x40, 0, 0]);
    page
}

pub fn region_information(width: usize, height: usize, left: u32, top: u32, combination: u8) -> Vec<u8> {
    let mut info = (width as u32).to_be_bytes().to_vec();
    info.extend((height as u32).to_be_bytes());
    info.extend(left.to_be_bytes());
    info.extend(top.to_be_bytes());
    info.push(combination);
    info
}

pub struct Region<'a> {
    pub picture: &'a Picture,
    pub left: u32,
    pub top: u32,
    pub combination: u8,
    pub coding: RegionCoding<'a>,
}

pub fn generic_region_data(region: &Region) -> Vec<u8> {
    let mut data =
        region_information(region.picture.width, region.picture.height, region.left, region.top, region.combination);
    match region.coding {
        RegionCoding::Arithmetic { template, typical_prediction, adaptive } => {
            data.push(template << 1 | (typical_prediction as u8) << 3);
            let count = if template == 0 { 4 } else { 1 };
            data.extend(adaptive[..count].iter().flat_map(|(dx, dy)| [*dx as u8, *dy as u8]));
            data.extend(arithmetic_region(region.picture, template, typical_prediction, adaptive));
        }
        RegionCoding::Mmr(mmr) => {
            data.push(1);
            data.extend(mmr);
        }
    }
    data
}

pub fn embedded_stream(page_width: usize, page_height: usize, regions: &[Region]) -> Vec<u8> {
    let mut stream = segment(0, 48, &[], true, &page_information(page_width, page_height));
    for (index, region) in regions.iter().enumerate() {
        stream.extend(segment(1 + index as u32, 38, &[], true, &generic_region_data(region)));
    }
    stream
}

pub enum NewSymbol<'a> {
    Drawn(&'a Picture),
    Refined { picture: &'a Picture, of: usize, offset: (i64, i64) },
    Aggregate { picture: &'a Picture, parts: Vec<(usize, i64, i64)> },
}

impl NewSymbol<'_> {
    fn picture(&self) -> &Picture {
        match self {
            NewSymbol::Drawn(picture) | NewSymbol::Refined { picture, .. } | NewSymbol::Aggregate { picture, .. } => {
                picture
            }
        }
    }
}

pub fn symbol_dictionary(known: &[Picture], new: &[NewSymbol], refine_aggregate: bool, exported: &[bool]) -> Vec<u8> {
    let template = 2u8;
    let refinement_template = 1u8;
    let flags: u16 = (refine_aggregate as u16) << 1 | (template as u16) << 10 | (refinement_template as u16) << 12;
    let mut data = flags.to_be_bytes().to_vec();
    data.extend([2u8, (-1i8) as u8]);
    data.extend((exported.iter().filter(|flag| **flag).count() as u32).to_be_bytes());
    data.extend((new.len() as u32).to_be_bytes());
    let mut encoder = MqEncoder::new();
    let mut state = TextEncoding::new(known.len() + new.len(), template, refinement_template);
    let mut symbols: Vec<Picture> = known.to_vec();
    let mut height = 0i64;
    let mut index = 0;
    while index < new.len() {
        let class_height = new[index].picture().height as i64;
        state.height_delta.encode(&mut encoder, Some(class_height - height));
        height = class_height;
        let mut width = 0i64;
        while index < new.len() && new[index].picture().height as i64 == height {
            let symbol = &new[index];
            let picture = symbol.picture();
            state.width_delta.encode(&mut encoder, Some(picture.width as i64 - width));
            width = picture.width as i64;
            match symbol {
                NewSymbol::Drawn(picture) => {
                    assert!(!refine_aggregate);
                    encode_generic(&mut encoder, &mut state.generic, picture, template, false, &[(2, -1)], None);
                }
                NewSymbol::Refined { picture, of, offset } => {
                    state.instance_count.encode(&mut encoder, Some(1));
                    state.symbol_id.encode(&mut encoder, *of);
                    state.refinement_x.encode(&mut encoder, Some(offset.0));
                    state.refinement_y.encode(&mut encoder, Some(offset.1));
                    let refinement = Refinement {
                        picture,
                        reference: &symbols[*of],
                        offset: *offset,
                        template: refinement_template,
                        typical_prediction: false,
                        adaptive: [(-1, -1); 2],
                    };
                    encode_refinement(&mut encoder, &mut state.refinement, &refinement);
                }
                NewSymbol::Aggregate { parts, .. } => {
                    state.instance_count.encode(&mut encoder, Some(parts.len() as i64));
                    let placed: Vec<Placed> = parts
                        .iter()
                        .map(|(id, left, top)| Placed { id: *id, left: *left, top: *top, refined: None })
                        .collect();
                    let mut sink = ArithmeticSink { encoder: &mut encoder, state: &mut state };
                    encode_text(&mut sink, &symbols, &placed, &TextLayout::plain(), refinement_template);
                }
            }
            symbols.push(picture.clone());
            index += 1;
        }
        state.width_delta.encode(&mut encoder, None);
    }
    let mut flag = false;
    let mut index = 0;
    while index < exported.len() {
        let run = exported[index..].iter().take_while(|exporting| **exporting == flag).count();
        state.export_run.encode(&mut encoder, Some(run as i64));
        index += run;
        flag = !flag;
    }
    data.extend(encoder.finish());
    data
}

pub struct TextEncoding {
    pub height_delta: IntegerEncoder,
    pub width_delta: IntegerEncoder,
    pub export_run: IntegerEncoder,
    pub instance_count: IntegerEncoder,
    pub strip_delta: IntegerEncoder,
    pub first_s: IntegerEncoder,
    pub s_delta: IntegerEncoder,
    pub instance_t: IntegerEncoder,
    pub refined: IntegerEncoder,
    pub refinement_width: IntegerEncoder,
    pub refinement_height: IntegerEncoder,
    pub refinement_x: IntegerEncoder,
    pub refinement_y: IntegerEncoder,
    pub symbol_id: IdEncoder,
    pub generic: Vec<Context>,
    pub refinement: Vec<Context>,
}

impl TextEncoding {
    pub fn new(symbol_count: usize, template: u8, refinement_template: u8) -> Self {
        Self {
            height_delta: IntegerEncoder::new(),
            width_delta: IntegerEncoder::new(),
            export_run: IntegerEncoder::new(),
            instance_count: IntegerEncoder::new(),
            strip_delta: IntegerEncoder::new(),
            first_s: IntegerEncoder::new(),
            s_delta: IntegerEncoder::new(),
            instance_t: IntegerEncoder::new(),
            refined: IntegerEncoder::new(),
            refinement_width: IntegerEncoder::new(),
            refinement_height: IntegerEncoder::new(),
            refinement_x: IntegerEncoder::new(),
            refinement_y: IntegerEncoder::new(),
            symbol_id: IdEncoder::new(symbol_count),
            generic: generic_contexts(template),
            refinement: refinement_contexts(refinement_template),
        }
    }
}

pub struct Placed<'a> {
    pub id: usize,
    pub left: i64,
    pub top: i64,
    pub refined: Option<(&'a Picture, i64, i64)>,
}

pub struct TextLayout {
    pub strips: i64,
    pub corner: u8,
    pub transposed: bool,
    pub s_offset: i64,
}

impl TextLayout {
    pub fn plain() -> TextLayout {
        TextLayout { strips: 1, corner: 1, transposed: false, s_offset: 0 }
    }
}

pub trait TextSink {
    fn strip_delta(&mut self, delta: i64);
    fn first_s(&mut self, delta: i64);
    fn s_delta(&mut self, delta: Option<i64>);
    fn instance_t(&mut self, t: i64, strips: i64);
    fn symbol_id(&mut self, id: usize);
    fn refinement(&mut self, symbol: &Picture, refined: Option<(&Picture, i64, i64)>, template: u8);
}

pub struct ArithmeticSink<'a> {
    pub encoder: &'a mut MqEncoder,
    pub state: &'a mut TextEncoding,
}

impl TextSink for ArithmeticSink<'_> {
    fn strip_delta(&mut self, delta: i64) {
        self.state.strip_delta.encode(self.encoder, Some(delta));
    }

    fn first_s(&mut self, delta: i64) {
        self.state.first_s.encode(self.encoder, Some(delta));
    }

    fn s_delta(&mut self, delta: Option<i64>) {
        self.state.s_delta.encode(self.encoder, delta);
    }

    fn instance_t(&mut self, t: i64, strips: i64) {
        if strips > 1 {
            self.state.instance_t.encode(self.encoder, Some(t));
        }
    }

    fn symbol_id(&mut self, id: usize) {
        self.state.symbol_id.encode(self.encoder, id);
    }

    fn refinement(&mut self, symbol: &Picture, refined: Option<(&Picture, i64, i64)>, template: u8) {
        let Some((picture, x, y)) = refined else {
            self.state.refined.encode(self.encoder, Some(0));
            return;
        };
        let (width_delta, height_delta) = size_change(symbol, picture);
        self.state.refined.encode(self.encoder, Some(1));
        self.state.refinement_width.encode(self.encoder, Some(width_delta));
        self.state.refinement_height.encode(self.encoder, Some(height_delta));
        self.state.refinement_x.encode(self.encoder, Some(x));
        self.state.refinement_y.encode(self.encoder, Some(y));
        let refinement = instance_refinement(symbol, picture, x, y, template);
        encode_refinement(self.encoder, &mut self.state.refinement, &refinement);
    }
}

pub fn size_change(symbol: &Picture, picture: &Picture) -> (i64, i64) {
    (picture.width as i64 - symbol.width as i64, picture.height as i64 - symbol.height as i64)
}

pub fn instance_refinement<'a>(
    symbol: &'a Picture,
    picture: &'a Picture,
    x: i64,
    y: i64,
    template: u8,
) -> Refinement<'a> {
    let (width_delta, height_delta) = size_change(symbol, picture);
    Refinement {
        picture,
        reference: symbol,
        offset: ((width_delta >> 1) + x, (height_delta >> 1) + y),
        template,
        typical_prediction: false,
        adaptive: [(-1, -1); 2],
    }
}

pub fn encode_text(
    sink: &mut dyn TextSink,
    symbols: &[Picture],
    placed: &[Placed],
    layout: &TextLayout,
    refinement_template: u8,
) {
    let left_corner = layout.corner == 0 || layout.corner == 1;
    let top_corner = layout.corner == 1 || layout.corner == 3;
    let mut instances: Vec<(i64, i64, &Placed, i64, i64)> = placed
        .iter()
        .map(|instance| {
            let picture = instance.refined.map_or(&symbols[instance.id], |(picture, _, _)| picture);
            let (width, height) = (picture.width as i64, picture.height as i64);
            let x = if left_corner { instance.left } else { instance.left + width - 1 };
            let y = if top_corner { instance.top } else { instance.top + height - 1 };
            let (s, t) = if layout.transposed { (y, x) } else { (x, y) };
            let before = if !layout.transposed && !left_corner {
                width - 1
            } else if layout.transposed && !top_corner {
                height - 1
            } else {
                0
            };
            let after = if !layout.transposed && left_corner {
                width - 1
            } else if layout.transposed && top_corner {
                height - 1
            } else {
                0
            };
            (s, t, instance, before, after)
        })
        .collect();
    instances.sort_by_key(|(s, t, ..)| (t.div_euclid(layout.strips), *s));
    sink.strip_delta(1);
    let mut strip_t = -layout.strips;
    let mut first_s = 0;
    let mut index = 0;
    while index < instances.len() {
        let strip = instances[index].1.div_euclid(layout.strips) * layout.strips;
        sink.strip_delta((strip - strip_t) / layout.strips);
        strip_t = strip;
        let mut current_s: Option<i64> = None;
        while index < instances.len() && instances[index].1.div_euclid(layout.strips) * layout.strips == strip {
            let (s, t, instance, before, after) = instances[index];
            let decoded_s = s - before;
            match current_s {
                None => {
                    sink.first_s(decoded_s - first_s);
                    first_s = decoded_s;
                }
                Some(current) => sink.s_delta(Some(decoded_s - current - layout.s_offset)),
            }
            sink.instance_t(t - strip, layout.strips);
            sink.symbol_id(instance.id);
            sink.refinement(&symbols[instance.id], instance.refined, refinement_template);
            current_s = Some(s + after);
            index += 1;
        }
        sink.s_delta(None);
    }
}

pub fn text_region(
    width: usize,
    height: usize,
    symbols: &[Picture],
    placed: &[Placed],
    layout: &TextLayout,
) -> Vec<u8> {
    let mut data = region_information(width, height, 0, 0, 0);
    let log_strips = layout.strips.trailing_zeros() as u16;
    let flags: u16 = 2
        | log_strips << 2
        | (layout.corner as u16) << 4
        | (layout.transposed as u16) << 6
        | ((layout.s_offset & 0x1f) as u16) << 10
        | 1 << 15;
    data.extend(flags.to_be_bytes());
    data.extend((placed.len() as u32).to_be_bytes());
    let mut encoder = MqEncoder::new();
    let mut state = TextEncoding::new(symbols.len(), 0, 1);
    encode_text(&mut ArithmeticSink { encoder: &mut encoder, state: &mut state }, symbols, placed, layout, 1);
    data.extend(encoder.finish());
    data
}

pub fn refinement_region(
    picture: &Picture,
    reference: &Picture,
    left: u32,
    top: u32,
    combination: u8,
    template: u8,
    typical_prediction: bool,
) -> Vec<u8> {
    let mut data = region_information(picture.width, picture.height, left, top, combination);
    data.push(template | (typical_prediction as u8) << 1);
    let adaptive = [(-1i64, -1i64), (-1, -1)];
    if template == 0 {
        data.extend([(-1i8) as u8, (-1i8) as u8, (-1i8) as u8, (-1i8) as u8]);
    }
    let mut encoder = MqEncoder::new();
    let mut contexts = refinement_contexts(template);
    let refinement = Refinement { picture, reference, offset: (0, 0), template, typical_prediction, adaptive };
    encode_refinement(&mut encoder, &mut contexts, &refinement);
    data.extend(encoder.finish());
    data
}

pub fn pattern_dictionary(patterns: &[Picture]) -> Vec<u8> {
    let (width, height) = (patterns[0].width, patterns[0].height);
    let mut data = vec![0u8, width as u8, height as u8];
    data.extend(((patterns.len() - 1) as u32).to_be_bytes());
    let mut collective = Picture::blank(width * patterns.len(), height);
    for (index, pattern) in patterns.iter().enumerate() {
        collective.stamp(pattern, (index * width) as i64, 0);
    }
    let mut encoder = MqEncoder::new();
    let mut contexts = generic_contexts(0);
    encode_generic(
        &mut encoder,
        &mut contexts,
        &collective,
        0,
        false,
        &[(-(width as i64), 0), (-3, -1), (2, -2), (-2, -2)],
        None,
    );
    data.extend(encoder.finish());
    data
}

pub fn halftone_region(
    width: usize,
    height: usize,
    gray: &[Vec<usize>],
    patterns: usize,
    (x, y): (i32, i32),
    (step_x, step_y): (u16, u16),
    pattern_size: usize,
) -> Vec<u8> {
    let mut data = region_information(width, height, 0, 0, 0);
    data.push(0x08);
    let (grid_height, grid_width) = (gray.len(), gray[0].len());
    data.extend((grid_width as u32).to_be_bytes());
    data.extend((grid_height as u32).to_be_bytes());
    data.extend(x.to_be_bytes());
    data.extend(y.to_be_bytes());
    data.extend(step_x.to_be_bytes());
    data.extend(step_y.to_be_bytes());
    let mut skip = Picture::blank(grid_width, grid_height);
    for row in 0..grid_height {
        for column in 0..grid_width {
            let cell_x = (x as i64 + row as i64 * step_y as i64 + column as i64 * step_x as i64) >> 8;
            let cell_y = (y as i64 + row as i64 * step_x as i64 - column as i64 * step_y as i64) >> 8;
            let size = pattern_size as i64;
            let outside = cell_x + size <= 0 || cell_x >= width as i64 || cell_y + size <= 0 || cell_y >= height as i64;
            skip.set(column as i64, row as i64, outside);
        }
    }
    let planes = usize::BITS - patterns.saturating_sub(1).leading_zeros();
    let mut encoder = MqEncoder::new();
    let mut contexts = generic_contexts(0);
    for plane in (0..planes).rev() {
        let mut bits = Picture::blank(grid_width, grid_height);
        for (row, values) in gray.iter().enumerate() {
            for (column, value) in values.iter().enumerate() {
                let gray_coded = (value >> plane & 1) ^ (value >> (plane + 1) & 1);
                bits.set(column as i64, row as i64, gray_coded == 1 && skip.get(column as i64, row as i64) == 0);
            }
        }
        encode_generic(
            &mut encoder,
            &mut contexts,
            &bits,
            0,
            false,
            &[(3, -1), (-3, -1), (2, -2), (-2, -2)],
            Some(&skip),
        );
    }
    data.extend(encoder.finish());
    data
}
