use crate::color::{ColorSpace, Rgb};
use crate::error::{Error, Result};
use crate::function::PdfFunction;
use crate::geometry::{Matrix, Rect};
use crate::shading::mesh_reader::MeshReader;
use crate::store::ObjectStore;
use crate::syntax::{Dict, Object};

#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    pub x: f64,
    pub y: f64,
    pub color: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Function { domain: [f64; 4], matrix: Matrix },
    Axial { start: (f64, f64), end: (f64, f64), domain: (f64, f64), extend: (bool, bool) },
    Radial { start: (f64, f64, f64), end: (f64, f64, f64), domain: (f64, f64), extend: (bool, bool) },
    Triangles(Vec<[Vertex; 3]>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shading {
    pub geometry: Geometry,
    pub space: ColorSpace,
    pub functions: Vec<PdfFunction>,
    pub bbox: Option<Rect>,
    pub background: Option<Vec<f64>>,
}

impl Shading {
    pub fn read(object: &Object, resources: &Dict, store: &ObjectStore) -> Result<Shading> {
        let object = store.resolve(object)?;
        let dict = object.as_dict().ok_or_else(|| Error::new("shading is not a dictionary"))?;
        let space = ColorSpace::read(dict.get("ColorSpace").unwrap_or(&Object::Null), resources, store)?;
        let functions = match dict.get("Function").map(|function| store.resolve(function)).transpose()? {
            Some(Object::Array(parts)) => {
                parts.iter().map(|part| PdfFunction::read(part, store)).collect::<Result<Vec<_>>>()?
            }
            Some(function) => vec![PdfFunction::read(&function, store)?],
            None => Vec::new(),
        };
        let numbers = |key: &str| -> Vec<f64> {
            dict.get(key).and_then(Object::as_array).unwrap_or_default().iter().filter_map(Object::as_number).collect()
        };
        let domain = match numbers("Domain")[..] {
            [t0, t1] => (t0, t1),
            _ => (0.0, 1.0),
        };
        let extend = match dict.get("Extend").and_then(Object::as_array) {
            Some([start, end]) => (start.as_bool().unwrap_or(false), end.as_bool().unwrap_or(false)),
            _ => (false, false),
        };
        let geometry = match dict.int("ShadingType") {
            Some(1) => Geometry::Function {
                domain: match numbers("Domain")[..] {
                    [x0, x1, y0, y1] => [x0, x1, y0, y1],
                    _ => [0.0, 1.0, 0.0, 1.0],
                },
                matrix: dict.get("Matrix").and_then(Matrix::from_object).unwrap_or(Matrix::IDENTITY),
            },
            Some(2) => match numbers("Coords")[..] {
                [x0, y0, x1, y1] => Geometry::Axial { start: (x0, y0), end: (x1, y1), domain, extend },
                _ => return Err(Error::new("axial shading without four /Coords")),
            },
            Some(3) => match numbers("Coords")[..] {
                [x0, y0, r0, x1, y1, r1] => Geometry::Radial { start: (x0, y0, r0), end: (x1, y1, r1), domain, extend },
                _ => return Err(Error::new("radial shading without six /Coords")),
            },
            Some(kind @ 4..=7) => {
                let stream = object.as_stream().ok_or_else(|| Error::new("mesh shading is not a stream"))?;
                let data = store.decode_stream(stream)?.data;
                let components = if functions.is_empty() { space.components() } else { 1 };
                Geometry::Triangles(MeshReader::new(dict, &data, components)?.triangles(kind)?)
            }
            other => return Err(Error::new(format!("unsupported shading type {other:?}"))),
        };
        Ok(Shading {
            geometry,
            space,
            functions,
            bbox: dict.get("BBox").and_then(Rect::from_object),
            background: dict
                .get("Background")
                .and_then(Object::as_array)
                .map(|values| values.iter().filter_map(Object::as_number).collect()),
        })
    }

    pub fn color(&self, inputs: &[f64]) -> Option<Rgb> {
        match self.functions.as_slice() {
            [] => self.space.to_rgb(inputs),
            [single] => self.space.to_rgb(&single.evaluate(inputs)),
            several => {
                let components: Vec<f64> = several.iter().flat_map(|function| function.evaluate(inputs)).collect();
                self.space.to_rgb(&components)
            }
        }
    }
}
