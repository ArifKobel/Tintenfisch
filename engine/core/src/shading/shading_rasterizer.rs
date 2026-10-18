use tiny_skia::{IntSize, Pixmap};

use crate::color::Rgb;
use crate::geometry::Matrix;
use crate::shading::{Geometry, Shading, Vertex};

const RAMP_SAMPLES: usize = 256;

pub struct ShadingRasterizer<'a> {
    shading: &'a Shading,
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

impl<'a> ShadingRasterizer<'a> {
    pub fn new(shading: &'a Shading, width: u32, height: u32) -> Self {
        let (width, height) = (width as usize, height as usize);
        Self { shading, width, height, rgba: vec![0; width * height * 4] }
    }

    pub fn rasterize(mut self, to_device: Matrix) -> Option<Pixmap> {
        let inverse = to_device.invert()?;
        match &self.shading.geometry {
            Geometry::Axial { start, end, domain, extend } => {
                let ramp = self.ramp(*domain);
                let (dx, dy) = (end.0 - start.0, end.1 - start.1);
                let length = dx * dx + dy * dy;
                self.fill_pixels(&inverse, |x, y| {
                    let t = if length > 0.0 { ((x - start.0) * dx + (y - start.1) * dy) / length } else { 0.0 };
                    extended(t, *extend).and_then(|t| ramp[sample_index(t)])
                });
            }
            Geometry::Radial { start, end, domain, extend } => {
                let ramp = self.ramp(*domain);
                self.fill_pixels(&inverse, |x, y| {
                    radial_parameter(*start, *end, *extend, x, y).and_then(|t| ramp[sample_index(t)])
                });
            }
            Geometry::Function { domain, matrix } => {
                let to_domain = matrix.invert()?;
                let shading = self.shading;
                self.fill_pixels(&inverse, |x, y| {
                    let (u, v) = to_domain.apply(x, y);
                    let inside = (domain[0]..=domain[1]).contains(&u) && (domain[2]..=domain[3]).contains(&v);
                    if inside { shading.color(&[u, v]) } else { None }
                });
            }
            Geometry::Triangles(triangles) => {
                for triangle in triangles {
                    self.fill_triangle(triangle, &to_device);
                }
            }
        }
        let mut pixmap = Pixmap::from_vec(self.rgba, IntSize::from_wh(self.width as u32, self.height as u32)?)?;
        for pixel in pixmap.pixels_mut() {
            *pixel =
                tiny_skia::ColorU8::from_rgba(pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()).premultiply();
        }
        Some(pixmap)
    }

    fn ramp(&self, (t0, t1): (f64, f64)) -> Vec<Option<Rgb>> {
        (0..RAMP_SAMPLES)
            .map(|index| self.shading.color(&[t0 + (t1 - t0) * index as f64 / (RAMP_SAMPLES - 1) as f64]))
            .collect()
    }

    fn fill_pixels(&mut self, inverse: &Matrix, color_at: impl Fn(f64, f64) -> Option<Rgb>) {
        for py in 0..self.height {
            for px in 0..self.width {
                let (x, y) = inverse.apply(px as f64 + 0.5, py as f64 + 0.5);
                if let Some(color) = color_at(x, y) {
                    self.put(px, py, color);
                }
            }
        }
    }

    fn put(&mut self, x: usize, y: usize, color: Rgb) {
        let start = (y * self.width + x) * 4;
        let byte = |value: f32| (value * 255.0).round() as u8;
        self.rgba[start..start + 4].copy_from_slice(&[byte(color.r), byte(color.g), byte(color.b), 255]);
    }

    fn fill_triangle(&mut self, triangle: &[Vertex; 3], to_device: &Matrix) {
        let corners: Vec<(f64, f64)> = triangle.iter().map(|vertex| to_device.apply(vertex.x, vertex.y)).collect();
        let area = edge(corners[0], corners[1], corners[2]);
        if area.abs() < 1e-12 {
            return;
        }
        let min_x = corners.iter().map(|c| c.0).fold(f64::INFINITY, f64::min).floor().max(0.0) as usize;
        let max_x = (corners.iter().map(|c| c.0).fold(f64::NEG_INFINITY, f64::max).ceil() as usize).min(self.width);
        let min_y = corners.iter().map(|c| c.1).fold(f64::INFINITY, f64::min).floor().max(0.0) as usize;
        let max_y = (corners.iter().map(|c| c.1).fold(f64::NEG_INFINITY, f64::max).ceil() as usize).min(self.height);
        let components = triangle[0].color.len();
        let mut blended = vec![0.0; components];
        for py in min_y..max_y {
            for px in min_x..max_x {
                let point = (px as f64 + 0.5, py as f64 + 0.5);
                let weights = [
                    edge(corners[1], corners[2], point) / area,
                    edge(corners[2], corners[0], point) / area,
                    edge(corners[0], corners[1], point) / area,
                ];
                if weights.iter().any(|&weight| weight < -1e-9) {
                    continue;
                }
                for (index, value) in blended.iter_mut().enumerate() {
                    *value = (0..3).map(|corner| weights[corner] * triangle[corner].color[index]).sum();
                }
                if let Some(color) = self.shading.color(&blended) {
                    self.put(px, py, color);
                }
            }
        }
    }
}

fn edge(a: (f64, f64), b: (f64, f64), point: (f64, f64)) -> f64 {
    (b.0 - a.0) * (point.1 - a.1) - (b.1 - a.1) * (point.0 - a.0)
}

fn extended(t: f64, (before, after): (bool, bool)) -> Option<f64> {
    match t {
        t if t < 0.0 => before.then_some(0.0),
        t if t > 1.0 => after.then_some(1.0),
        t => Some(t),
    }
}

fn sample_index(t: f64) -> usize {
    ((t * (RAMP_SAMPLES - 1) as f64).round() as usize).min(RAMP_SAMPLES - 1)
}

fn radial_parameter(start: (f64, f64, f64), end: (f64, f64, f64), extend: (bool, bool), x: f64, y: f64) -> Option<f64> {
    let (cdx, cdy, dr) = (end.0 - start.0, end.1 - start.1, end.2 - start.2);
    let (pdx, pdy) = (x - start.0, y - start.1);
    let a = cdx * cdx + cdy * cdy - dr * dr;
    let b = pdx * cdx + pdy * cdy + start.2 * dr;
    let c = pdx * pdx + pdy * pdy - start.2 * start.2;
    let candidates: Vec<f64> = if a.abs() < 1e-12 {
        if b.abs() < 1e-12 { Vec::new() } else { vec![c / (2.0 * b)] }
    } else {
        let discriminant = b * b - a * c;
        if discriminant < 0.0 {
            return None;
        }
        let root = discriminant.sqrt();
        vec![(b + root) / a, (b - root) / a]
    };
    let mut best: Option<f64> = None;
    for s in candidates {
        if start.2 + s * dr < 0.0 {
            continue;
        }
        if let Some(t) = extended(s, extend) {
            let usable = (0.0..=1.0).contains(&s) || t == 0.0 && s < 0.0 || t == 1.0 && s > 1.0;
            if usable && best.is_none_or(|current| s > current) {
                best = Some(s);
            }
        }
    }
    best.map(|s| s.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_radial_parameter_of_concentric_circles() {
        let t = radial_parameter((0.0, 0.0, 0.0), (0.0, 0.0, 10.0), (false, false), 5.0, 0.0).unwrap();
        assert!((t - 0.5).abs() < 1e-9);
        assert_eq!(radial_parameter((0.0, 0.0, 0.0), (0.0, 0.0, 10.0), (false, false), 20.0, 0.0), None);
        assert_eq!(radial_parameter((0.0, 0.0, 0.0), (0.0, 0.0, 10.0), (false, true), 20.0, 0.0), Some(1.0));
    }

    #[test]
    fn clamps_or_drops_axial_parameters_outside_the_axis() {
        assert_eq!(extended(-0.2, (false, true)), None);
        assert_eq!(extended(-0.2, (true, false)), Some(0.0));
        assert_eq!(extended(0.4, (false, false)), Some(0.4));
    }
}
