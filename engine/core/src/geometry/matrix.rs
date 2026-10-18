use std::ops::Mul;

use crate::syntax::Object;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Matrix {
    pub const IDENTITY: Matrix = Matrix { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    pub const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    pub fn translate(x: f64, y: f64) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, x, y)
    }

    pub fn scale(x: f64, y: f64) -> Self {
        Self::new(x, 0.0, 0.0, y, 0.0, 0.0)
    }

    pub fn from_numbers(numbers: &[f64]) -> Option<Self> {
        match *numbers {
            [a, b, c, d, e, f] => Some(Self::new(a, b, c, d, e, f)),
            _ => None,
        }
    }

    pub fn from_object(object: &Object) -> Option<Self> {
        let numbers: Vec<f64> = object.as_array()?.iter().filter_map(Object::as_number).collect();
        Self::from_numbers(&numbers)
    }

    pub fn then(&self, outer: &Matrix) -> Matrix {
        *self * *outer
    }

    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }

    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    pub fn invert(&self) -> Option<Matrix> {
        let determinant = self.determinant();
        if determinant.abs() < f64::EPSILON {
            return None;
        }
        Some(Matrix::new(
            self.d / determinant,
            -self.b / determinant,
            -self.c / determinant,
            self.a / determinant,
            (self.c * self.f - self.d * self.e) / determinant,
            (self.b * self.e - self.a * self.f) / determinant,
        ))
    }

    pub fn expansion(&self) -> f64 {
        self.determinant().abs().sqrt()
    }

    pub fn to_skia(&self) -> tiny_skia::Transform {
        tiny_skia::Transform::from_row(
            self.a as f32,
            self.b as f32,
            self.c as f32,
            self.d as f32,
            self.e as f32,
            self.f as f32,
        )
    }
}

impl Mul for Matrix {
    type Output = Matrix;

    fn mul(self, outer: Matrix) -> Matrix {
        Matrix::new(
            self.a * outer.a + self.b * outer.c,
            self.a * outer.b + self.b * outer.d,
            self.c * outer.a + self.d * outer.c,
            self.c * outer.b + self.d * outer.d,
            self.e * outer.a + self.f * outer.c + outer.e,
            self.e * outer.b + self.f * outer.d + outer.f,
        )
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_inner_transform_first() {
        let scaled_then_moved = Matrix::scale(2.0, 3.0).then(&Matrix::translate(10.0, 20.0));
        assert_eq!(scaled_then_moved.apply(1.0, 1.0), (12.0, 23.0));
        let moved_then_scaled = Matrix::translate(10.0, 20.0).then(&Matrix::scale(2.0, 3.0));
        assert_eq!(moved_then_scaled.apply(1.0, 1.0), (22.0, 63.0));
    }

    #[test]
    fn inverts_back_to_identity() {
        let matrix = Matrix::new(2.0, 1.0, -1.0, 3.0, 5.0, 7.0);
        let round_trip = matrix.then(&matrix.invert().unwrap());
        let (x, y) = round_trip.apply(4.0, -9.0);
        assert!((x - 4.0).abs() < 1e-9 && (y + 9.0).abs() < 1e-9);
        assert_eq!(Matrix::scale(0.0, 1.0).invert(), None);
    }
}
