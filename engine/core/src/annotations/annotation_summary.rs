use crate::annotations::Annotation;
use crate::error::Result;
use crate::geometry::Rect;
use crate::pages::Page;
use crate::render::DisplaySpace;
use crate::syntax::{ObjId, Object, decode_text};

const NOT_COMMENTS: [&str; 3] = ["Popup", "Link", "Widget"];

#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationSummary {
    pub id: ObjId,
    pub subtype: String,
    pub contents: String,
    pub author: String,
    pub date: String,
    pub rect: Rect,
    pub color: Option<[f64; 3]>,
}

impl AnnotationSummary {
    pub fn of(page: &Page, annotations: &[Annotation]) -> Result<Vec<AnnotationSummary>> {
        let space = DisplaySpace::of(page)?;
        Ok(annotations
            .iter()
            .filter(|annotation| !NOT_COMMENTS.iter().any(|kind| annotation.subtype.is(kind)))
            .filter_map(|annotation| {
                let text = |key: &str| {
                    annotation.dict.get(key).and_then(Object::as_string).map(decode_text).unwrap_or_default()
                };
                let color = annotation.dict.get("C").and_then(Object::as_array).and_then(|components| {
                    let values: Vec<f64> = components.iter().filter_map(Object::as_number).collect();
                    <[f64; 3]>::try_from(values).ok()
                });
                Some(AnnotationSummary {
                    id: annotation.id?,
                    subtype: String::from_utf8_lossy(annotation.subtype.as_bytes()).into_owned(),
                    contents: text("Contents"),
                    author: text("T"),
                    date: text("M"),
                    rect: space.rect_to_display(annotation.rect),
                    color,
                })
            })
            .collect())
    }
}
