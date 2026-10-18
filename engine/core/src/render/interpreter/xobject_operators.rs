use tiny_skia::{FillRule, PathBuilder, Rect};

use crate::content::{ContentParser, Operation};
use crate::geometry::{self, Matrix};
use crate::render::interpreter::Interpreter;
use crate::syntax::{Dict, ObjId, Stream};

impl Interpreter<'_> {
    pub(super) fn draw_xobject(&mut self, operation: &Operation, resources: &Dict) {
        let Some(reference) = operation.operands.first().and_then(|name| self.resource(resources, "XObject", name))
        else {
            return;
        };
        let Some(id) = reference.as_reference() else { return };
        let Ok(object) = self.context.store.object(id) else { return };
        let Some(stream) = object.as_stream() else { return };
        match stream.dict.name("Subtype").map(|subtype| subtype.as_bytes()) {
            Some(b"Image") => self.draw_image_xobject(id, stream, resources),
            Some(b"Form") => self.draw_form(id, stream, resources),
            _ => {}
        }
    }

    pub fn draw_appearance(&mut self, id: ObjId, placement: Matrix, resources: &Dict) {
        let Ok(object) = self.context.store.object(id) else { return };
        let Some(stream) = object.as_stream() else { return };
        self.save();
        self.state.ctm = placement.then(&self.state.ctm);
        self.draw_form(id, stream, resources);
        self.restore();
    }

    pub fn draw_generated(&mut self, content: &[u8], resources: &Dict, bounds: geometry::Rect, placement: Matrix) {
        self.save();
        self.state.ctm = placement.then(&self.state.ctm);
        let operations = ContentParser::new(content, 0).parse(0);
        self.draw_operations(&operations, resources, Matrix::IDENTITY, Some(bounds));
        self.restore();
    }

    fn draw_form(&mut self, id: ObjId, stream: &Stream, resources: &Dict) {
        if self.nested_too_deep() || self.forms.contains(&id) {
            return;
        }
        let Ok(decoded) = self.context.store.decode_stream(stream) else { return };
        let operations = ContentParser::new(&decoded.data, 0).parse(0);
        let form_resources = stream
            .dict
            .get("Resources")
            .and_then(|resources| self.context.store.resolve(resources).ok())
            .and_then(|resources| resources.as_dict().cloned())
            .unwrap_or_else(|| resources.clone());
        let matrix = stream.dict.get("Matrix").and_then(Matrix::from_object).unwrap_or(Matrix::IDENTITY);
        let bounds = stream.dict.get("BBox").and_then(geometry::Rect::from_object);
        self.forms.push(id);
        self.draw_operations(&operations, &form_resources, matrix, bounds);
        self.forms.pop();
    }

    fn draw_operations(
        &mut self,
        operations: &[Operation],
        resources: &Dict,
        matrix: Matrix,
        bounds: Option<geometry::Rect>,
    ) {
        self.save();
        self.state.ctm = matrix.then(&self.state.ctm);
        if let Some(bounds) = bounds
            && let Some(rect) = Rect::from_ltrb(bounds.x0 as f32, bounds.y0 as f32, bounds.x1 as f32, bounds.y1 as f32)
        {
            let path = PathBuilder::from_rect(rect);
            self.state.clip =
                self.canvas.clip(self.state.clip.as_ref(), &path, FillRule::Winding, self.user_to_device());
        }
        let base_ctm = std::mem::replace(&mut self.base_ctm, self.state.ctm);
        self.run(operations, resources);
        self.base_ctm = base_ctm;
        self.restore();
    }
}
