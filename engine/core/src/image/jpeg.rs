use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::colorspace::ColorSpace as JpegColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

use crate::error::{Error, Result};

pub struct JpegSamples {
    pub width: usize,
    pub height: usize,
    pub components: usize,
    pub data: Vec<u8>,
}

pub fn decode(data: &[u8], components: usize) -> Result<JpegSamples> {
    let output = match components {
        1 => JpegColorSpace::Luma,
        4 => JpegColorSpace::CMYK,
        _ => JpegColorSpace::RGB,
    };
    let options = DecoderOptions::default().jpeg_set_out_colorspace(output);
    let mut decoder = JpegDecoder::new_with_options(data, options);
    let pixels = decoder.decode().map_err(|error| Error::new(format!("DCTDecode: {error:?}")))?;
    let info = decoder.info().ok_or_else(|| Error::new("DCTDecode: no image header"))?;
    let components = decoder.get_output_colorspace().map_or(components, |space| space.num_components());
    Ok(JpegSamples { width: info.width as usize, height: info.height as usize, components, data: pixels })
}
