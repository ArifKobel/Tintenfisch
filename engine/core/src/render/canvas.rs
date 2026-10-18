use std::rc::Rc;

use tiny_skia::{
    BlendMode, Color, FillRule, FilterQuality, Mask, Paint, Path, Pattern, Pixmap, PixmapPaint, SpreadMode, Stroke,
    StrokeDash,
};

use crate::geometry::Matrix;
use crate::render::graphics_state::LineStyle;

pub struct Repeated<'a> {
    pub pixmap: &'a Pixmap,
    pub to_device: Matrix,
    pub opacity: f32,
}

pub struct Canvas {
    pub pixmap: Pixmap,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Option<Canvas> {
        let mut pixmap = Pixmap::new(width, height)?;
        pixmap.fill(Color::WHITE);
        Some(Canvas { pixmap })
    }

    pub fn transparent(width: u32, height: u32) -> Option<Canvas> {
        Some(Canvas { pixmap: Pixmap::new(width, height)? })
    }

    fn paint(color: Color, blend_mode: BlendMode) -> Paint<'static> {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        paint.blend_mode = blend_mode;
        paint
    }

    pub fn fill(
        &mut self,
        path: &Path,
        color: Color,
        rule: FillRule,
        transform: Matrix,
        clip: Option<&Mask>,
        blend: BlendMode,
    ) {
        self.pixmap.fill_path(path, &Self::paint(color, blend), rule, transform.to_skia(), clip);
    }

    pub fn stroke(
        &mut self,
        path: &Path,
        color: Color,
        line: &LineStyle,
        transform: Matrix,
        clip: Option<&Mask>,
        blend: BlendMode,
    ) {
        let dash = (!line.dash.is_empty() && line.dash.iter().any(|&length| length > 0.0))
            .then(|| {
                let mut lengths: Vec<f32> = line.dash.iter().map(|&length| length.max(0.0) as f32).collect();
                if lengths.len() % 2 == 1 {
                    lengths.extend_from_within(..);
                }
                StrokeDash::new(lengths, line.dash_phase as f32)
            })
            .flatten();
        let stroke = Stroke {
            width: line.width.max(0.0) as f32,
            miter_limit: line.miter_limit.max(1.0) as f32,
            line_cap: line.cap,
            line_join: line.join,
            dash,
        };
        self.pixmap.stroke_path(path, &Self::paint(color, blend), &stroke, transform.to_skia(), clip);
    }

    pub fn draw_image(
        &mut self,
        image: &Pixmap,
        smooth: bool,
        opacity: f32,
        transform: Matrix,
        clip: Option<&Mask>,
        blend_mode: BlendMode,
    ) {
        let quality = if smooth { FilterQuality::Bilinear } else { FilterQuality::Nearest };
        let paint = PixmapPaint { opacity, blend_mode, quality };
        self.pixmap.draw_pixmap(0, 0, image.as_ref(), &paint, transform.to_skia(), clip);
    }

    pub fn fill_with_repeated(
        &mut self,
        path: &Path,
        rule: FillRule,
        transform: Matrix,
        clip: Option<&Mask>,
        repeated: &Repeated,
    ) {
        let Some(inverse) = transform.invert() else { return };
        let to_path_space = repeated.to_device.then(&inverse).to_skia();
        let shader = Pattern::new(
            repeated.pixmap.as_ref(),
            SpreadMode::Repeat,
            FilterQuality::Bilinear,
            repeated.opacity,
            to_path_space,
        );
        let paint = Paint { shader, anti_alias: true, ..Paint::default() };
        self.pixmap.fill_path(path, &paint, rule, transform.to_skia(), clip);
    }

    pub fn clip(&self, current: Option<&Rc<Mask>>, path: &Path, rule: FillRule, transform: Matrix) -> Option<Rc<Mask>> {
        let mut mask = match current {
            Some(current) => (**current).clone(),
            None => {
                let mut full = Mask::new(self.pixmap.width(), self.pixmap.height())?;
                full.data_mut().fill(u8::MAX);
                full
            }
        };
        mask.intersect_path(path, rule, true, transform.to_skia());
        Some(Rc::new(mask))
    }
}
