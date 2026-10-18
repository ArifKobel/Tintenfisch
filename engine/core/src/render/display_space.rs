use crate::error::{Error, Result};
use crate::geometry::{Matrix, Rect};
use crate::pages::Page;
use crate::render::Viewport;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplaySpace {
    to_display: Matrix,
    to_user: Matrix,
}

impl DisplaySpace {
    pub fn of(page: &Page) -> Result<DisplaySpace> {
        let viewport = Viewport::new(page, 1.0);
        let to_display = viewport.device_matrix(viewport.whole());
        let to_user = to_display.invert().ok_or_else(|| Error::new("the page has no area"))?;
        Ok(DisplaySpace { to_display, to_user })
    }

    pub fn user_to_display(&self) -> Matrix {
        self.to_display
    }

    pub fn display_to_user(&self) -> Matrix {
        self.to_user
    }

    pub fn point_to_user(&self, (x, y): (f64, f64)) -> (f64, f64) {
        self.to_user.apply(x, y)
    }

    pub fn rect_to_display(&self, rect: Rect) -> Rect {
        corners_through(&self.to_display, rect)
    }

    pub fn rect_to_user(&self, rect: Rect) -> Rect {
        corners_through(&self.to_user, rect)
    }
}

fn corners_through(matrix: &Matrix, rect: Rect) -> Rect {
    let (x0, y0) = matrix.apply(rect.x0, rect.y0);
    let (x1, y1) = matrix.apply(rect.x1, rect.y1);
    Rect::new(x0, y0, x1, y1)
}
