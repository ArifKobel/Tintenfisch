use crate::annotations::Annotation;
use crate::error::Result;
use crate::geometry::Quad;
use crate::pages::Page;
use crate::store::ObjectStore;
use crate::syntax::{ObjId, Object};

const BLACK: [f64; 3] = [0.0, 0.0, 0.0];

#[derive(Clone, Debug, PartialEq)]
pub struct RedactionMark {
    pub annotation: ObjId,
    pub areas: Vec<Quad>,
    pub fill: [f64; 3],
}

impl RedactionMark {
    pub fn on_page(store: &ObjectStore, page: &Page) -> Result<Vec<RedactionMark>> {
        Ok(Annotation::on_page(store, page)?
            .into_iter()
            .filter(|annotation| annotation.subtype.is("Redact"))
            .filter_map(|annotation| {
                let quads: Vec<f64> = annotation
                    .dict
                    .get("QuadPoints")
                    .and_then(Object::as_array)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(Object::as_number)
                    .collect();
                let areas = if quads.len() >= 8 {
                    quads
                        .as_chunks::<8>()
                        .0
                        .iter()
                        .map(|[x0, y0, x1, y1, x2, y2, x3, y3]| Quad {
                            corners: [(*x0, *y0), (*x1, *y1), (*x3, *y3), (*x2, *y2)],
                        })
                        .collect()
                } else {
                    vec![Quad::from_rect(annotation.rect)]
                };
                let fill = annotation.dict.get("IC").and_then(Object::as_array).and_then(|color| {
                    <[f64; 3]>::try_from(color.iter().filter_map(Object::as_number).collect::<Vec<_>>()).ok()
                });
                Some(RedactionMark { annotation: annotation.id?, areas, fill: fill.unwrap_or(BLACK) })
            })
            .collect())
    }
}
