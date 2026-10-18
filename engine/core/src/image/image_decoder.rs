use crate::color::{ColorSpace, Rgb};
use crate::error::{Error, Result};
use crate::file::filters::{FilterChain, ImageCodec};
use crate::image::ccitt::{FaxDecoder, FaxParams};
use crate::image::jbig2::Jbig2Decoder;
use crate::image::jpeg;
use crate::image::jpx::{Jp2Color, JpxImage};
use crate::image::{DecodedImage, ImageDescription, Masking};
use crate::store::ObjectStore;
use crate::syntax::{Dict, Object};

pub struct ImageDecoder<'a> {
    store: &'a ObjectStore,
    resources: &'a Dict,
}

struct Samples {
    width: usize,
    height: usize,
    components: usize,
    bits: u32,
    row_bytes: usize,
    data: Vec<u8>,
    space: Option<ColorSpace>,
    opacity: Option<Vec<u8>>,
}

impl Samples {
    fn raw(&self, x: usize, y: usize, component: usize) -> u32 {
        let bits = self.bits as usize;
        if bits == 8 {
            return self.data.get(y * self.row_bytes + x * self.components + component).copied().unwrap_or(0) as u32;
        }
        let bit = (x * self.components + component) * bits;
        let row = &self.data[(y * self.row_bytes).min(self.data.len())..];
        (0..bits).fold(0, |value, offset| {
            let position = bit + offset;
            let byte = row.get(position / 8).copied().unwrap_or(0);
            value << 1 | ((byte >> (7 - position % 8)) & 1) as u32
        })
    }
}

impl<'a> ImageDecoder<'a> {
    pub fn new(store: &'a ObjectStore, resources: &'a Dict) -> Self {
        Self { store, resources }
    }

    pub fn decode(&self, description: &ImageDescription, raw: &[u8]) -> Result<DecodedImage> {
        let samples = self.samples(description, raw)?;
        let mut image = if description.is_stencil {
            stencil(&samples, &description.decode)
        } else {
            let space = description.color_space.clone().or_else(|| samples.space.clone()).unwrap_or(ColorSpace::Gray);
            colorize(&samples, &space, &description.decode, &description.masking)
        };
        match &description.masking {
            Masking::Soft(mask) | Masking::Stencil(mask) => self.apply_mask(&mut image, mask)?,
            Masking::None | Masking::ColorKey(_) => {}
        }
        if let Some(opacity) = samples.opacity.filter(|_| description.dict.int("SMaskInData").unwrap_or(0) > 0) {
            for (pixel, alpha) in image.rgba.chunks_mut(4).zip(opacity) {
                pixel[3] = alpha;
            }
        }
        Ok(image)
    }

    fn samples(&self, description: &ImageDescription, raw: &[u8]) -> Result<Samples> {
        let chain = FilterChain::of(&description.dict, |object| self.store.resolve(object).unwrap_or(Object::Null))?;
        let decoded = chain.decode(raw)?;
        let params = decoded.image.as_ref().map(|encoding| encoding.params.clone()).unwrap_or_default();
        match decoded.image.map(|encoding| encoding.codec) {
            None => Ok(Samples {
                width: description.width,
                height: description.height,
                components: description.components(),
                bits: description.bits_per_component,
                row_bytes: description.row_bytes(),
                data: decoded.data,
                space: None,
                opacity: None,
            }),
            Some(ImageCodec::Dct) => {
                let jpeg = jpeg::decode(&decoded.data, description.components())?;
                Ok(Samples {
                    width: jpeg.width,
                    height: jpeg.height,
                    components: jpeg.components,
                    bits: 8,
                    row_bytes: jpeg.width * jpeg.components,
                    data: jpeg.data,
                    space: None,
                    opacity: None,
                })
            }
            Some(ImageCodec::CcittFax) => {
                let params = FaxParams::read(&params, description.width, description.height);
                let (data, rows) = FaxDecoder::new(params).decode(&decoded.data);
                Ok(Samples {
                    width: params.columns,
                    height: rows,
                    components: 1,
                    bits: 1,
                    row_bytes: params.columns.div_ceil(8),
                    data,
                    space: None,
                    opacity: None,
                })
            }
            Some(ImageCodec::Jbig2) => {
                let globals = match params.get("JBIG2Globals") {
                    Some(globals) => {
                        let object = self.store.resolve(globals)?;
                        let stream = object.as_stream().ok_or_else(|| Error::new("JBIG2Globals is not a stream"))?;
                        self.store.decode_stream(stream)?.data
                    }
                    None => Vec::new(),
                };
                let (width, height, data) = Jbig2Decoder::decode(&globals, &decoded.data)?;
                Ok(Samples {
                    width,
                    height,
                    components: 1,
                    bits: 1,
                    row_bytes: width.div_ceil(8),
                    data,
                    space: None,
                    opacity: None,
                })
            }
            Some(ImageCodec::Jpx) => {
                let image = JpxImage::decode(&decoded.data)?;
                let indexed = matches!(description.color_space, Some(ColorSpace::Indexed { .. }));
                let to_byte = |value: u32, depth: u8| {
                    if indexed || depth == 8 {
                        value.min(255) as u8
                    } else {
                        let maximum = (1u64 << depth) - 1;
                        ((value as u64 * 255 + maximum / 2) / maximum) as u8
                    }
                };
                let components = image.channels.len();
                let mut data = Vec::with_capacity(image.width * image.height * components);
                for index in 0..image.width * image.height {
                    for (channel, depth) in image.channels.iter().zip(&image.depths) {
                        data.push(to_byte(channel[index], *depth));
                    }
                }
                let space = match (image.color, components) {
                    (Some(Jp2Color::Gray), _) | (_, 1) => ColorSpace::Gray,
                    (_, 4) => ColorSpace::Cmyk,
                    _ => ColorSpace::Rgb,
                };
                let depth = image.depths.first().copied().unwrap_or(8);
                let opacity = image.opacity.map(|alpha| alpha.into_iter().map(|value| to_byte(value, depth)).collect());
                Ok(Samples {
                    width: image.width,
                    height: image.height,
                    components,
                    bits: 8,
                    row_bytes: image.width * components,
                    data,
                    space: Some(space),
                    opacity,
                })
            }
        }
    }

    fn apply_mask(&self, image: &mut DecodedImage, mask: &Object) -> Result<()> {
        let object = self.store.resolve(mask)?;
        let Some(stream) = object.as_stream() else { return Ok(()) };
        let description = ImageDescription::read(&stream.dict, self.resources, self.store)?;
        let mask = if description.is_stencil {
            ImageDecoder::new(self.store, self.resources).decode(&description, &stream.raw)?
        } else {
            let gray = ImageDecoder::new(self.store, self.resources).decode(&description, &stream.raw)?;
            let mut alpha = gray.clone();
            for pixel in alpha.rgba.chunks_mut(4) {
                pixel[3] = pixel[0];
            }
            alpha
        };
        let (width, height) = (image.width, image.height);
        for y in 0..height {
            for x in 0..width {
                let alpha = &mut image.rgba[(y * width + x) * 4 + 3];
                *alpha = ((*alpha as u32 * mask.alpha_at(x, y, width, height) as u32) / 255) as u8;
            }
        }
        Ok(())
    }
}

fn normalized(raw: u32, bits: u32, (low, high): (f64, f64)) -> f64 {
    let max = ((1u64 << bits) - 1) as f64;
    low + raw as f64 * (high - low) / max
}

fn stencil(samples: &Samples, decode: &[(f64, f64)]) -> DecodedImage {
    let inverted = decode.first().is_some_and(|&(low, high)| low > high);
    let mut rgba = Vec::with_capacity(samples.width * samples.height * 4);
    for y in 0..samples.height {
        for x in 0..samples.width {
            let painted = (samples.raw(x, y, 0) == 0) != inverted;
            rgba.extend_from_slice(&[0, 0, 0, if painted { 255 } else { 0 }]);
        }
    }
    DecodedImage { width: samples.width, height: samples.height, rgba }
}

fn colorize(samples: &Samples, space: &ColorSpace, decode: &[(f64, f64)], masking: &Masking) -> DecodedImage {
    let components = samples.components;
    let keys = match masking {
        Masking::ColorKey(keys) => Some(keys),
        _ => None,
    };
    let single_byte = components == 1 && samples.bits <= 8;
    let palette: Option<Vec<Option<Rgb>>> = single_byte.then(|| {
        (0..1u32 << samples.bits)
            .map(|raw| space.to_rgb(&[normalized(raw, samples.bits, decode.first().copied().unwrap_or((0.0, 1.0)))]))
            .collect()
    });
    let mut rgba = Vec::with_capacity(samples.width * samples.height * 4);
    let mut values = vec![0.0; components];
    let mut raws = vec![0u32; components];
    for y in 0..samples.height {
        for x in 0..samples.width {
            for (component, raw) in raws.iter_mut().enumerate() {
                *raw = samples.raw(x, y, component);
            }
            let keyed = keys
                .is_some_and(|keys| raws.iter().zip(keys.iter()).all(|(raw, (low, high))| (low..=high).contains(&raw)));
            let color = match &palette {
                Some(palette) => palette[raws[0] as usize],
                None => {
                    for component in 0..components {
                        let range = decode.get(component).copied().unwrap_or((0.0, 1.0));
                        values[component] = normalized(raws[component], samples.bits, range);
                    }
                    space.to_rgb(&values)
                }
            };
            let color = color.unwrap_or(Rgb::gray(1.0));
            let to_byte = |value: f32| (value * 255.0).round() as u8;
            rgba.extend_from_slice(&[
                to_byte(color.r),
                to_byte(color.g),
                to_byte(color.b),
                if keyed { 0 } else { 255 },
            ]);
        }
    }
    DecodedImage { width: samples.width, height: samples.height, rgba }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(bits: u32, components: usize, width: usize, data: Vec<u8>) -> Samples {
        let row_bytes = (width * components * bits as usize).div_ceil(8);
        let height = data.len() / row_bytes;
        Samples { width, height, components, bits, row_bytes, data, space: None, opacity: None }
    }

    #[test]
    fn reads_samples_of_every_depth_with_padded_rows() {
        let four = samples(4, 1, 3, vec![0x12, 0x30, 0x45, 0x60]);
        assert_eq!((four.raw(0, 0, 0), four.raw(1, 0, 0), four.raw(2, 0, 0), four.raw(0, 1, 0)), (1, 2, 3, 4));
        let sixteen = samples(16, 1, 1, vec![0xab, 0xcd]);
        assert_eq!(sixteen.raw(0, 0, 0), 0xabcd);
    }

    #[test]
    fn paints_stencil_zeros_and_honours_an_inverted_decode() {
        let mask = samples(1, 1, 2, vec![0b0100_0000]);
        assert_eq!(stencil(&mask, &[(0.0, 1.0)]).rgba, vec![0, 0, 0, 255, 0, 0, 0, 0]);
        assert_eq!(stencil(&mask, &[(1.0, 0.0)]).rgba, vec![0, 0, 0, 0, 0, 0, 0, 255]);
    }

    #[test]
    fn colorizes_through_the_space_and_masks_color_keys() {
        let rgb = samples(8, 3, 2, vec![255, 0, 0, 0, 0, 255]);
        let image =
            colorize(&rgb, &ColorSpace::Rgb, &[(0.0, 1.0); 3], &Masking::ColorKey(vec![(0, 0), (0, 0), (250, 255)]));
        assert_eq!(image.rgba, vec![255, 0, 0, 255, 0, 0, 255, 0]);
        let gray = samples(1, 1, 2, vec![0b1000_0000]);
        assert_eq!(
            colorize(&gray, &ColorSpace::Gray, &[(1.0, 0.0)], &Masking::None).rgba,
            vec![0, 0, 0, 255, 255, 255, 255, 255]
        );
    }
}
