use crate::image::jbig2::bitmap::Bitmap;
use crate::image::mq_decoder::{MqContext, MqDecoder};

type Offsets = Vec<(i64, i64)>;

const TYPICAL_CONTEXTS: [usize; 2] = [0x0100, 0x0040];
const CONTEXT_BITS: [u32; 2] = [13, 10];

pub struct RefinementRegion<'r> {
    pub width: usize,
    pub height: usize,
    pub template: u8,
    pub reference: &'r Bitmap,
    pub offset: (i64, i64),
    pub typical_prediction: bool,
    pub adaptive: [(i8, i8); 2],
}

impl RefinementRegion<'_> {
    pub fn contexts(template: u8) -> Vec<MqContext> {
        vec![MqContext::default(); 1 << CONTEXT_BITS[template as usize & 1]]
    }

    pub fn decode(&self, decoder: &mut MqDecoder, contexts: &mut [MqContext]) -> Bitmap {
        let mut bitmap = Bitmap::new(self.width, self.height, false);
        let (own, referenced) = self.neighbours();
        let (dx, dy) = self.offset;
        let mut typical = false;
        for y in 0..self.height as i64 {
            if self.typical_prediction {
                typical ^= decoder.decode(&mut contexts[TYPICAL_CONTEXTS[self.template as usize & 1]]) == 1;
            }
            for x in 0..self.width as i64 {
                let (rx, ry) = (x - dx, y - dy);
                if typical {
                    let value = self.reference.get(rx, ry);
                    let uniform = (-1..=1).all(|oy| (-1..=1).all(|ox| self.reference.get(rx + ox, ry + oy) == value));
                    if uniform {
                        bitmap.set(x as usize, y as usize, value);
                        continue;
                    }
                }
                let context = own
                    .iter()
                    .map(|(ox, oy)| bitmap.get(x + ox, y + oy))
                    .chain(referenced.iter().map(|(ox, oy)| self.reference.get(rx + ox, ry + oy)));
                let context = context.enumerate().fold(0, |context, (bit, value)| context | (value as usize) << bit);
                if decoder.decode(&mut contexts[context]) == 1 {
                    bitmap.set(x as usize, y as usize, 1);
                }
            }
        }
        bitmap
    }

    fn neighbours(&self) -> (Offsets, Offsets) {
        let at = |index: usize| (self.adaptive[index].0 as i64, self.adaptive[index].1 as i64);
        if self.template == 0 {
            (
                vec![(-1, 0), (1, -1), (0, -1), at(0)],
                vec![(1, 1), (0, 1), (-1, 1), (1, 0), (0, 0), (-1, 0), (1, -1), (0, -1), at(1)],
            )
        } else {
            (vec![(-1, 0), (1, -1), (0, -1), (-1, -1)], vec![(1, 1), (0, 1), (1, 0), (0, 0), (-1, 0), (0, -1)])
        }
    }
}
