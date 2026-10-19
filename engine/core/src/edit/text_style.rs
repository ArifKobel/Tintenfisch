use std::rc::Rc;

use crate::fonts::{FallbackFace, FallbackFamily, Font};
use crate::syntax::Name;
use crate::text::ShowRecord;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontPick {
    Document(Name),
    Bundled(FallbackFamily),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStyle {
    pub font: Option<FontPick>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub size: Option<f64>,
    pub color: Option<[f64; 3]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleChange {
    pub start: usize,
    pub end: usize,
    pub style: TextStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClusterStyle {
    pub resource: Option<Name>,
    pub font: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub color: Option<[f64; 3]>,
}

impl ClusterStyle {
    pub fn of(record: &ShowRecord) -> ClusterStyle {
        let face = record.font.as_deref().map(Font::fallback_face);
        ClusterStyle {
            resource: record.font_name.clone(),
            font: record.font.as_deref().and_then(Font::family_name).unwrap_or_default().to_string(),
            size: record.state.size * record.display_scale(),
            bold: face.is_some_and(|face| face.bold),
            italic: face.is_some_and(|face| face.italic),
            color: record.fill.rgb(),
        }
    }
}

#[derive(Clone)]
pub enum FormatFont {
    Kept,
    Document { name: Name, font: Rc<Font> },
    Bundled(FallbackFace),
}

impl std::fmt::Debug for FormatFont {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FormatFont::Kept => formatter.write_str("Kept"),
            FormatFont::Document { name, .. } => write!(formatter, "Document({name:?})"),
            FormatFont::Bundled(face) => write!(formatter, "Bundled({face:?})"),
        }
    }
}

impl PartialEq for FormatFont {
    fn eq(&self, other: &FormatFont) -> bool {
        match (self, other) {
            (FormatFont::Kept, FormatFont::Kept) => true,
            (FormatFont::Document { name, .. }, FormatFont::Document { name: other, .. }) => name == other,
            (FormatFont::Bundled(face), FormatFont::Bundled(other)) => face == other,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlyphFormat {
    pub font: FormatFont,
    pub size: Option<f64>,
    pub color: Option<[f64; 3]>,
}

impl GlyphFormat {
    pub fn resolve(
        style: &TextStyle,
        current: Option<&Font>,
        document: impl Fn(&Name) -> Option<Rc<Font>>,
    ) -> GlyphFormat {
        let current_face = current.map(Font::fallback_face).unwrap_or(FallbackFace {
            family: FallbackFamily::Sans,
            bold: false,
            italic: false,
        });
        let restyled = |family: FallbackFamily| FallbackFace {
            family,
            bold: style.bold.unwrap_or(current_face.bold),
            italic: style.italic.unwrap_or(current_face.italic),
        };
        let font = match &style.font {
            Some(FontPick::Document(name)) => match document(name) {
                Some(font) => FormatFont::Document { name: name.clone(), font },
                None => FormatFont::Bundled(restyled(current_face.family)),
            },
            Some(FontPick::Bundled(family)) => FormatFont::Bundled(restyled(*family)),
            None if style.bold.is_some_and(|bold| bold != current_face.bold)
                || style.italic.is_some_and(|italic| italic != current_face.italic) =>
            {
                FormatFont::Bundled(restyled(current_face.family))
            }
            None => FormatFont::Kept,
        };
        GlyphFormat { font, size: style.size, color: style.color }
    }
}
