use tiny_skia::{FillRule, PathBuilder};

use crate::content::{ContentParser, Operation};
use crate::fonts::{Font, FontKind, Glyph, ShownGlyph};
use crate::geometry::Matrix;
use crate::render::interpreter::Interpreter;
use crate::syntax::{Dict, ObjId};
use crate::text::ShowItem;

impl Interpreter<'_> {
    pub(super) fn text(&mut self, operation: &Operation, resources: &Dict) {
        match operation.operator.as_str() {
            "ET" => self.finish_text_clip(),
            "Tf" => {
                let context = self.context;
                let font = operation
                    .operands
                    .first()
                    .and_then(|name| self.resource(resources, "Font", name))
                    .and_then(|font| context.fonts.font(&font, context.store, context.library).ok());
                if let Some(font) = font {
                    self.state.text.font = Some(font);
                }
            }
            _ => {}
        }
        for item in self.text.apply(operation, &mut self.state.text) {
            match item {
                ShowItem::Text(bytes) => self.show(bytes, resources),
                ShowItem::Shift(amount) => self.text.shift(amount, &self.state.text),
            }
        }
    }

    fn show(&mut self, bytes: &[u8], resources: &Dict) {
        let Some(font) = self.state.text.font.clone() else { return };
        for glyph in font.decode(bytes) {
            self.draw_glyph(&font, &glyph, resources);
            self.text.advance(&glyph, &self.state.text);
        }
    }

    fn draw_glyph(&mut self, font: &Font, glyph: &ShownGlyph, resources: &Dict) {
        let mode = self.state.text.render_mode;
        match glyph.glyph {
            Glyph::Outline(id) => {
                if mode == 3 {
                    return;
                }
                let Some(outline) = font.outline(id) else { return };
                let Some(path) = (*outline).clone().transform(self.text.glyph_to_user(&self.state.text).to_skia())
                else {
                    return;
                };
                if matches!(mode, 0 | 2 | 4 | 6) {
                    self.fill_path(&path, FillRule::Winding);
                }
                if matches!(mode, 1 | 2 | 5 | 6) {
                    self.stroke_path(&path);
                }
                if mode >= 4
                    && let Some(device_path) = path.transform(self.user_to_device().to_skia())
                {
                    self.text_clip.get_or_insert_with(PathBuilder::new).push_path(&device_path);
                }
            }
            Glyph::Procedure(procedure) => self.draw_type3_glyph(font, procedure, resources),
            Glyph::Missing => {}
        }
    }

    fn draw_type3_glyph(&mut self, font: &Font, procedure: ObjId, resources: &Dict) {
        let FontKind::Type3(type3) = font.kind() else { return };
        if self.nested_too_deep() || self.forms.contains(&procedure) {
            return;
        }
        let Ok(stream) = self.context.store.object(procedure) else { return };
        let Some(stream) = stream.as_stream() else { return };
        let Ok(decoded) = self.context.store.decode_stream(stream) else { return };
        let operations = ContentParser::new(&decoded.data, 0).parse(0);
        let glyph_resources = type3.resources.clone().unwrap_or_else(|| resources.clone());
        let glyph_ctm = type3.matrix.then(&self.text.glyph_to_user(&self.state.text)).then(&self.state.ctm);
        let text = self.text;
        self.save();
        self.state.ctm = glyph_ctm;
        self.forms.push(procedure);
        self.run(&operations, &glyph_resources);
        self.forms.pop();
        self.restore();
        self.text = text;
    }

    fn finish_text_clip(&mut self) {
        let Some(builder) = self.text_clip.take() else { return };
        if let Some(path) = builder.finish() {
            self.state.clip = self.canvas.clip(self.state.clip.as_ref(), &path, FillRule::Winding, Matrix::IDENTITY);
        }
    }
}
