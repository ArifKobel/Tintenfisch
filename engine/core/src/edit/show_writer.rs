use crate::content::ContentWriter;
use crate::geometry::Matrix;
use crate::syntax::{Name, Object};
use crate::text::{FillColor, ShowRecord, TextState};

#[derive(Clone, Debug, PartialEq)]
pub struct FontChoice {
    pub name: Name,
    pub size: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedGlyph {
    pub bytes: Vec<u8>,
    pub font: Option<FontChoice>,
    pub x: f64,
    pub advance: f64,
    pub color: Option<[f64; 3]>,
}

pub struct ShowWriter<'a> {
    state: &'a TextState,
    fill: &'a FillColor,
    font: FontChoice,
}

impl<'a> ShowWriter<'a> {
    pub fn new(record: &'a ShowRecord, font: &Name) -> Self {
        Self {
            state: &record.state,
            fill: &record.fill,
            font: FontChoice { name: font.clone(), size: record.state.size },
        }
    }

    pub fn rewrite(&self, record: &ShowRecord, glyphs: &[PlacedGlyph], shift: Option<(f64, f64)>) -> Vec<u8> {
        let mut writer = ContentWriter::new();
        match record.operator.as_str() {
            "'" => {
                writer.operation("T*", &[]);
            }
            "\"" => {
                writer.operation("Tw", &[Object::Real(self.state.word_spacing)]);
                writer.operation("Tc", &[Object::Real(self.state.char_spacing)]);
                writer.operation("T*", &[]);
            }
            _ => {}
        }
        match shift {
            Some((u, v)) => {
                self.show_at(&mut writer, record, (u, v), glyphs);
                Self::restore(&mut writer, record);
            }
            None => self.write(&mut writer, glyphs, Some(record.end)),
        }
        writer.finish()
    }

    pub fn show_at(&self, writer: &mut ContentWriter, record: &ShowRecord, (u, v): (f64, f64), glyphs: &[PlacedGlyph]) {
        writer.operation("Tm", &matrix_operands(&Matrix::translate(u, v).then(&record.text_matrix)));
        self.write(writer, glyphs, None);
    }

    pub fn restore(writer: &mut ContentWriter, record: &ShowRecord) {
        writer.operation("Tm", &matrix_operands(&record.line_matrix));
        let after = Matrix::translate(record.end, 0.0).then(&record.text_matrix);
        let Some(inverse) = record.line_matrix.invert() else { return };
        let (pen, _) = inverse.apply(after.e, after.f);
        let unit = record.state.size * record.state.horizontal_scale;
        if pen.abs() > 1e-9 && unit.abs() > f64::EPSILON {
            writer.operation("TJ", &[Object::Array(vec![Object::Real(-pen * 1000.0 / unit)])]);
        }
    }

    pub fn write(&self, writer: &mut ContentWriter, glyphs: &[PlacedGlyph], end: Option<f64>) {
        let mut array: Vec<Object> = Vec::new();
        let mut pen = 0.0;
        let mut current = &self.font;
        let mut color = None;
        for glyph in glyphs {
            if glyph.color != color {
                flush(writer, &mut array);
                self.paint(writer, glyph.color);
                color = glyph.color;
            }
            let wanted = glyph.font.as_ref().unwrap_or(&self.font);
            if wanted != current {
                flush(writer, &mut array);
                writer.operation("Tf", &[Object::Name(wanted.name.clone()), Object::Real(wanted.size)]);
                current = wanted;
            }
            self.push_shift(&mut array, glyph.x - pen, current.size);
            match array.last_mut() {
                Some(Object::String(bytes)) => bytes.extend_from_slice(&glyph.bytes),
                _ => array.push(Object::String(glyph.bytes.clone())),
            }
            pen = glyph.x + glyph.advance;
        }
        if let Some(end) = end {
            self.push_shift(&mut array, end - pen, current.size);
        }
        flush(writer, &mut array);
        if current != &self.font {
            writer.operation("Tf", &[Object::Name(self.font.name.clone()), Object::Real(self.font.size)]);
        }
        if color.is_some() {
            self.paint(writer, None);
        }
    }

    fn paint(&self, writer: &mut ContentWriter, color: Option<[f64; 3]>) {
        match color {
            Some(color) => {
                writer.operation("rg", &color.map(Object::Real));
            }
            None if self.fill.operations.is_empty() => {
                writer.operation("g", &[Object::Int(0)]);
            }
            None => {
                for (operator, operands) in &self.fill.operations {
                    writer.operation(operator, operands);
                }
            }
        }
    }

    fn push_shift(&self, array: &mut Vec<Object>, distance: f64, size: f64) {
        let unit = size * self.state.horizontal_scale;
        if distance.abs() > 1e-9 && unit.abs() > f64::EPSILON {
            array.push(Object::Real(-distance * 1000.0 / unit));
        }
    }
}

fn flush(writer: &mut ContentWriter, array: &mut Vec<Object>) {
    if !array.is_empty() {
        writer.operation("TJ", &[Object::Array(std::mem::take(array))]);
    }
}

fn matrix_operands(matrix: &Matrix) -> [Object; 6] {
    [matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f].map(Object::Real)
}
