use ttf_parser::Face;

use crate::annotations::markup::{DisplayPoint, Markup, MarkupStyle};
use crate::annotations::text_box_appearance::{TextBoxAppearance, win_ansi};
use crate::content::ContentWriter;
use crate::error::{Error, Result};
use crate::fonts::{FallbackFace, FallbackFamily, FontLibrary};
use crate::geometry::{Matrix, Rect};
use crate::pages::Page;
use crate::render::DisplaySpace;
use crate::syntax::{Dict, Name, Object, encode_text};
use crate::text::{TextLayout, TextPosition, to_display_space};

const NOTE_SIZE: f64 = 20.0;
const STAMP_SIZE: f64 = 16.0;
const STAMP_PADDING: f64 = 8.0;
const STAMP_FONT: &str = "Helvetica-Bold";
const TEXT_BOX_SIZE: f64 = 12.0;
const BEZIER_CIRCLE: f64 = 0.552_284_75;

pub struct BuiltMarkup {
    pub subtype: &'static str,
    pub rect: Rect,
    pub entries: Dict,
    pub appearance: Vec<u8>,
    pub resources: Dict,
    pub has_popup: bool,
}

pub struct MarkupBuilder<'a> {
    to_user: Matrix,
    layout: &'a TextLayout,
    library: &'a FontLibrary,
}

impl<'a> MarkupBuilder<'a> {
    pub fn new(page: &Page, layout: &'a TextLayout, library: &'a FontLibrary) -> Result<Self> {
        Ok(Self { to_user: DisplaySpace::of(page)?.display_to_user(), layout, library })
    }

    pub fn build(&self, markup: &Markup, style: &MarkupStyle) -> Result<BuiltMarkup> {
        match markup {
            Markup::Highlight { anchor, focus } => self.highlight(*anchor, *focus, style),
            Markup::Note { at, text } => Ok(self.note(*at, text, style)),
            Markup::Ink { strokes } => self.ink(strokes, style),
            Markup::Rectangle { from, to } => Ok(self.shape("Square", *from, *to, style)),
            Markup::Ellipse { from, to } => Ok(self.shape("Circle", *from, *to, style)),
            Markup::Arrow { from, to } => Ok(self.arrow(*from, *to, style)),
            Markup::Stamp { at, label } => Ok(self.stamp(*at, label, style)),
            Markup::TextBox { from, to, text } => Ok(self.text_box(*from, *to, text, style)),
            Markup::RedactArea { from, to } => {
                let (x0, y0, x1, y1) = (from.0.min(to.0), from.1.min(to.1), from.0.max(to.0), from.1.max(to.1));
                let corners = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)].map(|point| self.user(point));
                Ok(self.redaction(vec![corners], String::new(), style))
            }
            Markup::RedactText { anchor, focus } => {
                let quads = self.text_quads(*anchor, *focus)?;
                Ok(self.redaction(quads, self.layout.selected_text(*anchor, *focus), style))
            }
        }
    }

    fn user(&self, (x, y): DisplayPoint) -> (f64, f64) {
        self.to_user.apply(x, y)
    }

    fn upright_frame(&self, left: f64, bottom: f64) -> Matrix {
        Matrix::new(1.0, 0.0, 0.0, -1.0, left, bottom).then(&self.to_user)
    }

    fn text_quads(&self, anchor: TextPosition, focus: TextPosition) -> Result<Vec<[(f64, f64); 4]>> {
        let boxes = self.layout.selection_boxes(anchor, focus);
        if boxes.is_empty() {
            return Err(Error::unsupported("there is no text there"));
        }
        Ok(boxes
            .iter()
            .map(|text_box| {
                let corner = |x: f64, y: f64| self.user(to_display_space(text_box.angle, (x, y)));
                let rect = text_box.rect;
                [corner(rect.x0, rect.y0), corner(rect.x1, rect.y0), corner(rect.x0, rect.y1), corner(rect.x1, rect.y1)]
            })
            .collect())
    }

    fn highlight(&self, anchor: TextPosition, focus: TextPosition, style: &MarkupStyle) -> Result<BuiltMarkup> {
        let mut quads = Vec::new();
        let mut content = ContentWriter::new();
        content.operation("gs", &[Object::Name(Name::from("Multiply"))]);
        fill_color(&mut content, style.color);
        for corners in self.text_quads(anchor, focus)? {
            quads.extend(corners.iter().flat_map(|(x, y)| [*x, *y]));
            outline(&mut content, corners);
            content.operation("f", &[]);
        }
        let points: Vec<(f64, f64)> = quads.chunks(2).map(|pair| (pair[0], pair[1])).collect();
        let mut entries = Dict::new();
        entries.insert("QuadPoints", numbers(&quads));
        entries.insert("Contents", text_string(&self.layout.selected_text(anchor, focus)));
        let multiply: Dict = [(Name::from("BM"), Object::Name(Name::from("Multiply")))].into_iter().collect();
        let states: Dict = [(Name::from("Multiply"), Object::Dict(multiply))].into_iter().collect();
        let resources: Dict = [(Name::from("ExtGState"), Object::Dict(states))].into_iter().collect();
        Ok(BuiltMarkup {
            subtype: "Highlight",
            rect: bounds(&points, 0.0),
            entries,
            appearance: content.finish(),
            resources,
            has_popup: false,
        })
    }

    fn note(&self, (left, top): DisplayPoint, text: &str, style: &MarkupStyle) -> BuiltMarkup {
        let mut content = ContentWriter::new();
        concat(&mut content, &self.upright_frame(left, top + NOTE_SIZE));
        fill_color(&mut content, style.color);
        content.operation("RG", &numbers_slice(&[0.2, 0.2, 0.2])).operation("w", &[Object::Real(0.8)]);
        rounded_rect(&mut content, Rect::new(2.0, 6.0, 18.0, 18.0), 2.5);
        move_to(&mut content, (6.0, 6.0));
        line_to(&mut content, (5.0, 2.0));
        line_to(&mut content, (10.0, 6.0));
        content.operation("B", &[]);
        content.operation("RG", &numbers_slice(&[0.2, 0.2, 0.2])).operation("w", &[Object::Real(1.0)]);
        for y in [14.0, 11.0] {
            move_to(&mut content, (5.0, y));
            line_to(&mut content, (15.0, y));
        }
        content.operation("S", &[]);
        let corners = [(left, top), (left + NOTE_SIZE, top + NOTE_SIZE)].map(|point| self.user(point));
        let mut entries = Dict::new();
        entries.insert("Name", Object::Name(Name::from("Comment")));
        entries.insert("Contents", text_string(text));
        entries.insert("Open", Object::Bool(false));
        BuiltMarkup {
            subtype: "Text",
            rect: bounds(&corners, 0.0),
            entries,
            appearance: content.finish(),
            resources: Dict::new(),
            has_popup: true,
        }
    }

    fn ink(&self, strokes: &[Vec<DisplayPoint>], style: &MarkupStyle) -> Result<BuiltMarkup> {
        let strokes: Vec<Vec<(f64, f64)>> = strokes
            .iter()
            .filter(|stroke| !stroke.is_empty())
            .map(|stroke| stroke.iter().map(|point| self.user(*point)).collect())
            .collect();
        if strokes.is_empty() {
            return Err(Error::unsupported("there is nothing drawn"));
        }
        let mut content = ContentWriter::new();
        stroke_style(&mut content, style);
        for stroke in &strokes {
            move_to(&mut content, stroke[0]);
            let rest = if stroke.len() == 1 { &stroke[..] } else { &stroke[1..] };
            for point in rest {
                line_to(&mut content, *point);
            }
        }
        content.operation("S", &[]);
        let mut entries = Dict::new();
        let list = strokes
            .iter()
            .map(|stroke| numbers(&stroke.iter().flat_map(|(x, y)| [*x, *y]).collect::<Vec<_>>()))
            .collect();
        entries.insert("InkList", Object::Array(list));
        entries.insert("BS", border(style.width));
        let points: Vec<(f64, f64)> = strokes.concat();
        Ok(BuiltMarkup {
            subtype: "Ink",
            rect: bounds(&points, style.width),
            entries,
            appearance: content.finish(),
            resources: Dict::new(),
            has_popup: false,
        })
    }

    fn shape(&self, subtype: &'static str, from: DisplayPoint, to: DisplayPoint, style: &MarkupStyle) -> BuiltMarkup {
        let box_ = bounds(&[self.user(from), self.user(to)], 0.0);
        let mut content = ContentWriter::new();
        stroke_style(&mut content, style);
        if subtype == "Square" {
            content.operation("re", &numbers_slice(&[box_.x0, box_.y0, box_.width(), box_.height()]));
        } else {
            ellipse(&mut content, box_);
        }
        content.operation("S", &[]);
        let mut entries = Dict::new();
        entries.insert("BS", border(style.width));
        BuiltMarkup {
            subtype,
            rect: inflate(box_, style.width),
            entries,
            appearance: content.finish(),
            resources: Dict::new(),
            has_popup: false,
        }
    }

    fn arrow(&self, from: DisplayPoint, to: DisplayPoint, style: &MarkupStyle) -> BuiltMarkup {
        let (start, end) = (self.user(from), self.user(to));
        let length = 3.0 * style.width + 6.0;
        let direction = (end.1 - start.1).atan2(end.0 - start.0);
        let wing = |turn: f64| (end.0 - length * (direction + turn).cos(), end.1 - length * (direction + turn).sin());
        let mut content = ContentWriter::new();
        stroke_style(&mut content, style);
        move_to(&mut content, start);
        line_to(&mut content, end);
        move_to(&mut content, wing(0.5));
        line_to(&mut content, end);
        line_to(&mut content, wing(-0.5));
        content.operation("S", &[]);
        let mut entries = Dict::new();
        entries.insert("L", numbers(&[start.0, start.1, end.0, end.1]));
        entries
            .insert("LE", Object::Array(vec![Object::Name(Name::from("None")), Object::Name(Name::from("OpenArrow"))]));
        entries.insert("BS", border(style.width));
        let points = [start, end, wing(0.5), wing(-0.5)];
        BuiltMarkup {
            subtype: "Line",
            rect: bounds(&points, style.width),
            entries,
            appearance: content.finish(),
            resources: Dict::new(),
            has_popup: false,
        }
    }

    fn stamp(&self, (x, y): DisplayPoint, label: &str, style: &MarkupStyle) -> BuiltMarkup {
        let label = label.trim().to_uppercase();
        let width = self.label_width(&label) + 2.0 * STAMP_PADDING;
        let height = STAMP_SIZE + 2.0 * STAMP_PADDING;
        let (left, top) = (x - width / 2.0, y - height / 2.0);
        let mut content = ContentWriter::new();
        concat(&mut content, &self.upright_frame(left, top + height));
        content.operation("RG", &numbers_slice(&style.color)).operation("w", &[Object::Real(2.0)]);
        rounded_rect(&mut content, Rect::new(1.0, 1.0, width - 1.0, height - 1.0), 4.0);
        content.operation("S", &[]);
        fill_color(&mut content, style.color);
        content
            .operation("BT", &[])
            .operation("Tf", &[Object::Name(Name::from("Stamp")), Object::Real(STAMP_SIZE)])
            .operation("Td", &numbers_slice(&[STAMP_PADDING, STAMP_PADDING + STAMP_SIZE * 0.22]))
            .operation("Tj", &[Object::String(win_ansi(&label))])
            .operation("ET", &[]);
        let font: Dict = [
            ("Type", Object::Name(Name::from("Font"))),
            ("Subtype", Object::Name(Name::from("Type1"))),
            ("BaseFont", Object::Name(Name::from(STAMP_FONT))),
            ("Encoding", Object::Name(Name::from("WinAnsiEncoding"))),
        ]
        .into_iter()
        .map(|(key, value)| (Name::from(key), value))
        .collect();
        let fonts: Dict = [(Name::from("Stamp"), Object::Dict(font))].into_iter().collect();
        let resources: Dict = [(Name::from("Font"), Object::Dict(fonts))].into_iter().collect();
        let corners = [(left, top), (left + width, top + height)].map(|point| self.user(point));
        let mut entries = Dict::new();
        entries.insert("Name", Object::Name(Name::from(label.replace(' ', "").as_str())));
        entries.insert("Contents", text_string(&label));
        BuiltMarkup {
            subtype: "Stamp",
            rect: bounds(&corners, 0.0),
            entries,
            appearance: content.finish(),
            resources,
            has_popup: false,
        }
    }

    fn text_box(&self, from: DisplayPoint, to: DisplayPoint, text: &str, style: &MarkupStyle) -> BuiltMarkup {
        let (left, top) = (from.0.min(to.0), from.1.min(to.1));
        let width = (from.0 - to.0).abs().max(TEXT_BOX_SIZE * 2.0);
        let height = (from.1 - to.1).abs().max(TEXT_BOX_SIZE * 1.6);
        let appearance = TextBoxAppearance::new(self.library).content(
            &self.upright_frame(left, top + height),
            width,
            height,
            text,
            TEXT_BOX_SIZE,
            style.color,
        );
        let corners = [(left, top), (left + width, top + height)].map(|point| self.user(point));
        let mut entries = Dict::new();
        entries.insert("Contents", text_string(text));
        entries.insert("DA", Object::String(TextBoxAppearance::default_appearance(TEXT_BOX_SIZE, style.color)));
        BuiltMarkup {
            subtype: "FreeText",
            rect: bounds(&corners, 0.0),
            entries,
            appearance,
            resources: TextBoxAppearance::resources(),
            has_popup: false,
        }
    }

    fn redaction(&self, quads: Vec<[(f64, f64); 4]>, quoted: String, style: &MarkupStyle) -> BuiltMarkup {
        let mut content = ContentWriter::new();
        content.operation("RG", &numbers_slice(&style.color)).operation("w", &[Object::Real(1.0)]);
        for corners in &quads {
            outline(&mut content, *corners);
        }
        content.operation("S", &[]);
        let flat: Vec<f64> = quads.iter().flatten().flat_map(|(x, y)| [*x, *y]).collect();
        let points: Vec<(f64, f64)> = quads.concat();
        let mut entries = Dict::new();
        entries.insert("QuadPoints", numbers(&flat));
        entries.insert("IC", numbers(&[0.0, 0.0, 0.0]));
        entries.insert("Contents", text_string(&quoted));
        BuiltMarkup {
            subtype: "Redact",
            rect: bounds(&points, 0.0),
            entries,
            appearance: content.finish(),
            resources: Dict::new(),
            has_popup: false,
        }
    }

    fn label_width(&self, label: &str) -> f64 {
        let face = FallbackFace { family: FallbackFamily::Sans, bold: true, italic: false };
        let measured = self.library.face(face).and_then(|data| {
            let face = Face::parse(&data, 0).ok()?;
            let per_em = face.units_per_em() as f64;
            Some(
                label
                    .chars()
                    .map(|character| {
                        face.glyph_index(character).and_then(|glyph| face.glyph_hor_advance(glyph)).unwrap_or(0) as f64
                            / per_em
                    })
                    .sum::<f64>(),
            )
        });
        measured.unwrap_or(label.chars().count() as f64 * 0.65) * STAMP_SIZE
    }
}

fn numbers(values: &[f64]) -> Object {
    Object::Array(values.iter().map(|value| Object::Real(*value)).collect())
}

fn numbers_slice(values: &[f64]) -> Vec<Object> {
    values.iter().map(|value| Object::Real(*value)).collect()
}

fn text_string(text: &str) -> Object {
    Object::String(encode_text(text))
}

fn border(width: f64) -> Object {
    let border: Dict = [(Name::from("W"), Object::Real(width)), (Name::from("S"), Object::Name(Name::from("S")))]
        .into_iter()
        .collect();
    Object::Dict(border)
}

fn bounds(points: &[(f64, f64)], margin: f64) -> Rect {
    let x0 = points.iter().map(|point| point.0).fold(f64::INFINITY, f64::min);
    let x1 = points.iter().map(|point| point.0).fold(f64::NEG_INFINITY, f64::max);
    let y0 = points.iter().map(|point| point.1).fold(f64::INFINITY, f64::min);
    let y1 = points.iter().map(|point| point.1).fold(f64::NEG_INFINITY, f64::max);
    inflate(Rect::new(x0, y0, x1, y1), margin)
}

fn inflate(rect: Rect, width: f64) -> Rect {
    let margin = width / 2.0 + 1.0;
    Rect::new(rect.x0 - margin, rect.y0 - margin, rect.x1 + margin, rect.y1 + margin)
}

fn outline(content: &mut ContentWriter, [top_left, top_right, bottom_left, bottom_right]: [(f64, f64); 4]) {
    move_to(content, top_left);
    for point in [top_right, bottom_right, bottom_left] {
        line_to(content, point);
    }
    content.operation("h", &[]);
}

fn concat(content: &mut ContentWriter, matrix: &Matrix) {
    content.operation("cm", &numbers_slice(&[matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f]));
}

fn fill_color(content: &mut ContentWriter, color: [f64; 3]) {
    content.operation("rg", &numbers_slice(&color));
}

fn stroke_style(content: &mut ContentWriter, style: &MarkupStyle) {
    content
        .operation("RG", &numbers_slice(&style.color))
        .operation("w", &[Object::Real(style.width)])
        .operation("J", &[Object::Int(1)])
        .operation("j", &[Object::Int(1)]);
}

fn move_to(content: &mut ContentWriter, (x, y): (f64, f64)) {
    content.operation("m", &numbers_slice(&[x, y]));
}

fn line_to(content: &mut ContentWriter, (x, y): (f64, f64)) {
    content.operation("l", &numbers_slice(&[x, y]));
}

fn curve_to(content: &mut ContentWriter, points: [(f64, f64); 3]) {
    content.operation("c", &numbers_slice(&points.iter().flat_map(|(x, y)| [*x, *y]).collect::<Vec<_>>()));
}

fn rounded_rect(content: &mut ContentWriter, rect: Rect, radius: f64) {
    let k = radius * (1.0 - BEZIER_CIRCLE);
    move_to(content, (rect.x0 + radius, rect.y0));
    line_to(content, (rect.x1 - radius, rect.y0));
    curve_to(content, [(rect.x1 - k, rect.y0), (rect.x1, rect.y0 + k), (rect.x1, rect.y0 + radius)]);
    line_to(content, (rect.x1, rect.y1 - radius));
    curve_to(content, [(rect.x1, rect.y1 - k), (rect.x1 - k, rect.y1), (rect.x1 - radius, rect.y1)]);
    line_to(content, (rect.x0 + radius, rect.y1));
    curve_to(content, [(rect.x0 + k, rect.y1), (rect.x0, rect.y1 - k), (rect.x0, rect.y1 - radius)]);
    line_to(content, (rect.x0, rect.y0 + radius));
    curve_to(content, [(rect.x0, rect.y0 + k), (rect.x0 + k, rect.y0), (rect.x0 + radius, rect.y0)]);
    content.operation("h", &[]);
}

fn ellipse(content: &mut ContentWriter, rect: Rect) {
    let (cx, cy) = ((rect.x0 + rect.x1) / 2.0, (rect.y0 + rect.y1) / 2.0);
    let (rx, ry) = (rect.width() / 2.0, rect.height() / 2.0);
    let (kx, ky) = (rx * BEZIER_CIRCLE, ry * BEZIER_CIRCLE);
    move_to(content, (cx + rx, cy));
    curve_to(content, [(cx + rx, cy + ky), (cx + kx, cy + ry), (cx, cy + ry)]);
    curve_to(content, [(cx - kx, cy + ry), (cx - rx, cy + ky), (cx - rx, cy)]);
    curve_to(content, [(cx - rx, cy - ky), (cx - kx, cy - ry), (cx, cy - ry)]);
    curve_to(content, [(cx + kx, cy - ry), (cx + rx, cy - ky), (cx + rx, cy)]);
    content.operation("h", &[]);
}
