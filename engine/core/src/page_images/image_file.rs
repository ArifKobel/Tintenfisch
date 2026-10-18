use crate::error::{Error, Result};
use crate::geometry::Matrix;
use crate::image::{ImageStreams, JpegHeader, decode_png};

const POINTS_PER_PIXEL: f64 = 0.75;

pub struct ImageFile {
    pub streams: ImageStreams,
    pub orientation: Matrix,
    pub natural_size: (f64, f64),
}

impl ImageFile {
    pub fn read(data: &[u8]) -> Result<ImageFile> {
        let (streams, (width, height), orientation) = if data.starts_with(b"\x89PNG") {
            let decoded = decode_png(data)?;
            (ImageStreams::rgb(&decoded), (decoded.width, decoded.height), 1)
        } else if data.starts_with(&[0xFF, 0xD8]) {
            let header = JpegHeader::read(data)?;
            (ImageStreams::jpeg(&header, data), (header.width, header.height), header.orientation)
        } else {
            return Err(Error::unsupported("only PNG and JPEG images can be placed"));
        };
        let turned = orientation >= 5;
        let (shown_width, shown_height) = if turned { (height, width) } else { (width, height) };
        Ok(ImageFile {
            streams,
            orientation: oriented(orientation),
            natural_size: (shown_width as f64 * POINTS_PER_PIXEL, shown_height as f64 * POINTS_PER_PIXEL),
        })
    }

    pub fn aspect(&self) -> f64 {
        self.natural_size.0 / self.natural_size.1
    }
}

fn oriented(orientation: u16) -> Matrix {
    match orientation {
        2 => Matrix::new(-1.0, 0.0, 0.0, 1.0, 1.0, 0.0),
        3 => Matrix::new(-1.0, 0.0, 0.0, -1.0, 1.0, 1.0),
        4 => Matrix::new(1.0, 0.0, 0.0, -1.0, 0.0, 1.0),
        5 => Matrix::new(0.0, -1.0, -1.0, 0.0, 1.0, 1.0),
        6 => Matrix::new(0.0, -1.0, 1.0, 0.0, 0.0, 1.0),
        7 => Matrix::new(0.0, 1.0, 1.0, 0.0, 0.0, 0.0),
        8 => Matrix::new(0.0, 1.0, -1.0, 0.0, 1.0, 0.0),
        _ => Matrix::IDENTITY,
    }
}
