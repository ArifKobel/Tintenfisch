use tiny_skia::{Path, PathBuilder};

use crate::fonts::encoding_tables::standard;
use crate::fonts::type1_program::Type1Program;

const MAX_SUBROUTINE_DEPTH: usize = 10;

pub struct CharstringInterpreter<'a> {
    program: &'a Type1Program,
    builder: PathBuilder,
    stack: Vec<f64>,
    other_results: Vec<f64>,
    x: f64,
    y: f64,
    side_bearing: f64,
    pub advance: f64,
    flexing: bool,
    flex_start: (f64, f64),
    flex_points: Vec<(f64, f64)>,
    started: bool,
}

impl<'a> CharstringInterpreter<'a> {
    pub fn new(program: &'a Type1Program) -> Self {
        Self {
            program,
            builder: PathBuilder::new(),
            stack: Vec::new(),
            other_results: Vec::new(),
            x: 0.0,
            y: 0.0,
            side_bearing: 0.0,
            advance: 0.0,
            flexing: false,
            flex_start: (0.0, 0.0),
            flex_points: Vec::new(),
            started: false,
        }
    }

    pub fn run(mut self, glyph: u16) -> (Option<Path>, f64) {
        if let Some(charstring) = self.program.charstrings.get(glyph as usize) {
            self.execute(charstring, 0);
        }
        let advance = self.advance;
        (self.builder.finish(), advance)
    }

    fn move_to(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
        if self.flexing {
            self.flex_points.push((self.x, self.y));
            return;
        }
        self.builder.move_to(self.x as f32, self.y as f32);
        self.started = true;
    }

    fn line_to(&mut self, dx: f64, dy: f64) {
        self.ensure_started();
        self.x += dx;
        self.y += dy;
        self.builder.line_to(self.x as f32, self.y as f32);
    }

    fn curve_to(&mut self, d: [f64; 6]) {
        self.ensure_started();
        let (x1, y1) = (self.x + d[0], self.y + d[1]);
        let (x2, y2) = (x1 + d[2], y1 + d[3]);
        let (x3, y3) = (x2 + d[4], y2 + d[5]);
        self.builder.cubic_to(x1 as f32, y1 as f32, x2 as f32, y2 as f32, x3 as f32, y3 as f32);
        self.x = x3;
        self.y = y3;
    }

    fn ensure_started(&mut self) {
        if !self.started {
            self.builder.move_to(self.x as f32, self.y as f32);
            self.started = true;
        }
    }

    fn execute(&mut self, charstring: &[u8], depth: usize) -> bool {
        let mut position = 0;
        while position < charstring.len() {
            let byte = charstring[position];
            position += 1;
            match byte {
                32..=246 => self.stack.push(byte as f64 - 139.0),
                247..=250 => {
                    let next = *charstring.get(position).unwrap_or(&0) as f64;
                    position += 1;
                    self.stack.push((byte as f64 - 247.0) * 256.0 + next + 108.0);
                }
                251..=254 => {
                    let next = *charstring.get(position).unwrap_or(&0) as f64;
                    position += 1;
                    self.stack.push(-(byte as f64 - 251.0) * 256.0 - next - 108.0);
                }
                255 => {
                    let bytes = charstring.get(position..position + 4).unwrap_or(&[0; 4]);
                    position += 4;
                    self.stack.push(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64);
                }
                12 => {
                    let escape = *charstring.get(position).unwrap_or(&0);
                    position += 1;
                    if self.escape(escape, depth) {
                        return true;
                    }
                }
                _ => {
                    if self.command(byte, depth) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn take<const N: usize>(&mut self) -> [f64; N] {
        let mut values = [0.0; N];
        let start = self.stack.len().saturating_sub(N);
        for (slot, value) in values.iter_mut().zip(self.stack.drain(start..)) {
            *slot = value;
        }
        self.stack.clear();
        values
    }

    fn command(&mut self, command: u8, depth: usize) -> bool {
        match command {
            1 | 3 => self.stack.clear(),
            4 => {
                let [dy] = self.take();
                self.move_to(0.0, dy);
            }
            5 => {
                let [dx, dy] = self.take();
                self.line_to(dx, dy);
            }
            6 => {
                let [dx] = self.take();
                self.line_to(dx, 0.0);
            }
            7 => {
                let [dy] = self.take();
                self.line_to(0.0, dy);
            }
            8 => {
                let d = self.take::<6>();
                self.curve_to(d);
            }
            9 => {
                self.builder.close();
                self.stack.clear();
            }
            10 => {
                let index = self.stack.pop().unwrap_or(-1.0);
                if depth < MAX_SUBROUTINE_DEPTH
                    && index >= 0.0
                    && let Some(subroutine) = self.program.subroutines.get(index as usize)
                {
                    return self.execute(subroutine, depth + 1);
                }
            }
            11 => return false,
            13 => {
                let [side_bearing, width] = self.take();
                self.side_bearing = side_bearing;
                self.advance = width;
                self.x = side_bearing;
                self.y = 0.0;
            }
            14 => {
                self.stack.clear();
                return true;
            }
            21 => {
                let [dx, dy] = self.take();
                self.move_to(dx, dy);
            }
            22 => {
                let [dx] = self.take();
                self.move_to(dx, 0.0);
            }
            30 => {
                let [dy1, dx2, dy2, dx3] = self.take();
                self.curve_to([0.0, dy1, dx2, dy2, dx3, 0.0]);
            }
            31 => {
                let [dx1, dx2, dy2, dy3] = self.take();
                self.curve_to([dx1, 0.0, dx2, dy2, 0.0, dy3]);
            }
            _ => self.stack.clear(),
        }
        false
    }

    fn escape(&mut self, escape: u8, depth: usize) -> bool {
        match escape {
            6 => {
                let [accent_side_bearing, dx, dy, base, accent] = self.take();
                self.seac(accent_side_bearing, dx, dy, base as u8, accent as u8, depth);
                return true;
            }
            7 => {
                let [side_bearing_x, side_bearing_y, width, _] = self.take();
                self.side_bearing = side_bearing_x;
                self.advance = width;
                self.x = side_bearing_x;
                self.y = side_bearing_y;
            }
            12 => {
                let divisor = self.stack.pop().unwrap_or(1.0);
                let dividend = self.stack.pop().unwrap_or(0.0);
                self.stack.push(if divisor == 0.0 { 0.0 } else { dividend / divisor });
            }
            16 => self.call_other_subroutine(),
            17 => {
                let value = self.other_results.pop().unwrap_or(0.0);
                self.stack.push(value);
            }
            33 => {
                let [x, y] = self.take();
                self.x = x;
                self.y = y;
            }
            _ => self.stack.clear(),
        }
        false
    }

    fn call_other_subroutine(&mut self) {
        let number = self.stack.pop().unwrap_or(-1.0) as i64;
        let count = (self.stack.pop().unwrap_or(0.0).max(0.0) as usize).min(self.stack.len());
        let arguments: Vec<f64> = self.stack.drain(self.stack.len() - count..).collect();
        self.other_results.clear();
        match number {
            1 => {
                self.flexing = true;
                self.flex_start = (self.x, self.y);
                self.flex_points.clear();
            }
            0 => {
                self.flexing = false;
                if self.flex_points.len() >= 7 {
                    let points = std::mem::take(&mut self.flex_points);
                    let (end_x, end_y) = points[6];
                    (self.x, self.y) = self.flex_start;
                    self.ensure_started();
                    for curve in [&points[1..4], &points[4..7]] {
                        self.builder.cubic_to(
                            curve[0].0 as f32,
                            curve[0].1 as f32,
                            curve[1].0 as f32,
                            curve[1].1 as f32,
                            curve[2].0 as f32,
                            curve[2].1 as f32,
                        );
                    }
                    self.x = end_x;
                    self.y = end_y;
                }
                self.other_results = vec![self.y, self.x];
            }
            3 => self.other_results = vec![3.0],
            _ => self.other_results = arguments.into_iter().rev().collect(),
        }
    }

    fn seac(&mut self, accent_side_bearing: f64, dx: f64, dy: f64, base: u8, accent: u8, depth: usize) {
        let table = standard();
        let glyph_of = |code: u8| table[code as usize].and_then(|name| self.program.glyph(name));
        let (side_bearing, advance) = (self.side_bearing, self.advance);
        if let Some(glyph) = glyph_of(base)
            && let Some(charstring) = self.program.charstrings.get(glyph as usize)
        {
            self.execute(charstring, depth + 1);
        }
        if let Some(glyph) = glyph_of(accent)
            && let Some(charstring) = self.program.charstrings.get(glyph as usize)
        {
            let origin = (side_bearing - accent_side_bearing + dx, dy);
            self.started = false;
            let mut accent_run = CharstringInterpreter::new(self.program);
            accent_run.execute(charstring, depth + 1);
            if let Some(path) = accent_run.builder.finish()
                && let Some(moved) =
                    path.transform(tiny_skia::Transform::from_translate(origin.0 as f32, origin.1 as f32))
            {
                self.builder.push_path(&moved);
            }
        }
        self.side_bearing = side_bearing;
        self.advance = advance;
    }
}

#[cfg(test)]
mod tests {
    use crate::fonts::FontProgram;
    use crate::fonts::type1_program::CHARSTRING_KEY;

    fn encrypt(plain: &[u8], key: u16) -> Vec<u8> {
        let mut register = key;
        let mut cipher = Vec::new();
        for &byte in [0u8, 0, 0, 0].iter().chain(plain) {
            let encrypted = byte ^ (register >> 8) as u8;
            register = (encrypted as u16).wrapping_add(register).wrapping_mul(52845).wrapping_add(22719);
            cipher.push(encrypted);
        }
        cipher
    }

    fn number(value: i32) -> Vec<u8> {
        match value {
            -107..=107 => vec![(value + 139) as u8],
            108..=1131 => vec![(247 + (value - 108) / 256) as u8, ((value - 108) % 256) as u8],
            -1131..=-108 => vec![(251 + (-value - 108) / 256) as u8, ((-value - 108) % 256) as u8],
            _ => [vec![255], value.to_be_bytes().to_vec()].concat(),
        }
    }

    fn charstring(parts: &[(&[i32], &[u8])]) -> Vec<u8> {
        parts
            .iter()
            .flat_map(|(numbers, command)| numbers.iter().flat_map(|&n| number(n)).chain(command.iter().copied()))
            .collect()
    }

    fn program(hex: bool) -> FontProgram {
        let square = charstring(&[
            (&[0, 600], &[13]),
            (&[100, 0], &[21]),
            (&[400, 0], &[5]),
            (&[0, 400], &[5]),
            (&[-400, 0], &[5]),
            (&[], &[9, 14]),
        ]);
        let notdef = charstring(&[(&[0, 250], &[13]), (&[], &[14])]);
        let mut private = b"dup /Private 8 dict dup begin /lenIV 4 def /Subrs 1 array\ndup 0 1 RD ".to_vec();
        private.extend(encrypt(&[11], CHARSTRING_KEY));
        private.extend_from_slice(b" NP\n2 index /CharStrings 2 dict dup begin\n");
        for (name, glyph) in [("/.notdef", notdef), ("/A", square)] {
            let encrypted = encrypt(&glyph, CHARSTRING_KEY);
            private.extend_from_slice(format!("{name} {} RD ", encrypted.len()).as_bytes());
            private.extend(encrypted);
            private.extend_from_slice(b" ND\n");
        }
        private.extend_from_slice(b"end end\nmark currentfile closefile\n");
        let mut data = b"%!PS-AdobeFont-1.0: Synthetic\n/FontMatrix [0.001 0 0 0.001 0 0] readonly def\n/Encoding StandardEncoding def\ncurrentfile eexec\n".to_vec();
        let length1 = data.len();
        let encrypted = encrypt(&private, 55665);
        if hex {
            let text: String = encrypted
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .chunks(32)
                .map(|line| line.concat() + "\n")
                .collect();
            data.extend_from_slice(text.as_bytes());
        } else {
            data.extend(encrypted);
        }
        FontProgram::parse_type1(data.into(), length1).unwrap()
    }

    #[test]
    fn decrypts_and_draws_a_type1_glyph_in_em_units() {
        let program = program(false);
        let glyph = program.glyph_by_name("A").unwrap();
        assert_eq!(program.glyph_for_builtin_code(b'A'), Some(glyph));
        assert!((program.advance(glyph).unwrap() - 0.6).abs() < 1e-9);
        let bounds = program.outline(glyph).unwrap().bounds();
        assert!((bounds.left() - 0.1).abs() < 1e-6 && (bounds.right() - 0.5).abs() < 1e-6);
        assert!((bounds.top() - 0.0).abs() < 1e-6 && (bounds.bottom() - 0.4).abs() < 1e-6);
        assert!(program.outline(program.glyph_by_name(".notdef").unwrap()).is_none());
    }

    #[test]
    fn reads_the_encrypted_part_in_hex_too() {
        let program = program(true);
        assert_eq!(program.glyph_count(), 2);
        assert!(program.outline(program.glyph_by_name("A").unwrap()).is_some());
    }
}
