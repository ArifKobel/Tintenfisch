use crate::error::Result;
use crate::geometry::{Matrix, Rect};
use crate::pages::Page;
use crate::store::ObjectStore;
use crate::syntax::{Dict, Name, ObjId, Object};

const HIDDEN: u32 = 1 << 1;
const NO_VIEW: u32 = 1 << 5;

#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    pub id: Option<ObjId>,
    pub subtype: Name,
    pub rect: Rect,
    pub flags: u32,
    pub dict: Dict,
}

impl Annotation {
    pub fn on_page(store: &ObjectStore, page: &Page) -> Result<Vec<Annotation>> {
        let page_object = store.object(page.id.0)?;
        let Some(annots) = page_object.as_dict().and_then(|dict| dict.get("Annots")) else {
            return Ok(Vec::new());
        };
        let annots = store.resolve(annots)?;
        let mut annotations = Vec::new();
        for entry in annots.as_array().unwrap_or_default() {
            let resolved = store.resolve(entry)?;
            let Some(dict) = resolved.as_dict() else { continue };
            let Some(rect) =
                dict.get("Rect").map(|rect| store.resolve(rect)).transpose()?.and_then(|rect| Rect::from_object(&rect))
            else {
                continue;
            };
            annotations.push(Annotation {
                id: entry.as_reference(),
                subtype: dict.name("Subtype").cloned().unwrap_or_else(|| Name::from("Unknown")),
                rect,
                flags: dict.int("F").unwrap_or(0) as u32,
                dict: dict.clone(),
            });
        }
        Ok(annotations)
    }

    pub fn is_shown(&self) -> bool {
        self.flags & (HIDDEN | NO_VIEW) == 0 && !self.subtype.is("Popup")
    }

    pub fn normal_appearance(&self, store: &ObjectStore) -> Result<Option<ObjId>> {
        let Some(appearances) = self.dict.get("AP") else { return Ok(None) };
        let appearances = store.resolve(appearances)?;
        let Some(normal) = appearances.as_dict().and_then(|dict| dict.get("N")) else { return Ok(None) };
        if let Some(id) = normal.as_reference()
            && store.object(id)?.as_stream().is_some()
        {
            return Ok(Some(id));
        }
        let states = store.resolve(normal)?;
        let state = self.dict.name("AS");
        Ok(state.and_then(|state| states.as_dict()?.get_name(state)).and_then(Object::as_reference))
    }
}

pub fn appearance_placement(bbox: Rect, matrix: Matrix, rect: Rect) -> Matrix {
    let corners = [(bbox.x0, bbox.y0), (bbox.x1, bbox.y0), (bbox.x0, bbox.y1), (bbox.x1, bbox.y1)]
        .map(|(x, y)| matrix.apply(x, y));
    let x0 = corners.iter().map(|c| c.0).fold(f64::INFINITY, f64::min);
    let x1 = corners.iter().map(|c| c.0).fold(f64::NEG_INFINITY, f64::max);
    let y0 = corners.iter().map(|c| c.1).fold(f64::INFINITY, f64::min);
    let y1 = corners.iter().map(|c| c.1).fold(f64::NEG_INFINITY, f64::max);
    let scale_x = if x1 > x0 { rect.width() / (x1 - x0) } else { 1.0 };
    let scale_y = if y1 > y0 { rect.height() / (y1 - y0) } else { 1.0 };
    Matrix::translate(-x0, -y0).then(&Matrix::scale(scale_x, scale_y)).then(&Matrix::translate(rect.x0, rect.y0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_a_scaled_and_rotated_appearance_onto_its_rectangle() {
        let rect = Rect::new(100.0, 200.0, 140.0, 220.0);
        let plain = appearance_placement(Rect::new(0.0, 0.0, 20.0, 10.0), Matrix::IDENTITY, rect);
        assert_eq!(plain.apply(0.0, 0.0), (100.0, 200.0));
        assert_eq!(plain.apply(20.0, 10.0), (140.0, 220.0));
        let turned = Matrix::new(0.0, 1.0, -1.0, 0.0, 0.0, 0.0);
        let placed = turned.then(&appearance_placement(Rect::new(0.0, 0.0, 10.0, 40.0), turned, rect));
        let (x, y) = placed.apply(0.0, 0.0);
        assert!((x - 140.0).abs() < 1e-9 && (y - 200.0).abs() < 1e-9);
    }
}
