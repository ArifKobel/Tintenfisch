use crate::error::{Error, Result};
use crate::image::jbig2::bitmap::{Bitmap, Combination};
use crate::image::jbig2::text_source::TextSource;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    BottomLeft,
    TopLeft,
    BottomRight,
    TopRight,
}

impl Corner {
    pub fn from_bits(bits: u16) -> Corner {
        match bits & 3 {
            0 => Corner::BottomLeft,
            1 => Corner::TopLeft,
            2 => Corner::BottomRight,
            _ => Corner::TopRight,
        }
    }

    fn is_left(self) -> bool {
        matches!(self, Corner::BottomLeft | Corner::TopLeft)
    }

    fn is_top(self) -> bool {
        matches!(self, Corner::TopLeft | Corner::TopRight)
    }
}

pub struct TextRegion<'s> {
    pub width: usize,
    pub height: usize,
    pub instances: usize,
    pub strips: i64,
    pub corner: Corner,
    pub transposed: bool,
    pub combination: Combination,
    pub default_black: bool,
    pub s_offset: i64,
    pub refine: bool,
    pub refinement_template: u8,
    pub refinement_adaptive: [(i8, i8); 2],
    pub symbols: &'s [Bitmap],
}

impl TextRegion<'_> {
    pub fn decode(&self, source: &mut dyn TextSource) -> Result<Bitmap> {
        let mut region = Bitmap::new(self.width, self.height, self.default_black);
        let mut strip_t = -source.strip_delta()? * self.strips;
        let mut first_s = 0;
        let mut placed = 0;
        while placed < self.instances {
            strip_t += source.strip_delta()? * self.strips;
            let mut current_s = None;
            loop {
                let s = match current_s {
                    None => {
                        first_s += source.first_s()?;
                        first_s
                    }
                    Some(s) => match source.s_delta()? {
                        Some(delta) => s + delta + self.s_offset,
                        None => break,
                    },
                };
                let t = strip_t + source.instance_t(self.strips)?;
                let id = source.symbol_id()?;
                let symbol = self
                    .symbols
                    .get(id)
                    .ok_or_else(|| Error::new("a JBIG2 text region names a symbol it does not have"))?;
                let refined = match self.refine {
                    true => source.refinement(symbol, self.refinement_template, self.refinement_adaptive)?,
                    false => None,
                };
                let instance = refined.as_ref().unwrap_or(symbol);
                let (width, height) = (instance.width as i64, instance.height as i64);
                let mut s = s;
                if !self.transposed && !self.corner.is_left() {
                    s += width - 1;
                }
                if self.transposed && !self.corner.is_top() {
                    s += height - 1;
                }
                let (x, y) = if self.transposed { (t, s) } else { (s, t) };
                let left = if self.transposed {
                    if self.corner.is_left() { x } else { x - width + 1 }
                } else if self.corner.is_left() {
                    x
                } else {
                    x - width + 1
                };
                let top = if self.transposed {
                    if self.corner.is_top() { y } else { y - height + 1 }
                } else if self.corner.is_top() {
                    y
                } else {
                    y - height + 1
                };
                region.compose(instance, left, top, self.combination);
                if !self.transposed && self.corner.is_left() {
                    s += width - 1;
                }
                if self.transposed && self.corner.is_top() {
                    s += height - 1;
                }
                current_s = Some(s);
                placed += 1;
                if placed > self.instances {
                    return Err(Error::new("a JBIG2 text region places more symbols than it announces"));
                }
            }
        }
        Ok(region)
    }
}
