use crate::content::ContentWriter;
use crate::geometry::{Matrix, Rect};
use crate::syntax::Object;

const TOUCH: f64 = 0.01;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub corners: [(f64, f64); 4],
}

impl Quad {
    pub fn from_rect(rect: Rect) -> Quad {
        Quad { corners: [(rect.x0, rect.y0), (rect.x1, rect.y0), (rect.x1, rect.y1), (rect.x0, rect.y1)] }
    }

    pub fn unit_square(matrix: &Matrix) -> Quad {
        Quad { corners: [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| matrix.apply(x, y)) }
    }

    pub fn mapped(&self, map: impl Fn((f64, f64)) -> (f64, f64)) -> Quad {
        Quad { corners: self.corners.map(map) }
    }

    pub fn bounds(&self) -> Rect {
        let xs = self.corners.map(|corner| corner.0);
        let ys = self.corners.map(|corner| corner.1);
        Rect::new(
            xs.iter().copied().fold(f64::INFINITY, f64::min),
            ys.iter().copied().fold(f64::INFINITY, f64::min),
            xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    }

    pub fn overlaps(&self, other: &Quad) -> bool {
        [self, other].iter().all(|quad| {
            (0..4).all(|index| {
                let (a, b) = (quad.corners[index], quad.corners[(index + 1) % 4]);
                let normal = (a.1 - b.1, b.0 - a.0);
                let length = normal.0.hypot(normal.1);
                if length < f64::EPSILON {
                    return true;
                }
                let project = |quad: &Quad| {
                    let values = quad.corners.map(|(x, y)| (x * normal.0 + y * normal.1) / length);
                    (
                        values.iter().copied().fold(f64::INFINITY, f64::min),
                        values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    )
                };
                let ((low_a, high_a), (low_b, high_b)) = (project(self), project(other));
                high_a.min(high_b) - low_a.max(low_b) > TOUCH
            })
        })
    }

    pub fn contains(&self, (x, y): (f64, f64)) -> bool {
        let sides: Vec<f64> = (0..4)
            .map(|index| {
                let (a, b) = (self.corners[index], self.corners[(index + 1) % 4]);
                (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0)
            })
            .collect();
        sides.iter().all(|side| *side >= 0.0) || sides.iter().all(|side| *side <= 0.0)
    }

    pub fn write_path(&self, writer: &mut ContentWriter) {
        let point = |(x, y): (f64, f64)| [Object::Real(x), Object::Real(y)];
        writer.operation("m", &point(self.corners[0]));
        for corner in &self.corners[1..] {
            writer.operation("l", &point(*corner));
        }
        writer.operation("h", &[]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_needs_shared_area_and_turned_quads_count() {
        let area = Quad::from_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert!(area.overlaps(&Quad::from_rect(Rect::new(9.0, 9.0, 20.0, 20.0))));
        assert!(!area.overlaps(&Quad::from_rect(Rect::new(10.0, 0.0, 20.0, 10.0))), "touching edges do not overlap");
        let diamond = Quad { corners: [(15.0, 5.0), (20.0, 10.0), (15.0, 15.0), (10.5, 10.0)] };
        assert!(!area.overlaps(&diamond));
        let reaching = Quad { corners: [(12.0, 5.0), (17.0, 10.0), (12.0, 15.0), (7.0, 10.0)] };
        assert!(reaching.overlaps(&area));
        assert!(area.contains((5.0, 5.0)) && !area.contains((11.0, 5.0)));
    }
}
