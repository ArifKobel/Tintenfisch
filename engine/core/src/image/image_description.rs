use crate::color::ColorSpace;
use crate::error::{Error, Result};
use crate::store::ObjectStore;
use crate::syntax::{Dict, Name, Object};

const INLINE_KEYS: [(&str, &str); 10] = [
    ("BPC", "BitsPerComponent"),
    ("CS", "ColorSpace"),
    ("D", "Decode"),
    ("DP", "DecodeParms"),
    ("F", "Filter"),
    ("H", "Height"),
    ("IM", "ImageMask"),
    ("I", "Interpolate"),
    ("W", "Width"),
    ("L", "Length"),
];

#[derive(Clone, Debug, PartialEq)]
pub enum Masking {
    None,
    ColorKey(Vec<(u32, u32)>),
    Stencil(Object),
    Soft(Object),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageDescription {
    pub dict: Dict,
    pub width: usize,
    pub height: usize,
    pub bits_per_component: u32,
    pub color_space: Option<ColorSpace>,
    pub decode: Vec<(f64, f64)>,
    pub is_stencil: bool,
    pub interpolate: bool,
    pub masking: Masking,
}

impl ImageDescription {
    pub fn read(dict: &Dict, resources: &Dict, store: &ObjectStore) -> Result<ImageDescription> {
        let dict = expand_abbreviations(dict);
        let dimension = |key: &str| {
            dict.get(key)
                .map(|value| store.resolve(value))
                .transpose()?
                .and_then(|value| value.as_int())
                .filter(|&value| value > 0)
                .map(|value| value as usize)
                .ok_or_else(|| Error::new(format!("image without a valid /{key}")))
        };
        let width = dimension("Width")?;
        let height = dimension("Height")?;
        let is_stencil = dict.get("ImageMask").and_then(Object::as_bool).unwrap_or(false);
        let bits_per_component = if is_stencil { 1 } else { dict.int("BitsPerComponent").unwrap_or(8) as u32 };
        if ![1, 2, 4, 8, 16].contains(&bits_per_component) {
            return Err(Error::new("image with an invalid /BitsPerComponent"));
        }
        let color_space = match dict.get("ColorSpace").filter(|_| !is_stencil) {
            Some(space) => Some(ColorSpace::read(&expand_space(space), resources, store)?),
            None => None,
        };
        let components = color_space.as_ref().map_or(1, ColorSpace::components);
        let decode = match dict.get("Decode").and_then(Object::as_array) {
            Some(values) => values
                .iter()
                .filter_map(Object::as_number)
                .collect::<Vec<_>>()
                .chunks(2)
                .filter(|pair| pair.len() == 2)
                .map(|pair| (pair[0], pair[1]))
                .collect(),
            None => default_decode(color_space.as_ref(), bits_per_component, components),
        };
        let masking = if let Some(soft) = dict.get("SMask") {
            Masking::Soft(soft.clone())
        } else {
            match dict.get("Mask").map(|mask| store.resolve(mask)).transpose()? {
                Some(Object::Array(ranges)) => {
                    let values: Vec<u32> =
                        ranges.iter().filter_map(Object::as_int).map(|value| value.max(0) as u32).collect();
                    Masking::ColorKey(
                        values.chunks(2).filter(|pair| pair.len() == 2).map(|pair| (pair[0], pair[1])).collect(),
                    )
                }
                Some(Object::Stream(_)) => Masking::Stencil(dict.get("Mask").cloned().unwrap_or(Object::Null)),
                _ => Masking::None,
            }
        };
        let interpolate = dict.get("Interpolate").and_then(Object::as_bool).unwrap_or(false);
        Ok(ImageDescription {
            dict,
            width,
            height,
            bits_per_component,
            color_space,
            decode,
            is_stencil,
            interpolate,
            masking,
        })
    }

    pub fn components(&self) -> usize {
        self.color_space.as_ref().map_or(1, ColorSpace::components)
    }

    pub fn row_bytes(&self) -> usize {
        (self.width * self.components() * self.bits_per_component as usize).div_ceil(8)
    }
}

fn default_decode(space: Option<&ColorSpace>, bits: u32, components: usize) -> Vec<(f64, f64)> {
    match space {
        Some(ColorSpace::Indexed { .. }) => vec![(0.0, ((1u32 << bits) - 1) as f64)],
        Some(ColorSpace::Lab { .. }) => vec![(0.0, 100.0), (-100.0, 100.0), (-100.0, 100.0)],
        _ => vec![(0.0, 1.0); components],
    }
}

fn expand_abbreviations(dict: &Dict) -> Dict {
    dict.iter()
        .map(|(key, value)| {
            let full = INLINE_KEYS
                .iter()
                .find(|(short, _)| key.is(short))
                .map_or_else(|| key.clone(), |(_, long)| Name::from(*long));
            (full, value.clone())
        })
        .collect()
}

fn expand_space(space: &Object) -> Object {
    let expanded = |name: &Name| match name.as_bytes() {
        b"G" => Name::from("DeviceGray"),
        b"RGB" => Name::from("DeviceRGB"),
        b"CMYK" => Name::from("DeviceCMYK"),
        b"I" => Name::from("Indexed"),
        _ => name.clone(),
    };
    match space {
        Object::Name(name) => Object::Name(expanded(name)),
        Object::Array(items) => Object::Array(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| match item {
                    Object::Name(name) if index < 2 => Object::Name(expanded(name)),
                    other => other.clone(),
                })
                .collect(),
        ),
        other => other.clone(),
    }
}
