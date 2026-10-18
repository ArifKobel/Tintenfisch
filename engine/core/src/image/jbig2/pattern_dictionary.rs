use crate::error::Result;
use crate::image::ccitt::{FaxDecoder, FaxParams};
use crate::image::jbig2::bitmap::Bitmap;
use crate::image::jbig2::generic_region::GenericRegion;
use crate::image::jbig2::jbig2_decoder::{byte, number};
use crate::image::mq_decoder::MqDecoder;

pub fn pattern_dictionary(data: &[u8]) -> Result<Vec<Bitmap>> {
    let flags = byte(data, 0)?;
    let (width, height) = (byte(data, 1)? as usize, byte(data, 2)? as usize);
    let count = number(data, 3)? as usize + 1;
    let total_width = width * count;
    let collective = if flags & 1 != 0 {
        let params = FaxParams {
            k: -1,
            columns: total_width,
            rows: Some(height),
            byte_aligned: false,
            end_of_line: false,
            black_is_1: true,
        };
        let (pixels, _) = FaxDecoder::new(params).decode(&data[7..]);
        Bitmap::from_packed(total_width, height, pixels)
    } else {
        let template = (flags >> 1) & 3;
        let region = GenericRegion {
            width: total_width,
            height,
            template,
            typical_prediction: false,
            adaptive: vec![(-(width as i64), 0), (-3, -1), (2, -2), (-2, -2)],
            skip: None,
        };
        let mut contexts = GenericRegion::contexts(template);
        region.decode(&mut MqDecoder::new(&data[7..]), &mut contexts)
    };
    Ok((0..count)
        .map(|index| {
            let mut pattern = Bitmap::new(width, height, false);
            pattern.compose_from(&collective, (index * width) as i64, 0);
            pattern
        })
        .collect())
}
