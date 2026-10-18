use crate::content::ContentWriter;
use crate::error::{Error, Result};
use crate::geometry::{Matrix, Rect};
use crate::image::DecodedImage;
use crate::syntax::{Name, Object};

#[derive(Clone, Debug, PartialEq)]
pub enum SignatureMark {
    Ink { strokes: Vec<Vec<(f64, f64)>>, width: f64, color: [f64; 3] },
    Image(DecodedImage),
}

impl SignatureMark {
    pub fn extent(&self) -> Result<Rect> {
        let extent = match self {
            SignatureMark::Ink { strokes, width, .. } => {
                let reach = width / 2.0;
                let (xs, ys): (Vec<f64>, Vec<f64>) = strokes.iter().flatten().copied().unzip();
                Rect::new(
                    xs.iter().copied().fold(f64::INFINITY, f64::min) - reach,
                    ys.iter().copied().fold(f64::INFINITY, f64::min) - reach,
                    xs.iter().copied().fold(f64::NEG_INFINITY, f64::max) + reach,
                    ys.iter().copied().fold(f64::NEG_INFINITY, f64::max) + reach,
                )
            }
            SignatureMark::Image(image) => {
                if image.width == 0 || image.height == 0 || image.rgba.len() != image.width * image.height * 4 {
                    return Err(Error::unsupported("the signature picture has no pixels"));
                }
                Rect::new(0.0, 0.0, image.width as f64, image.height as f64)
            }
        };
        if extent.width() <= 0.0 || extent.height() <= 0.0 {
            return Err(Error::unsupported("the signature has no area"));
        }
        Ok(extent)
    }

    pub fn content(&self, frame: &Matrix, image: &Name) -> Vec<u8> {
        let mut content = ContentWriter::new();
        content.operation("q", &[]);
        content.operation("cm", &[frame.a, frame.b, frame.c, frame.d, frame.e, frame.f].map(Object::Real));
        match self {
            SignatureMark::Ink { strokes, width, color } => {
                content
                    .operation("RG", &color.map(Object::Real))
                    .operation("w", &[Object::Real(*width)])
                    .operation("J", &[Object::Int(1)])
                    .operation("j", &[Object::Int(1)]);
                for stroke in strokes.iter().filter(|stroke| !stroke.is_empty()) {
                    content.operation("m", &[Object::Real(stroke[0].0), Object::Real(stroke[0].1)]);
                    let rest = if stroke.len() == 1 { &stroke[..] } else { &stroke[1..] };
                    for (x, y) in rest {
                        content.operation("l", &[Object::Real(*x), Object::Real(*y)]);
                    }
                    content.operation("S", &[]);
                }
            }
            SignatureMark::Image(picture) => {
                let (width, height) = (picture.width as f64, picture.height as f64);
                content
                    .operation("cm", &[width, 0.0, 0.0, -height, 0.0, height].map(Object::Real))
                    .operation("Do", &[Object::Name(image.clone())]);
            }
        }
        content.operation("Q", &[]);
        content.finish()
    }
}
