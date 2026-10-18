use tiny_skia::{FillRule, Path, PathBuilder};

use crate::content::Operation;
use crate::render::interpreter::Interpreter;
use crate::syntax::Object;

impl Interpreter<'_> {
    pub(super) fn construct(&mut self, operation: &Operation) {
        let n: Vec<f32> = operation.operands.iter().filter_map(Object::as_number).map(|value| value as f32).collect();
        match (operation.operator.as_str(), n.as_slice()) {
            ("m", &[x, y]) => {
                self.path.move_to(x, y);
                self.current_point = (x, y);
            }
            ("l", &[x, y]) => {
                self.ensure_started();
                self.path.line_to(x, y);
                self.current_point = (x, y);
            }
            ("c", &[x1, y1, x2, y2, x, y]) => {
                self.ensure_started();
                self.path.cubic_to(x1, y1, x2, y2, x, y);
                self.current_point = (x, y);
            }
            ("v", &[x2, y2, x, y]) => {
                self.ensure_started();
                let (x1, y1) = self.current_point;
                self.path.cubic_to(x1, y1, x2, y2, x, y);
                self.current_point = (x, y);
            }
            ("y", &[x1, y1, x, y]) => {
                self.ensure_started();
                self.path.cubic_to(x1, y1, x, y, x, y);
                self.current_point = (x, y);
            }
            ("h", _) => self.path.close(),
            ("re", &[x, y, width, height]) => {
                self.path.move_to(x, y);
                self.path.line_to(x + width, y);
                self.path.line_to(x + width, y + height);
                self.path.line_to(x, y + height);
                self.path.close();
                self.current_point = (x, y);
            }
            _ => {}
        }
    }

    fn ensure_started(&mut self) {
        if self.path.is_empty() {
            let (x, y) = self.current_point;
            self.path.move_to(x, y);
        }
    }

    pub(super) fn paint(&mut self, operation: &Operation) {
        let operator = operation.operator.as_str();
        if matches!(operator, "s" | "b" | "b*") {
            self.path.close();
        }
        let path = std::mem::replace(&mut self.path, PathBuilder::new()).finish();
        let clip = self.pending_clip.take();
        let Some(path) = path else { return };
        let even_odd = matches!(operator, "f*" | "B*" | "b*");
        let fill_rule = if even_odd { FillRule::EvenOdd } else { FillRule::Winding };
        if matches!(operator, "f" | "F" | "f*" | "B" | "B*" | "b" | "b*") {
            self.fill_path(&path, fill_rule);
        }
        if matches!(operator, "S" | "s" | "B" | "B*" | "b" | "b*") {
            self.stroke_path(&path);
        }
        if let Some(rule) = clip {
            self.state.clip = self.canvas.clip(self.state.clip.as_ref(), &path, rule, self.user_to_device());
        }
    }

    pub(super) fn fill_path(&mut self, path: &Path, rule: FillRule) {
        if self.state.fill.pattern.is_some() {
            let paint = self.state.fill.clone();
            self.fill_with_pattern(path, rule, &paint, self.state.fill_alpha);
            return;
        }
        let Some(color) = self.state.fill.rgb() else { return };
        let transform = self.user_to_device();
        let clip = self.state.clip.clone();
        self.canvas.fill(
            path,
            color.to_skia(self.state.fill_alpha),
            rule,
            transform,
            clip.as_deref(),
            self.state.blend,
        );
    }

    pub(super) fn stroke_path(&mut self, path: &Path) {
        if self.state.stroke.pattern.is_some() {
            let paint = self.state.stroke.clone();
            self.stroke_with_pattern(path, &paint, self.state.stroke_alpha);
            return;
        }
        let Some(color) = self.state.stroke.rgb() else { return };
        let transform = self.user_to_device();
        let clip = self.state.clip.clone();
        let line = self.state.line.clone();
        self.canvas.stroke(
            path,
            color.to_skia(self.state.stroke_alpha),
            &line,
            transform,
            clip.as_deref(),
            self.state.blend,
        );
    }
}
