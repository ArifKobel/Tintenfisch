use std::rc::Rc;

use tiny_skia::{FillRule, Path, PathBuilder, Pixmap, Rect, Stroke, StrokeDash};

use crate::color::{ColorSpace, Rgb};
use crate::content::Operation;
use crate::geometry::Matrix;
use crate::render::canvas::{Canvas, Repeated};
use crate::render::graphics_state::PaintColor;
use crate::render::interpreter::Interpreter;
use crate::render::pattern::{Pattern, TilingPattern};
use crate::shading::{Shading, ShadingRasterizer};
use crate::syntax::Dict;

const MAX_TILE_SIDE: f64 = 4096.0;

impl Interpreter<'_> {
    pub(super) fn paint_shading_operator(&mut self, operation: &Operation, resources: &Dict) {
        let Some(object) = operation.operands.first().and_then(|name| self.resource(resources, "Shading", name)) else {
            return;
        };
        let Ok(shading) = Shading::read(&object, resources, self.context.store) else { return };
        let mut clip = self.state.clip.clone();
        if let Some(bounds) = shading.bbox
            && let Some(rect) = Rect::from_ltrb(bounds.x0 as f32, bounds.y0 as f32, bounds.x1 as f32, bounds.y1 as f32)
        {
            clip = self.canvas.clip(
                clip.as_ref(),
                &PathBuilder::from_rect(rect),
                FillRule::Winding,
                self.user_to_device(),
            );
        }
        self.draw_shading(&shading, self.user_to_device(), clip);
    }

    pub(super) fn fill_with_pattern(&mut self, path: &Path, rule: FillRule, paint: &PaintColor, alpha: f32) {
        let Some(pattern) = paint.pattern.clone() else { return };
        let transform = self.user_to_device();
        match &*pattern {
            Pattern::Shading { shading, matrix } => {
                let Some(area) = self.canvas.clip(self.state.clip.as_ref(), path, rule, transform) else { return };
                if let Some(background) = shading.background.as_ref().and_then(|values| shading.space.to_rgb(values)) {
                    self.canvas.fill(path, background.to_skia(alpha), rule, transform, Some(&area), self.state.blend);
                }
                let to_device = matrix.then(&self.base_ctm).then(&self.device);
                self.draw_shading(shading, to_device, Some(area));
            }
            Pattern::Tiling(tiling) => {
                let color = match &paint.space {
                    ColorSpace::Pattern(Some(base)) if !tiling.colored => base.to_rgb(&paint.components),
                    _ => None,
                };
                let Some((tile, to_device)) = self.render_tile(tiling, color) else { return };
                let clip = self.state.clip.clone();
                let repeated = Repeated { pixmap: &tile, to_device, opacity: alpha };
                self.canvas.fill_with_repeated(path, rule, transform, clip.as_deref(), &repeated);
            }
        }
    }

    pub(super) fn stroke_with_pattern(&mut self, path: &Path, paint: &PaintColor, alpha: f32) {
        let line = &self.state.line;
        let dash = (!line.dash.is_empty())
            .then(|| {
                StrokeDash::new(
                    line.dash.iter().map(|&length| length.max(0.0) as f32).collect(),
                    line.dash_phase as f32,
                )
            })
            .flatten();
        let stroke = Stroke {
            width: line.width.max(0.0) as f32,
            miter_limit: line.miter_limit.max(1.0) as f32,
            line_cap: line.cap,
            line_join: line.join,
            dash,
        };
        if let Some(outline) = path.stroke(&stroke, self.user_to_device().expansion() as f32) {
            self.fill_with_pattern(&outline, FillRule::Winding, paint, alpha);
        }
    }

    fn draw_shading(&mut self, shading: &Shading, to_device: Matrix, clip: Option<Rc<tiny_skia::Mask>>) {
        let (width, height) = (self.canvas.pixmap.width(), self.canvas.pixmap.height());
        let Some(pixmap) = ShadingRasterizer::new(shading, width, height).rasterize(to_device) else { return };
        self.canvas.draw_image(
            &pixmap,
            false,
            self.state.fill_alpha,
            Matrix::IDENTITY,
            clip.as_deref(),
            self.state.blend,
        );
    }

    fn render_tile(&self, tiling: &TilingPattern, color: Option<Rgb>) -> Option<(Pixmap, Matrix)> {
        if self.nested_too_deep() || self.forms.contains(&tiling.id) {
            return None;
        }
        let pattern_to_device = tiling.matrix.then(&self.base_ctm).then(&self.device);
        let scale_x = pattern_to_device.a.hypot(pattern_to_device.b);
        let scale_y = pattern_to_device.c.hypot(pattern_to_device.d);
        let width = (tiling.x_step.abs() * scale_x).ceil().clamp(1.0, MAX_TILE_SIDE);
        let height = (tiling.y_step.abs() * scale_y).ceil().clamp(1.0, MAX_TILE_SIDE);
        let cell_to_tile = Matrix::translate(-tiling.bbox.x0, -tiling.bbox.y0)
            .then(&Matrix::scale(width / tiling.x_step.abs(), height / tiling.y_step.abs()));
        let mut canvas = Canvas::transparent(width as u32, height as u32)?;
        {
            let mut cell = Interpreter::new(self.context, &mut canvas, cell_to_tile);
            cell.forms = self.forms.clone();
            cell.forms.push(tiling.id);
            let bounds = Rect::from_ltrb(
                tiling.bbox.x0 as f32,
                tiling.bbox.y0 as f32,
                tiling.bbox.x1 as f32,
                tiling.bbox.y1 as f32,
            )?;
            cell.state.clip = cell.canvas.clip(None, &PathBuilder::from_rect(bounds), FillRule::Winding, cell_to_tile);
            if let Some(color) = color {
                let solid = PaintColor::solid(ColorSpace::Rgb, vec![color.r as f64, color.g as f64, color.b as f64]);
                cell.state.fill = solid.clone();
                cell.state.stroke = solid;
                cell.locked_color = true;
            }
            cell.run(&tiling.operations, &tiling.resources);
        }
        Some((canvas.pixmap, cell_to_tile.invert()?.then(&pattern_to_device)))
    }
}
