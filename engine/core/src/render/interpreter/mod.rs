mod image_operators;
mod path_operators;
mod pattern_operators;
mod state_operators;
mod text_operators;
mod xobject_operators;

use tiny_skia::{FillRule, PathBuilder};

use crate::content::{self, Operation};
use crate::geometry::Matrix;
use crate::render::RenderContext;
use crate::render::canvas::Canvas;
use crate::render::graphics_state::GraphicsState;
use crate::syntax::{Dict, ObjId, Object};
use crate::text::TextMachine;

const MAX_NESTING: usize = 16;

pub struct Interpreter<'a> {
    context: RenderContext<'a>,
    canvas: &'a mut Canvas,
    device: Matrix,
    state: GraphicsState,
    saved: Vec<GraphicsState>,
    path: PathBuilder,
    current_point: (f32, f32),
    pending_clip: Option<FillRule>,
    text: TextMachine,
    text_clip: Option<PathBuilder>,
    forms: Vec<ObjId>,
    base_ctm: Matrix,
    locked_color: bool,
}

impl<'a> Interpreter<'a> {
    pub fn new(context: RenderContext<'a>, canvas: &'a mut Canvas, device: Matrix) -> Self {
        Self {
            context,
            canvas,
            device,
            state: GraphicsState::new(Matrix::IDENTITY),
            saved: Vec::new(),
            path: PathBuilder::new(),
            current_point: (0.0, 0.0),
            pending_clip: None,
            text: TextMachine::default(),
            text_clip: None,
            forms: Vec::new(),
            base_ctm: Matrix::IDENTITY,
            locked_color: false,
        }
    }

    pub fn run(&mut self, operations: &[Operation], resources: &Dict) {
        for operation in operations {
            self.execute(operation, resources);
        }
    }

    fn execute(&mut self, operation: &Operation, resources: &Dict) {
        match operation.operator.as_str() {
            "q" => self.save(),
            "Q" => self.restore(),
            "cm" => self.concat(operation),
            "w" | "J" | "j" | "M" | "d" | "gs" => self.line_or_external_state(operation, resources),
            "CS" | "cs" | "SC" | "SCN" | "sc" | "scn" | "G" | "g" | "RG" | "rg" | "K" | "k" => {
                self.color(operation, resources)
            }
            "m" | "l" | "c" | "v" | "y" | "h" | "re" => self.construct(operation),
            "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => self.paint(operation),
            "W" => self.pending_clip = Some(FillRule::Winding),
            "W*" => self.pending_clip = Some(FillRule::EvenOdd),
            "BT" | "ET" | "Tc" | "Tw" | "Tz" | "TL" | "Tf" | "Tr" | "Ts" | "Td" | "TD" | "Tm" | "T*" | "Tj" | "TJ"
            | "'" | "\"" => self.text(operation, resources),
            "Do" => self.draw_xobject(operation, resources),
            "BI" => self.draw_inline_image(operation, resources),
            "sh" => self.paint_shading_operator(operation, resources),
            _ => {}
        }
    }

    fn save(&mut self) {
        self.saved.push(self.state.clone());
    }

    fn restore(&mut self) {
        if let Some(state) = self.saved.pop() {
            self.state = state;
        }
    }

    fn user_to_device(&self) -> Matrix {
        self.state.ctm.then(&self.device)
    }

    fn resource(&self, resources: &Dict, category: &str, name: &Object) -> Option<Object> {
        content::resource(self.context.store, resources, category, name)
    }

    fn nested_too_deep(&self) -> bool {
        self.forms.len() >= MAX_NESTING
    }
}
