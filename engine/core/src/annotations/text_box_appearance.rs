use ttf_parser::Face;

use crate::content::ContentWriter;
use crate::fonts::{FallbackFace, FallbackFamily, FontLibrary};
use crate::geometry::Matrix;
use crate::syntax::{Dict, Name, Object};

const PADDING: f64 = 3.0;
const LEADING: f64 = 1.2;
const FONT: &str = "Helv";

pub struct TextBoxAppearance<'a> {
    library: &'a FontLibrary,
}

impl<'a> TextBoxAppearance<'a> {
    pub fn new(library: &'a FontLibrary) -> Self {
        Self { library }
    }

    pub fn content(&self, frame: &Matrix, width: f64, height: f64, text: &str, size: f64, color: [f64; 3]) -> Vec<u8> {
        let numbers = |values: &[f64]| values.iter().map(|value| Object::Real(*value)).collect::<Vec<_>>();
        let mut content = ContentWriter::new();
        content
            .operation("q", &[])
            .operation("cm", &numbers(&[frame.a, frame.b, frame.c, frame.d, frame.e, frame.f]))
            .operation("RG", &numbers(&color))
            .operation("w", &[Object::Real(1.0)])
            .operation("re", &numbers(&[0.5, 0.5, width - 1.0, height - 1.0]))
            .operation("S", &[])
            .operation("re", &numbers(&[1.0, 1.0, width - 2.0, height - 2.0]))
            .operation("W", &[])
            .operation("n", &[])
            .operation("rg", &numbers(&color))
            .operation("BT", &[])
            .operation("Tf", &[Object::Name(Name::from(FONT)), Object::Real(size)])
            .operation("Td", &numbers(&[PADDING, height - PADDING - size]));
        for (index, line) in self.wrap(text, width - 2.0 * PADDING, size).iter().enumerate() {
            if index > 0 {
                content.operation("Td", &numbers(&[0.0, -size * LEADING]));
            }
            content.operation("Tj", &[Object::String(win_ansi(line))]);
        }
        content.operation("ET", &[]).operation("Q", &[]);
        content.finish()
    }

    pub fn resources() -> Dict {
        let font: Dict = [
            ("Type", Object::Name(Name::from("Font"))),
            ("Subtype", Object::Name(Name::from("Type1"))),
            ("BaseFont", Object::Name(Name::from("Helvetica"))),
            ("Encoding", Object::Name(Name::from("WinAnsiEncoding"))),
        ]
        .into_iter()
        .map(|(key, value)| (Name::from(key), value))
        .collect();
        let fonts: Dict = [(Name::from(FONT), Object::Dict(font))].into_iter().collect();
        [(Name::from("Font"), Object::Dict(fonts))].into_iter().collect()
    }

    pub fn default_appearance(size: f64, [red, green, blue]: [f64; 3]) -> Vec<u8> {
        format!("/{FONT} {size} Tf {red} {green} {blue} rg").into_bytes()
    }

    fn wrap(&self, text: &str, width: f64, size: f64) -> Vec<String> {
        let face = FallbackFace { family: FallbackFamily::Sans, bold: false, italic: false };
        let data = self.library.face(face);
        let parsed = data.as_deref().and_then(|data| Face::parse(data, 0).ok());
        let measure = |words: &str| match &parsed {
            Some(face) => {
                words
                    .chars()
                    .map(|character| {
                        face.glyph_index(character).and_then(|glyph| face.glyph_hor_advance(glyph)).unwrap_or(0) as f64
                    })
                    .sum::<f64>()
                    / face.units_per_em() as f64
                    * size
            }
            None => words.chars().count() as f64 * size * 0.5,
        };
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let mut line = String::new();
            for word in paragraph.split(' ') {
                let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if measure(&candidate) <= width || line.is_empty() {
                    line = candidate;
                } else {
                    lines.push(std::mem::take(&mut line));
                    line = word.to_string();
                }
            }
            lines.push(line);
        }
        lines
    }
}

pub fn win_ansi(text: &str) -> Vec<u8> {
    text.chars().map(|character| if (character as u32) < 0x100 { character as u8 } else { b'?' }).collect()
}
