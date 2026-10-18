use crate::geometry::Rect;
use crate::text::GlyphOrigin;

#[derive(Clone, Debug, PartialEq)]
pub struct TextGlyph {
    pub origin: GlyphOrigin,
    pub text: String,
    pub angle: f64,
    pub bounds: Rect,
    pub baseline: f64,
    pub start: f64,
    pub end: f64,
    pub size: f64,
    pub hidden: bool,
}

pub fn to_line_space(angle: f64, (x, y): (f64, f64)) -> (f64, f64) {
    let (sin, cos) = angle.sin_cos();
    (x * cos + y * sin, -x * sin + y * cos)
}

pub fn to_display_space(angle: f64, (u, v): (f64, f64)) -> (f64, f64) {
    let (sin, cos) = angle.sin_cos();
    (u * cos - v * sin, u * sin + v * cos)
}
