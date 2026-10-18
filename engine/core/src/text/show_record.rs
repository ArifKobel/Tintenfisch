use std::rc::Rc;

use crate::fonts::Font;
use crate::geometry::Matrix;
use crate::syntax::{Name, ObjId};
use crate::text::{FillColor, TextState};

#[derive(Clone, Debug, PartialEq)]
pub struct RecordedGlyph {
    pub code: u32,
    pub byte_length: usize,
    pub x: f64,
    pub advance: f64,
}

#[derive(Clone)]
pub struct ShowRecord {
    pub form: Option<ObjId>,
    pub ordinal: usize,
    pub operator: String,
    pub font_name: Option<Name>,
    pub font: Option<Rc<Font>>,
    pub state: TextState,
    pub fill: FillColor,
    pub text_matrix: Matrix,
    pub line_matrix: Matrix,
    pub to_display: Matrix,
    pub glyphs: Vec<RecordedGlyph>,
    pub end: f64,
}

impl ShowRecord {
    pub fn display_scale(&self) -> f64 {
        let (x0, y0) = self.to_display.apply(0.0, 0.0);
        let (x1, y1) = self.to_display.apply(1.0, 0.0);
        (x1 - x0).hypot(y1 - y0).max(f64::EPSILON)
    }
}
