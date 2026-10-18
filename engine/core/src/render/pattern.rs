use crate::content::{ContentParser, Operation};
use crate::error::{Error, Result};
use crate::geometry::{Matrix, Rect};
use crate::shading::Shading;
use crate::store::ObjectStore;
use crate::syntax::{Dict, ObjId, Object};

pub struct TilingPattern {
    pub id: ObjId,
    pub operations: Vec<Operation>,
    pub resources: Dict,
    pub bbox: Rect,
    pub x_step: f64,
    pub y_step: f64,
    pub matrix: Matrix,
    pub colored: bool,
}

pub enum Pattern {
    Shading { shading: Box<Shading>, matrix: Matrix },
    Tiling(TilingPattern),
}

impl Pattern {
    pub fn read(object: &Object, resources: &Dict, store: &ObjectStore) -> Result<Pattern> {
        let id = object.as_reference();
        let resolved = store.resolve(object)?;
        let dict = resolved.as_dict().ok_or_else(|| Error::new("pattern is not a dictionary"))?;
        let matrix = dict.get("Matrix").and_then(Matrix::from_object).unwrap_or(Matrix::IDENTITY);
        match dict.int("PatternType") {
            Some(2) => Ok(Pattern::Shading {
                shading: Box::new(Shading::read(dict.get("Shading").unwrap_or(&Object::Null), resources, store)?),
                matrix,
            }),
            Some(1) => {
                let stream = resolved.as_stream().ok_or_else(|| Error::new("tiling pattern is not a stream"))?;
                let data = store.decode_stream(stream)?.data;
                let bbox = dict
                    .get("BBox")
                    .and_then(Rect::from_object)
                    .ok_or_else(|| Error::new("tiling pattern without /BBox"))?;
                let step = |key: &str| dict.get(key).and_then(Object::as_number).filter(|step| *step != 0.0);
                let cell_resources = match dict.get("Resources") {
                    Some(own) => store.resolve(own)?.as_dict().cloned().unwrap_or_default(),
                    None => resources.clone(),
                };
                Ok(Pattern::Tiling(TilingPattern {
                    id: id.unwrap_or(ObjId::new(0, 0)),
                    operations: ContentParser::new(&data, 0).parse(0),
                    resources: cell_resources,
                    bbox,
                    x_step: step("XStep").unwrap_or(bbox.width()),
                    y_step: step("YStep").unwrap_or(bbox.height()),
                    matrix,
                    colored: dict.int("PaintType") != Some(2),
                }))
            }
            other => Err(Error::new(format!("unsupported pattern type {other:?}"))),
        }
    }
}
