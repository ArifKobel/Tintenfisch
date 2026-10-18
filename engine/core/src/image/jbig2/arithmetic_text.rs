use crate::error::{Error, Result};
use crate::image::jbig2::bitmap::Bitmap;
use crate::image::jbig2::coding_state::CodingState;
use crate::image::jbig2::refinement_region::RefinementRegion;
use crate::image::jbig2::text_source::{TextSource, refinement_offset};
use crate::image::mq_decoder::MqDecoder;

pub struct ArithmeticText<'s, 'd> {
    pub decoder: &'s mut MqDecoder<'d>,
    pub state: &'s mut CodingState,
}

fn broken() -> Error {
    Error::new("a JBIG2 text region ends early")
}

impl TextSource for ArithmeticText<'_, '_> {
    fn strip_delta(&mut self) -> Result<i64> {
        self.state.strip_delta.decode(self.decoder).ok_or_else(broken)
    }

    fn first_s(&mut self) -> Result<i64> {
        self.state.first_s.decode(self.decoder).ok_or_else(broken)
    }

    fn s_delta(&mut self) -> Result<Option<i64>> {
        Ok(self.state.s_delta.decode(self.decoder))
    }

    fn instance_t(&mut self, strips: i64) -> Result<i64> {
        if strips == 1 { Ok(0) } else { self.state.instance_t.decode(self.decoder).ok_or_else(broken) }
    }

    fn symbol_id(&mut self) -> Result<usize> {
        Ok(self.state.symbol_id.decode(self.decoder))
    }

    fn refinement(&mut self, symbol: &Bitmap, template: u8, adaptive: [(i8, i8); 2]) -> Result<Option<Bitmap>> {
        if self.state.refined.decode(self.decoder).ok_or_else(broken)? == 0 {
            return Ok(None);
        }
        let width_delta = self.state.refinement_width.decode(self.decoder).ok_or_else(broken)?;
        let height_delta = self.state.refinement_height.decode(self.decoder).ok_or_else(broken)?;
        let x = self.state.refinement_x.decode(self.decoder).ok_or_else(broken)?;
        let y = self.state.refinement_y.decode(self.decoder).ok_or_else(broken)?;
        let region = RefinementRegion {
            width: (symbol.width as i64 + width_delta).max(0) as usize,
            height: (symbol.height as i64 + height_delta).max(0) as usize,
            template,
            reference: symbol,
            offset: refinement_offset(width_delta, height_delta, x, y),
            typical_prediction: false,
            adaptive,
        };
        Ok(Some(region.decode(self.decoder, &mut self.state.refinement)))
    }
}
