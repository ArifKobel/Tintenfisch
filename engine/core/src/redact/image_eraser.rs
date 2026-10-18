use crate::error::Result;
use crate::geometry::Matrix;
use crate::geometry::Quad;
use crate::image::{ImageDecoder, ImageDescription, ImageStreams};
use crate::store::ObjectStore;
use crate::syntax::Dict;

pub struct ImageEraser<'s> {
    store: &'s ObjectStore,
}

impl<'s> ImageEraser<'s> {
    pub fn new(store: &'s ObjectStore) -> Self {
        Self { store }
    }

    pub fn erase(
        &self,
        dict: &Dict,
        raw: &[u8],
        resources: &Dict,
        matrix: &Matrix,
        areas: &[Quad],
    ) -> Result<Option<ImageStreams>> {
        let description = ImageDescription::read(dict, resources, self.store)?;
        let mut decoded = ImageDecoder::new(self.store, resources).decode(&description, raw)?;
        let (width, height) = (decoded.width, decoded.height);
        let mut erased = false;
        for y in 0..height {
            for x in 0..width {
                let unit = ((x as f64 + 0.5) / width as f64, 1.0 - (y as f64 + 0.5) / height as f64);
                let point = matrix.apply(unit.0, unit.1);
                if areas.iter().any(|area| area.contains(point)) {
                    let pixel = (y * width + x) * 4;
                    let blank = if description.is_stencil { [0, 0, 0, 0] } else { [0, 0, 0, 255] };
                    decoded.rgba[pixel..pixel + 4].copy_from_slice(&blank);
                    erased = true;
                }
            }
        }
        if !erased {
            return Ok(None);
        }
        Ok(Some(if description.is_stencil { ImageStreams::stencil(&decoded) } else { ImageStreams::rgb(&decoded) }))
    }
}
