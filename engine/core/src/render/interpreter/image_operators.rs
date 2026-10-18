use std::rc::Rc;

use tiny_skia::{FillRule, PathBuilder, Rect};

use crate::color::Rgb;
use crate::content::Operation;
use crate::geometry::Matrix;
use crate::image::{DecodedImage, ImageDecoder, ImageDescription};
use crate::render::interpreter::Interpreter;
use crate::syntax::{Dict, ObjId, Object, Stream};

const PLACEHOLDER_GRAY: f64 = 0.85;

impl Interpreter<'_> {
    pub(super) fn draw_image_xobject(&mut self, id: ObjId, stream: &Stream, resources: &Dict) {
        let context = self.context;
        let Ok(description) = ImageDescription::read(&stream.dict, resources, context.store) else { return };
        let Ok(source) = context.store.object(id) else { return };
        let decoded = context
            .images
            .image(id, source, || ImageDecoder::new(context.store, resources).decode(&description, &stream.raw));
        match decoded {
            Ok(image) => self.draw_image(&image, &description),
            Err(_) => self.draw_placeholder(),
        }
    }

    pub(super) fn draw_inline_image(&mut self, operation: &Operation, resources: &Dict) {
        let (Some(Object::Dict(dict)), Some(Object::String(data))) =
            (operation.operands.first(), operation.operands.get(1))
        else {
            return;
        };
        let store = self.context.store;
        let Ok(description) = ImageDescription::read(dict, resources, store) else { return };
        match ImageDecoder::new(store, resources).decode(&description, data) {
            Ok(image) => self.draw_image(&Rc::new(image), &description),
            Err(_) => self.draw_placeholder(),
        }
    }

    fn draw_image(&mut self, image: &Rc<DecodedImage>, description: &ImageDescription) {
        let unit_to_device = self.user_to_device();
        let (origin_x, origin_y) = unit_to_device.apply(0.0, 0.0);
        let span = |x: f64, y: f64| {
            let (end_x, end_y) = unit_to_device.apply(x, y);
            ((end_x - origin_x).powi(2) + (end_y - origin_y).powi(2)).sqrt()
        };
        let factor_x = (image.width as f64 / span(1.0, 0.0).max(1.0)).floor() as usize;
        let factor_y = (image.height as f64 / span(0.0, 1.0).max(1.0)).floor() as usize;
        let reduced =
            if factor_x > 1 || factor_y > 1 { Rc::new(image.reduced(factor_x, factor_y)) } else { image.clone() };
        let upscaled = span(1.0, 0.0) > image.width as f64 || span(0.0, 1.0) > image.height as f64;
        let smooth = description.interpolate || !upscaled;
        let shown = if description.is_stencil {
            let Some(color) = self.state.fill.rgb() else { return };
            Rc::new(tinted(&reduced, color))
        } else {
            reduced
        };
        let Some(pixmap) = shown.to_pixmap() else { return };
        let image_to_unit = Matrix::new(1.0 / shown.width as f64, 0.0, 0.0, -1.0 / shown.height as f64, 0.0, 1.0);
        let clip = self.state.clip.clone();
        self.canvas.draw_image(
            &pixmap,
            smooth,
            self.state.fill_alpha,
            image_to_unit.then(&unit_to_device),
            clip.as_deref(),
            self.state.blend,
        );
    }

    fn draw_placeholder(&mut self) {
        let Some(rect) = Rect::from_ltrb(0.0, 0.0, 1.0, 1.0) else { return };
        let color = Rgb::gray(PLACEHOLDER_GRAY).to_skia(1.0);
        let clip = self.state.clip.clone();
        self.canvas.fill(
            &PathBuilder::from_rect(rect),
            color,
            FillRule::Winding,
            self.user_to_device(),
            clip.as_deref(),
            self.state.blend,
        );
    }
}

fn tinted(mask: &DecodedImage, color: Rgb) -> DecodedImage {
    let channel = |value: f32| (value * 255.0).round() as u8;
    let mut rgba = mask.rgba.clone();
    for pixel in rgba.chunks_mut(4) {
        pixel[0] = channel(color.r);
        pixel[1] = channel(color.g);
        pixel[2] = channel(color.b);
    }
    DecodedImage { width: mask.width, height: mask.height, rgba }
}
