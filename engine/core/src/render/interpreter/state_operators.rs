use std::rc::Rc;

use tiny_skia::{BlendMode, LineCap, LineJoin};

use crate::color::ColorSpace;
use crate::content::Operation;
use crate::geometry::Matrix;
use crate::render::graphics_state::PaintColor;
use crate::render::interpreter::Interpreter;
use crate::render::pattern::Pattern;
use crate::syntax::{Dict, Object};

impl Interpreter<'_> {
    pub(super) fn concat(&mut self, operation: &Operation) {
        let numbers: Vec<f64> = operation.operands.iter().filter_map(Object::as_number).collect();
        if let Some(matrix) = Matrix::from_numbers(&numbers) {
            self.state.ctm = matrix.then(&self.state.ctm);
        }
    }

    pub(super) fn line_or_external_state(&mut self, operation: &Operation, resources: &Dict) {
        let first = operation.operands.first();
        match operation.operator.as_str() {
            "w" => self.set_line_width(first),
            "J" => self.set_line_cap(first),
            "j" => self.set_line_join(first),
            "M" => self.set_miter_limit(first),
            "d" => self.set_dash(first, operation.operands.get(1)),
            _ => {
                let Some(state) = first.and_then(|name| self.resource(resources, "ExtGState", name)) else {
                    return;
                };
                if let Ok(state) = self.context.store.resolve(&state)
                    && let Some(state) = state.as_dict()
                {
                    self.apply_external_state(state);
                }
            }
        }
    }

    fn set_line_width(&mut self, value: Option<&Object>) {
        if let Some(width) = value.and_then(Object::as_number) {
            self.state.line.width = width;
        }
    }

    fn set_line_cap(&mut self, value: Option<&Object>) {
        self.state.line.cap = match value.and_then(Object::as_int) {
            Some(1) => LineCap::Round,
            Some(2) => LineCap::Square,
            _ => LineCap::Butt,
        };
    }

    fn set_line_join(&mut self, value: Option<&Object>) {
        self.state.line.join = match value.and_then(Object::as_int) {
            Some(1) => LineJoin::Round,
            Some(2) => LineJoin::Bevel,
            _ => LineJoin::Miter,
        };
    }

    fn set_miter_limit(&mut self, value: Option<&Object>) {
        if let Some(limit) = value.and_then(Object::as_number) {
            self.state.line.miter_limit = limit;
        }
    }

    fn set_dash(&mut self, lengths: Option<&Object>, phase: Option<&Object>) {
        self.state.line.dash =
            lengths.and_then(Object::as_array).unwrap_or_default().iter().filter_map(Object::as_number).collect();
        self.state.line.dash_phase = phase.and_then(Object::as_number).unwrap_or(0.0);
    }

    fn apply_external_state(&mut self, state: &Dict) {
        self.set_line_width(state.get("LW"));
        if state.contains("LC") {
            self.set_line_cap(state.get("LC"));
        }
        if state.contains("LJ") {
            self.set_line_join(state.get("LJ"));
        }
        self.set_miter_limit(state.get("ML"));
        if let Some(dash) = state.get("D").and_then(Object::as_array) {
            self.set_dash(dash.first(), dash.get(1));
        }
        if let Some(alpha) = state.get("CA").and_then(Object::as_number) {
            self.state.stroke_alpha = alpha as f32;
        }
        if let Some(alpha) = state.get("ca").and_then(Object::as_number) {
            self.state.fill_alpha = alpha as f32;
        }
        if let Some(mode) = state.get("BM").and_then(blend_mode) {
            self.state.blend = mode;
        }
        if let Some([font, size]) = state.get("Font").and_then(Object::as_array) {
            let context = self.context;
            if let Ok(font) = context.fonts.font(font, context.store, context.library) {
                self.state.text.font = Some(font);
                self.state.text.size = size.as_number().unwrap_or(self.state.text.size);
            }
        }
    }

    pub(super) fn color(&mut self, operation: &Operation, resources: &Dict) {
        if self.locked_color {
            return;
        }
        let numbers: Vec<f64> = operation.operands.iter().filter_map(Object::as_number).collect();
        let stroking = operation.operator.chars().next().is_some_and(|first| first.is_ascii_uppercase());
        let paint = match operation.operator.as_str() {
            "CS" | "cs" => {
                let Some(name) = operation.operands.first() else { return };
                match ColorSpace::read(name, resources, self.context.store) {
                    Ok(space) => PaintColor::new(space),
                    Err(_) => return,
                }
            }
            "SC" | "SCN" | "sc" | "scn" => {
                let space = if stroking { &self.state.stroke.space } else { &self.state.fill.space }.clone();
                let pattern = operation
                    .operands
                    .last()
                    .filter(|_| matches!(space, ColorSpace::Pattern(_)))
                    .and_then(|name| self.resource(resources, "Pattern", name))
                    .and_then(|pattern| Pattern::read(&pattern, resources, self.context.store).ok())
                    .map(Rc::new);
                if numbers.is_empty() && pattern.is_none() {
                    return;
                }
                PaintColor { space, components: numbers, pattern }
            }
            "G" | "g" => PaintColor::solid(ColorSpace::Gray, numbers),
            "RG" | "rg" => PaintColor::solid(ColorSpace::Rgb, numbers),
            _ => PaintColor::solid(ColorSpace::Cmyk, numbers),
        };
        if stroking {
            self.state.stroke = paint;
        } else {
            self.state.fill = paint;
        }
    }
}

fn blend_mode(entry: &Object) -> Option<BlendMode> {
    let names = match entry {
        Object::Array(items) => items.iter().filter_map(Object::as_name).collect(),
        other => other.as_name().into_iter().collect::<Vec<_>>(),
    };
    names.into_iter().find_map(|name| {
        Some(match name.as_bytes() {
            b"Normal" | b"Compatible" => BlendMode::SourceOver,
            b"Multiply" => BlendMode::Multiply,
            b"Screen" => BlendMode::Screen,
            b"Overlay" => BlendMode::Overlay,
            b"Darken" => BlendMode::Darken,
            b"Lighten" => BlendMode::Lighten,
            b"ColorDodge" => BlendMode::ColorDodge,
            b"ColorBurn" => BlendMode::ColorBurn,
            b"HardLight" => BlendMode::HardLight,
            b"SoftLight" => BlendMode::SoftLight,
            b"Difference" => BlendMode::Difference,
            b"Exclusion" => BlendMode::Exclusion,
            b"Hue" => BlendMode::Hue,
            b"Saturation" => BlendMode::Saturation,
            b"Color" => BlendMode::Color,
            b"Luminosity" => BlendMode::Luminosity,
            _ => return None,
        })
    })
}
