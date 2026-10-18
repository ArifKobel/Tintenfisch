use std::rc::Rc;

use crate::content::ContentWriter;
use crate::error::Result;
use crate::fonts::{Font, FontCache, FontLibrary};
use crate::forms::default_appearance::DefaultAppearance;
use crate::forms::field::{Field, FieldKind, Widget};
use crate::geometry::Rect;
use crate::store::ObjectStore;
use crate::syntax::{Dict, Name, Object};

const PADDING: f64 = 2.0;
const LARGEST_AUTO_SIZE: f64 = 12.0;
const SMALLEST_AUTO_SIZE: f64 = 4.0;
const LEADING: f64 = 1.15;
const SELECTED: [f64; 3] = [0.6, 0.75, 0.94];
const BEZIER_CIRCLE: f64 = 0.552_284_75;

pub struct WidgetAppearance {
    pub width: f64,
    pub height: f64,
    pub content: Vec<u8>,
    pub resources: Dict,
}

pub struct AppearanceWriter<'a> {
    store: &'a ObjectStore,
    fonts: &'a FontCache,
    library: &'a FontLibrary,
    default_resources: Dict,
}

struct SetFont {
    name: Name,
    reference: Object,
    font: Rc<Font>,
}

impl<'a> AppearanceWriter<'a> {
    pub fn new(
        store: &'a ObjectStore,
        fonts: &'a FontCache,
        library: &'a FontLibrary,
        default_resources: Dict,
    ) -> Self {
        Self { store, fonts, library, default_resources }
    }

    pub fn text(&self, field: &Field, widget: &Widget, text: &str, chosen: &[String]) -> Result<WidgetAppearance> {
        let (width, height) = (widget.rect.width(), widget.rect.height());
        let appearance = DefaultAppearance::parse(
            widget.dict.get("DA").and_then(Object::as_string).or(field.default_appearance.as_deref()),
        );
        let font = self.font(&appearance.font)?;
        let border = self.frame(widget);
        let inner = Rect::new(border + 1.0, border + 1.0, width - border - 1.0, height - border - 1.0);
        let mut writer = ContentWriter::new();
        self.write_frame(&mut writer, widget, (width, height), border, false);
        writer.operation("BMC", &[Object::Name(Name::from("Tx"))]);
        writer.operation("q", &[]);
        writer.operation("re", &numbers(&[inner.x0, inner.y0, inner.width().max(0.0), inner.height().max(0.0)]));
        writer.operation("W", &[]).operation("n", &[]);
        let mut resources = Dict::new();
        if let Some(font) = &font {
            let lines: Vec<(String, bool)> = match &field.kind {
                FieldKind::Choice { combo: false, options, .. } => {
                    options.iter().map(|option| (option.label.clone(), chosen.contains(&option.value))).collect()
                }
                FieldKind::Text { password: true, .. } => vec![("*".repeat(text.chars().count()), false)],
                _ => vec![(text.to_string(), false)],
            };
            self.write_text(&mut writer, field, font, &appearance, inner, &lines);
            resources.insert("Font", Object::Dict([(font.name.clone(), font.reference.clone())].into_iter().collect()));
        }
        writer.operation("Q", &[]).operation("EMC", &[]);
        Ok(WidgetAppearance { width, height, content: writer.finish(), resources })
    }

    pub fn states(&self, field: &Field, widget: &Widget) -> (WidgetAppearance, WidgetAppearance) {
        let (width, height) = (widget.rect.width(), widget.rect.height());
        let border = self.frame(widget);
        let appearance = DefaultAppearance::parse(
            widget.dict.get("DA").and_then(Object::as_string).or(field.default_appearance.as_deref()),
        );
        let round = field.kind == FieldKind::Radio;
        let mut off = ContentWriter::new();
        self.write_frame(&mut off, widget, (width, height), border, round);
        let mut on = ContentWriter::new();
        self.write_frame(&mut on, widget, (width, height), border, round);
        appearance.write_color(&mut on);
        let (cx, cy, size) = (width / 2.0, height / 2.0, width.min(height) - 2.0 * (border + PADDING));
        if field.kind == FieldKind::Radio {
            circle(&mut on, cx, cy, size / 2.0 * 0.6);
            on.operation("f", &[]);
        } else {
            let point = |x: f64, y: f64| [cx + size * (x - 0.5), cy + size * (y - 0.5)];
            on.operation("w", &[Object::Real((size * 0.12).max(1.0))])
                .operation("J", &[Object::Int(1)])
                .operation("j", &[Object::Int(1)]);
            on.operation("RG", &numbers(&[0.0, 0.0, 0.0]));
            on.operation("m", &numbers(&point(0.15, 0.5)));
            on.operation("l", &numbers(&point(0.4, 0.2)));
            on.operation("l", &numbers(&point(0.85, 0.85)));
            on.operation("S", &[]);
        }
        let state = |content: Vec<u8>| WidgetAppearance { width, height, content, resources: Dict::new() };
        (state(on.finish()), state(off.finish()))
    }

    fn frame(&self, widget: &Widget) -> f64 {
        let has_border = widget
            .dict
            .get("MK")
            .and_then(Object::as_dict)
            .is_some_and(|look| look.get("BC").and_then(Object::as_array).is_some_and(|color| !color.is_empty()));
        if !has_border {
            return 0.0;
        }
        widget
            .dict
            .get("BS")
            .and_then(Object::as_dict)
            .and_then(|style| style.get("W"))
            .and_then(Object::as_number)
            .unwrap_or(1.0)
    }

    fn write_frame(
        &self,
        writer: &mut ContentWriter,
        widget: &Widget,
        (width, height): (f64, f64),
        border: f64,
        round: bool,
    ) {
        let look = widget.dict.get("MK").and_then(Object::as_dict);
        let outline = |writer: &mut ContentWriter, inset: f64| {
            if round {
                circle(writer, width / 2.0, height / 2.0, (width.min(height) - inset * 2.0) / 2.0);
            } else {
                writer.operation("re", &numbers(&[inset, inset, width - 2.0 * inset, height - 2.0 * inset]));
            }
        };
        if let Some(background) = look.and_then(|look| look.get("BG")).and_then(color_operator(false)) {
            writer.operation(background.0, &background.1);
            outline(writer, 0.0);
            writer.operation("f", &[]);
        }
        if border > 0.0
            && let Some(stroke) = look.and_then(|look| look.get("BC")).and_then(color_operator(true))
        {
            writer.operation(stroke.0, &stroke.1).operation("w", &[Object::Real(border)]);
            outline(writer, border / 2.0);
            writer.operation("S", &[]);
        }
    }

    fn write_text(
        &self,
        writer: &mut ContentWriter,
        field: &Field,
        font: &SetFont,
        appearance: &DefaultAppearance,
        inner: Rect,
        lines: &[(String, bool)],
    ) {
        let (ascent, descent) = font.font.vertical_extent();
        let extent = (ascent - descent).max(0.5);
        let available = inner.width() - 2.0 * PADDING;
        let multiline =
            matches!(field.kind, FieldKind::Text { multiline: true, .. } | FieldKind::Choice { combo: false, .. });
        let size = match appearance.size {
            size if size > 0.0 => size,
            _ if multiline => LARGEST_AUTO_SIZE,
            _ => {
                let text_width = lines.first().map_or(0.0, |(text, _)| self.width(&font.font, text));
                let fits_height = inner.height() / extent;
                let fits_width = if text_width > 0.0 { available / text_width } else { f64::INFINITY };
                fits_height.min(fits_width).clamp(SMALLEST_AUTO_SIZE, LARGEST_AUTO_SIZE)
            }
        };
        let wrapped: Vec<(String, bool)> = match field.kind {
            FieldKind::Text { multiline: true, .. } => lines
                .iter()
                .flat_map(|(text, chosen)| {
                    self.wrap(&font.font, text, available / size).into_iter().map(move |line| (line, *chosen))
                })
                .collect(),
            _ => lines.to_vec(),
        };
        let leading = size * LEADING;
        let first_baseline = if multiline {
            inner.y1 - PADDING - ascent * size
        } else {
            inner.y0 + (inner.height() - extent * size) / 2.0 - descent * size
        };
        for (index, (_, chosen)) in wrapped.iter().enumerate() {
            if *chosen {
                let baseline = first_baseline - leading * index as f64;
                writer.operation("rg", &numbers(&SELECTED));
                writer
                    .operation("re", &numbers(&[inner.x0, baseline + descent * size, inner.width(), leading]))
                    .operation("f", &[]);
            }
        }
        writer.operation("BT", &[]);
        appearance.write_color(writer);
        writer.operation("Tf", &[Object::Name(font.name.clone()), Object::Real(size)]);
        for (index, (text, _)) in wrapped.iter().enumerate() {
            let baseline = first_baseline - leading * index as f64;
            match field.kind {
                FieldKind::Text { comb: Some(cells), .. } if cells > 0 => {
                    let cell = inner.width() / cells as f64;
                    for (position, character) in text.chars().take(cells).enumerate() {
                        let glyph = character.to_string();
                        let x =
                            inner.x0 + cell * position as f64 + (cell - self.width(&font.font, &glyph) * size) / 2.0;
                        self.show(writer, &font.font, &glyph, x, baseline);
                    }
                }
                _ => {
                    let text_width = self.width(&font.font, text) * size;
                    let x = match field.quadding {
                        1 => inner.x0 + (inner.width() - text_width) / 2.0,
                        2 => inner.x1 - PADDING - text_width,
                        _ => inner.x0 + PADDING,
                    };
                    self.show(writer, &font.font, text, x, baseline);
                }
            }
        }
        writer.operation("ET", &[]);
    }

    fn show(&self, writer: &mut ContentWriter, font: &Font, text: &str, x: f64, baseline: f64) {
        writer.operation("Tm", &numbers(&[1.0, 0.0, 0.0, 1.0, x, baseline]));
        writer.operation("Tj", &[Object::String(encode(font, text))]);
    }

    fn wrap(&self, font: &Font, text: &str, available: f64) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let mut line = String::new();
            for word in paragraph.split(' ') {
                let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if !line.is_empty() && self.width(font, &candidate) > available {
                    lines.push(std::mem::replace(&mut line, word.to_string()));
                } else {
                    line = candidate;
                }
            }
            lines.push(line);
        }
        lines
    }

    fn width(&self, font: &Font, text: &str) -> f64 {
        font.decode(&encode(font, text)).iter().map(|glyph| glyph.width).sum()
    }

    fn font(&self, name: &Name) -> Result<Option<SetFont>> {
        let Some(fonts) = self.default_resources.get("Font") else { return Ok(None) };
        let fonts = self.store.resolve(fonts)?;
        let Some(reference) = fonts.as_dict().and_then(|fonts| fonts.get_name(name)).cloned() else { return Ok(None) };
        let font = self.fonts.font(&reference, self.store, self.library)?;
        Ok(Some(SetFont { name: name.clone(), reference, font }))
    }
}

fn encode(font: &Font, text: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    for character in text.chars() {
        if let Some((code, length)) = font.encode(character).or_else(|| font.encode('?')) {
            bytes.extend(Font::encode_code(code, length));
        }
    }
    bytes
}

fn numbers(values: &[f64]) -> Vec<Object> {
    values.iter().map(|value| Object::Real(*value)).collect()
}

fn color_operator(stroking: bool) -> impl Fn(&Object) -> Option<(&'static str, Vec<Object>)> {
    move |color| {
        let components: Vec<Object> =
            color.as_array()?.iter().filter_map(Object::as_number).map(Object::Real).collect();
        let operator = match (components.len(), stroking) {
            (1, false) => "g",
            (1, true) => "G",
            (3, false) => "rg",
            (3, true) => "RG",
            (4, false) => "k",
            (4, true) => "K",
            _ => return None,
        };
        Some((operator, components))
    }
}

fn circle(writer: &mut ContentWriter, cx: f64, cy: f64, radius: f64) {
    let k = radius * BEZIER_CIRCLE;
    writer.operation("m", &numbers(&[cx + radius, cy]));
    for [a, b, c, d, e, f] in [
        [cx + radius, cy + k, cx + k, cy + radius, cx, cy + radius],
        [cx - k, cy + radius, cx - radius, cy + k, cx - radius, cy],
        [cx - radius, cy - k, cx - k, cy - radius, cx, cy - radius],
        [cx + k, cy - radius, cx + radius, cy - k, cx + radius, cy],
    ] {
        writer.operation("c", &numbers(&[a, b, c, d, e, f]));
    }
}
