use crate::syntax::Object;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self { x0: x0.min(x1), y0: y0.min(y1), x1: x0.max(x1), y1: y0.max(y1) }
    }

    pub fn from_object(object: &Object) -> Option<Rect> {
        let numbers: Vec<f64> = object.as_array()?.iter().filter_map(Object::as_number).collect();
        match numbers[..] {
            [x0, y0, x1, y1] => Some(Rect::new(x0, y0, x1, y1)),
            _ => None,
        }
    }

    pub fn to_object(&self) -> Object {
        Object::Array([self.x0, self.y0, self.x1, self.y1].map(Object::Real).to_vec())
    }

    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }

    pub fn is_empty(&self) -> bool {
        self.width() <= 0.0 || self.height() <= 0.0
    }

    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let clipped = Rect {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        };
        (!clipped.is_empty()).then_some(clipped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_corners_and_intersects() {
        let a = Rect::new(100.0, 100.0, 0.0, 0.0);
        assert_eq!(a, Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(a.intersect(&Rect::new(50.0, -10.0, 150.0, 40.0)), Some(Rect::new(50.0, 0.0, 100.0, 40.0)));
        assert_eq!(a.intersect(&Rect::new(200.0, 200.0, 300.0, 300.0)), None);
    }

    #[test]
    fn reads_four_numbers_from_an_array() {
        let object = Object::Array(vec![Object::Int(0), Object::Real(0.5), Object::Int(612), Object::Int(792)]);
        assert_eq!(Rect::from_object(&object), Some(Rect::new(0.0, 0.5, 612.0, 792.0)));
        assert_eq!(Rect::from_object(&Object::Array(vec![Object::Int(1)])), None);
    }
}
