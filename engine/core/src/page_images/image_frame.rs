use crate::geometry::Matrix;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub angle: f64,
}

impl ImageFrame {
    pub fn of(unit_to_display: &Matrix) -> ImageFrame {
        let mut corners = [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)].map(|(u, v)| unit_to_display.apply(u, v));
        let turning: f64 = (0..4)
            .map(|index| {
                let ((x0, y0), (x1, y1)) = (corners[index], corners[(index + 1) % 4]);
                x0 * y1 - x1 * y0
            })
            .sum();
        if turning < 0.0 {
            corners.reverse();
        }
        (0..4)
            .map(|start| {
                let (left, top) = corners[start];
                let (right, top_right) = corners[(start + 1) % 4];
                let (bottom_left, bottom) = corners[(start + 3) % 4];
                ImageFrame {
                    x: left,
                    y: top,
                    width: (right - left).hypot(top_right - top),
                    height: (bottom_left - left).hypot(bottom - top),
                    angle: (top_right - top).atan2(right - left),
                }
            })
            .min_by(|a, b| a.angle.abs().total_cmp(&b.angle.abs()))
            .unwrap_or(ImageFrame { x: 0.0, y: 0.0, width: 0.0, height: 0.0, angle: 0.0 })
    }

    pub fn mapping(&self) -> Matrix {
        let (sin, cos) = self.angle.sin_cos();
        let (across, down) = ((self.width * cos, self.width * sin), (-self.height * sin, self.height * cos));
        Matrix::new(across.0, across.1, -down.0, -down.1, self.x + down.0, self.y + down.1)
    }

    pub fn moved(&self, unit_to_display: &Matrix) -> Option<Matrix> {
        let old = ImageFrame::of(unit_to_display).mapping().invert()?;
        Some(unit_to_display.then(&old).then(&self.mapping()))
    }
}
