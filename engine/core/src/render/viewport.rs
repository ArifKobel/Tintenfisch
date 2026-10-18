use crate::geometry::Matrix;
use crate::pages::Page;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    device: Matrix,
    width: u32,
    height: u32,
}

impl Viewport {
    pub fn new(page: &Page, scale: f64) -> Viewport {
        let visible = page.visible_box();
        let unrotated_width = visible.width() * scale;
        let unrotated_height = visible.height() * scale;
        let upright = Matrix::new(scale, 0.0, 0.0, -scale, -visible.x0 * scale, visible.y1 * scale);
        let turn = match page.rotate {
            90 => Matrix::new(0.0, 1.0, -1.0, 0.0, unrotated_height, 0.0),
            180 => Matrix::new(-1.0, 0.0, 0.0, -1.0, unrotated_width, unrotated_height),
            270 => Matrix::new(0.0, -1.0, 1.0, 0.0, 0.0, unrotated_width),
            _ => Matrix::IDENTITY,
        };
        let (width, height) = page.display_size();
        Viewport {
            device: upright.then(&turn),
            width: (width * scale).ceil().max(1.0) as u32,
            height: (height * scale).ceil().max(1.0) as u32,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn whole(&self) -> Tile {
        Tile { x: 0, y: 0, width: self.width, height: self.height }
    }

    pub fn device_matrix(&self, tile: Tile) -> Matrix {
        self.device.then(&Matrix::translate(-(tile.x as f64), -(tile.y as f64)))
    }

    pub fn expand(&self, tile: Tile, margin: u32) -> Tile {
        let x = tile.x.saturating_sub(margin);
        let y = tile.y.saturating_sub(margin);
        let right = (tile.x + tile.width + margin).min(self.width);
        let bottom = (tile.y + tile.height + margin).min(self.height);
        Tile { x, y, width: right - x, height: bottom - y }
    }

    pub fn clip(&self, tile: Tile) -> Option<Tile> {
        let right = (tile.x + tile.width).min(self.width);
        let bottom = (tile.y + tile.height).min(self.height);
        (tile.x < right && tile.y < bottom).then(|| Tile {
            x: tile.x,
            y: tile.y,
            width: right - tile.x,
            height: bottom - tile.y,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::pages::PageId;
    use crate::syntax::{Dict, ObjId};

    fn page(rotate: u16) -> Page {
        Page {
            id: PageId(ObjId::new(1, 0)),
            parent: None,
            media_box: Rect::new(0.0, 0.0, 200.0, 100.0),
            crop_box: None,
            rotate,
            resources: Dict::new(),
            contents: Vec::new(),
        }
    }

    #[test]
    fn maps_the_corners_for_every_rotation() {
        let corner = |rotate, x, y| Viewport::new(&page(rotate), 2.0).device.apply(x, y);
        assert_eq!(corner(0, 0.0, 100.0), (0.0, 0.0));
        assert_eq!(corner(0, 200.0, 0.0), (400.0, 200.0));
        assert_eq!(corner(90, 0.0, 100.0), (200.0, 0.0));
        assert_eq!(corner(180, 0.0, 100.0), (400.0, 200.0));
        assert_eq!(corner(270, 0.0, 100.0), (0.0, 400.0));
        let turned = Viewport::new(&page(90), 2.0);
        assert_eq!((turned.width(), turned.height()), (200, 400));
    }

    #[test]
    fn clips_tiles_to_the_page() {
        let viewport = Viewport::new(&page(0), 1.0);
        assert_eq!(
            viewport.clip(Tile { x: 150, y: 50, width: 100, height: 100 }),
            Some(Tile { x: 150, y: 50, width: 50, height: 50 })
        );
        assert_eq!(viewport.clip(Tile { x: 300, y: 0, width: 10, height: 10 }), None);
    }
}
