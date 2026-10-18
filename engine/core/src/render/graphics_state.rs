use std::rc::Rc;

use tiny_skia::{BlendMode, LineCap, LineJoin, Mask};

use crate::color::{ColorSpace, Rgb};
use crate::geometry::Matrix;
use crate::render::pattern::Pattern;
use crate::text::TextState;

#[derive(Clone)]
pub struct PaintColor {
    pub space: ColorSpace,
    pub components: Vec<f64>,
    pub pattern: Option<Rc<Pattern>>,
}

impl PaintColor {
    pub fn new(space: ColorSpace) -> Self {
        let components = space.initial_color();
        Self { space, components, pattern: None }
    }

    pub fn solid(space: ColorSpace, components: Vec<f64>) -> Self {
        Self { space, components, pattern: None }
    }

    pub fn rgb(&self) -> Option<Rgb> {
        self.space.to_rgb(&self.components)
    }
}

#[derive(Clone)]
pub struct LineStyle {
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
    pub dash: Vec<f64>,
    pub dash_phase: f64,
}

#[derive(Clone)]
pub struct GraphicsState {
    pub ctm: Matrix,
    pub fill: PaintColor,
    pub stroke: PaintColor,
    pub fill_alpha: f32,
    pub stroke_alpha: f32,
    pub blend: BlendMode,
    pub line: LineStyle,
    pub clip: Option<Rc<Mask>>,
    pub text: TextState,
}

impl GraphicsState {
    pub fn new(ctm: Matrix) -> Self {
        Self {
            ctm,
            fill: PaintColor::new(ColorSpace::Gray),
            stroke: PaintColor::new(ColorSpace::Gray),
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            blend: BlendMode::SourceOver,
            line: LineStyle {
                width: 1.0,
                cap: LineCap::Butt,
                join: LineJoin::Miter,
                miter_limit: 10.0,
                dash: Vec::new(),
                dash_phase: 0.0,
            },
            clip: None,
            text: TextState::default(),
        }
    }
}
