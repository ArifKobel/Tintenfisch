use crate::error::Result;
use crate::image::jbig2::bitmap::Bitmap;

pub trait TextSource {
    fn strip_delta(&mut self) -> Result<i64>;
    fn first_s(&mut self) -> Result<i64>;
    fn s_delta(&mut self) -> Result<Option<i64>>;
    fn instance_t(&mut self, strips: i64) -> Result<i64>;
    fn symbol_id(&mut self) -> Result<usize>;
    fn refinement(&mut self, symbol: &Bitmap, template: u8, adaptive: [(i8, i8); 2]) -> Result<Option<Bitmap>>;
}

pub fn refinement_offset(width_delta: i64, height_delta: i64, x: i64, y: i64) -> (i64, i64) {
    ((width_delta >> 1) + x, (height_delta >> 1) + y)
}
