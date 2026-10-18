use crate::geometry::Rect;
use crate::pages::PageId;
use crate::syntax::{Dict, ObjId};

const US_LETTER: Rect = Rect { x0: 0.0, y0: 0.0, x1: 612.0, y1: 792.0 };

#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub id: PageId,
    pub parent: Option<ObjId>,
    pub media_box: Rect,
    pub crop_box: Option<Rect>,
    pub rotate: u16,
    pub resources: Dict,
    pub contents: Vec<ObjId>,
}

impl Page {
    pub fn default_media_box() -> Rect {
        US_LETTER
    }

    pub fn visible_box(&self) -> Rect {
        self.crop_box.and_then(|crop| crop.intersect(&self.media_box)).unwrap_or(self.media_box)
    }

    pub fn display_size(&self) -> (f64, f64) {
        let visible = self.visible_box();
        if self.rotate % 180 == 90 { (visible.height(), visible.width()) } else { (visible.width(), visible.height()) }
    }
}

pub fn normalized_rotation(value: i64) -> u16 {
    if value % 90 != 0 {
        return 0;
    }
    value.rem_euclid(360) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_rotation() {
        assert_eq!(normalized_rotation(-90), 270);
        assert_eq!(normalized_rotation(450), 90);
        assert_eq!(normalized_rotation(45), 0);
    }
}
