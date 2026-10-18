use crate::image::jbig2::bitmap::Bitmap;
use crate::image::mq_decoder::{MqContext, MqDecoder};

const TYPICAL_CONTEXTS: [usize; 4] = [0x9b25, 0x0795, 0x00e5, 0x0195];
const CONTEXT_BITS: [u32; 4] = [16, 13, 10, 10];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenericRegion<'s> {
    pub width: usize,
    pub height: usize,
    pub template: u8,
    pub typical_prediction: bool,
    pub adaptive: Vec<(i64, i64)>,
    pub skip: Option<&'s Bitmap>,
}

impl GenericRegion<'_> {
    pub fn contexts(template: u8) -> Vec<MqContext> {
        vec![MqContext::default(); 1 << CONTEXT_BITS[template as usize & 3]]
    }

    pub fn decode(&self, decoder: &mut MqDecoder, contexts: &mut [MqContext]) -> Bitmap {
        let mut bitmap = Bitmap::new(self.width, self.height, false);
        let neighbours = self.neighbours();
        let mut typical = false;
        for y in 0..self.height {
            if self.typical_prediction {
                typical ^= decoder.decode(&mut contexts[TYPICAL_CONTEXTS[self.template as usize & 3]]) == 1;
                if typical {
                    if y > 0 {
                        bitmap.copy_row(y - 1, y);
                    }
                    continue;
                }
            }
            for x in 0..self.width {
                let (x, y) = (x as i64, y as i64);
                if self.skip.is_some_and(|skip| skip.get(x, y) == 1) {
                    continue;
                }
                let context = neighbours
                    .iter()
                    .enumerate()
                    .fold(0, |context, (bit, (dx, dy))| context | (bitmap.get(x + dx, y + dy) as usize) << bit);
                if decoder.decode(&mut contexts[context]) == 1 {
                    bitmap.set(x as usize, y as usize, 1);
                }
            }
        }
        bitmap
    }

    fn neighbours(&self) -> Vec<(i64, i64)> {
        let adaptive = |index: usize| self.adaptive[index];
        match self.template {
            0 => vec![
                (-1, 0),
                (-2, 0),
                (-3, 0),
                (-4, 0),
                adaptive(0),
                (2, -1),
                (1, -1),
                (0, -1),
                (-1, -1),
                (-2, -1),
                adaptive(1),
                adaptive(2),
                (1, -2),
                (0, -2),
                (-1, -2),
                adaptive(3),
            ],
            1 => vec![
                (-1, 0),
                (-2, 0),
                (-3, 0),
                adaptive(0),
                (2, -1),
                (1, -1),
                (0, -1),
                (-1, -1),
                (-2, -1),
                (2, -2),
                (1, -2),
                (0, -2),
                (-1, -2),
            ],
            2 => vec![(-1, 0), (-2, 0), adaptive(0), (1, -1), (0, -1), (-1, -1), (-2, -1), (1, -2), (0, -2), (-1, -2)],
            _ => vec![(-1, 0), (-2, 0), (-3, 0), (-4, 0), adaptive(0), (1, -1), (0, -1), (-1, -1), (-2, -1), (-3, -1)],
        }
    }
}
