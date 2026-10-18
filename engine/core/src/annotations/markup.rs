use crate::text::TextPosition;

pub type DisplayPoint = (f64, f64);

#[derive(Clone, Debug, PartialEq)]
pub enum Markup {
    Highlight { anchor: TextPosition, focus: TextPosition },
    Note { at: DisplayPoint, text: String },
    Ink { strokes: Vec<Vec<DisplayPoint>> },
    Rectangle { from: DisplayPoint, to: DisplayPoint },
    Ellipse { from: DisplayPoint, to: DisplayPoint },
    Arrow { from: DisplayPoint, to: DisplayPoint },
    TextBox { from: DisplayPoint, to: DisplayPoint, text: String },
    RedactArea { from: DisplayPoint, to: DisplayPoint },
    RedactText { anchor: TextPosition, focus: TextPosition },
    Stamp { at: DisplayPoint, label: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkupStyle {
    pub color: [f64; 3],
    pub width: f64,
    pub author: String,
    pub date: String,
}
