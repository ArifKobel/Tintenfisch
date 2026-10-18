use crate::color::Rgb;
use crate::error::{Error, Result};
use crate::function::PdfFunction;
use crate::store::ObjectStore;
use crate::syntax::{Dict, Name, Object};

const D65: [f64; 3] = [0.9505, 1.0, 1.089];
const MAX_NESTING: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub enum ColorSpace {
    Gray,
    Rgb,
    Cmyk,
    Lab { white: [f64; 3] },
    Indexed { base: Box<ColorSpace>, high: usize, lookup: Vec<u8> },
    Separation { alternate: Box<ColorSpace>, tint: PdfFunction, paints: bool },
    DeviceN { inputs: usize, alternate: Box<ColorSpace>, tint: PdfFunction },
    Pattern(Option<Box<ColorSpace>>),
}

impl ColorSpace {
    pub fn read(object: &Object, resources: &Dict, store: &ObjectStore) -> Result<ColorSpace> {
        Self::read_nested(object, resources, store, 0)
    }

    fn read_nested(object: &Object, resources: &Dict, store: &ObjectStore, depth: usize) -> Result<ColorSpace> {
        if depth > MAX_NESTING {
            return Err(Error::new("color spaces nested too deeply"));
        }
        let object = store.resolve(object)?;
        match &object {
            Object::Name(name) => match Self::device(name) {
                Some(space) => Ok(space),
                None => {
                    let named = resources
                        .get("ColorSpace")
                        .map(|spaces| store.resolve(spaces))
                        .transpose()?
                        .and_then(|spaces| spaces.as_dict().and_then(|spaces| spaces.get_name(name).cloned()))
                        .ok_or_else(|| Error::new(format!("unknown color space {name:?}")))?;
                    Self::read_nested(&named, resources, store, depth + 1)
                }
            },
            Object::Array(items) => Self::family(items, resources, store, depth),
            _ => Err(Error::new("color space is neither a name nor an array")),
        }
    }

    fn device(name: &Name) -> Option<ColorSpace> {
        Some(match name.as_bytes() {
            b"DeviceGray" | b"G" | b"CalGray" => ColorSpace::Gray,
            b"DeviceRGB" | b"RGB" | b"CalRGB" => ColorSpace::Rgb,
            b"DeviceCMYK" | b"CMYK" => ColorSpace::Cmyk,
            b"Pattern" => ColorSpace::Pattern(None),
            _ => return None,
        })
    }

    fn family(items: &[Object], resources: &Dict, store: &ObjectStore, depth: usize) -> Result<ColorSpace> {
        let family =
            items.first().and_then(Object::as_name).ok_or_else(|| Error::new("color space array without a family"))?;
        let parameter = |index: usize| -> Result<Object> { store.resolve(items.get(index).unwrap_or(&Object::Null)) };
        match family.as_bytes() {
            b"DeviceGray" | b"CalGray" => Ok(ColorSpace::Gray),
            b"DeviceRGB" | b"CalRGB" => Ok(ColorSpace::Rgb),
            b"DeviceCMYK" => Ok(ColorSpace::Cmyk),
            b"Pattern" => Ok(ColorSpace::Pattern(match items.get(1) {
                Some(base) => Some(Box::new(Self::read_nested(base, resources, store, depth + 1)?)),
                None => None,
            })),
            b"Lab" => {
                let dict = parameter(1)?;
                let white = dict
                    .as_dict()
                    .and_then(|dict| dict.get("WhitePoint"))
                    .and_then(Object::as_array)
                    .map(|values| values.iter().filter_map(Object::as_number).collect::<Vec<_>>())
                    .filter(|values| values.len() == 3)
                    .map_or(D65, |values| [values[0], values[1], values[2]]);
                Ok(ColorSpace::Lab { white })
            }
            b"ICCBased" => {
                let profile = parameter(1)?;
                let dict = profile.as_dict().ok_or_else(|| Error::new("ICCBased without a profile stream"))?;
                if let Some(alternate) = dict.get("Alternate") {
                    return Self::read_nested(alternate, resources, store, depth + 1);
                }
                Ok(match dict.int("N") {
                    Some(1) => ColorSpace::Gray,
                    Some(4) => ColorSpace::Cmyk,
                    _ => ColorSpace::Rgb,
                })
            }
            b"Indexed" | b"I" => {
                let base = Self::read_nested(items.get(1).unwrap_or(&Object::Null), resources, store, depth + 1)?;
                let high = parameter(2)?.as_int().unwrap_or(0).clamp(0, 255) as usize;
                let lookup = match parameter(3)? {
                    Object::String(bytes) => bytes,
                    Object::Stream(stream) => store.decode_stream(&stream)?.data,
                    _ => return Err(Error::new("Indexed color space without a lookup table")),
                };
                Ok(ColorSpace::Indexed { base: Box::new(base), high, lookup })
            }
            b"Separation" => {
                let colorant = parameter(1)?;
                let alternate = Self::read_nested(items.get(2).unwrap_or(&Object::Null), resources, store, depth + 1)?;
                let tint = PdfFunction::read(items.get(3).unwrap_or(&Object::Null), store)?;
                let paints = !colorant.as_name().is_some_and(|name| name.is("None"));
                Ok(ColorSpace::Separation { alternate: Box::new(alternate), tint, paints })
            }
            b"DeviceN" => {
                let inputs = parameter(1)?.as_array().map_or(1, <[Object]>::len);
                let alternate = Self::read_nested(items.get(2).unwrap_or(&Object::Null), resources, store, depth + 1)?;
                let tint = PdfFunction::read(items.get(3).unwrap_or(&Object::Null), store)?;
                Ok(ColorSpace::DeviceN { inputs, alternate: Box::new(alternate), tint })
            }
            _ => Err(Error::new(format!("unsupported color space {family:?}"))),
        }
    }

    pub fn components(&self) -> usize {
        match self {
            ColorSpace::Gray | ColorSpace::Indexed { .. } | ColorSpace::Separation { .. } => 1,
            ColorSpace::Pattern(base) => base.as_ref().map_or(1, |base| base.components()),
            ColorSpace::Rgb | ColorSpace::Lab { .. } => 3,
            ColorSpace::Cmyk => 4,
            ColorSpace::DeviceN { inputs, .. } => *inputs,
        }
    }

    pub fn initial_color(&self) -> Vec<f64> {
        match self {
            ColorSpace::Cmyk => vec![0.0, 0.0, 0.0, 1.0],
            ColorSpace::Lab { .. } => vec![0.0, 0.0, 0.0],
            ColorSpace::Separation { .. } | ColorSpace::DeviceN { .. } => vec![1.0; self.components()],
            _ => vec![0.0; self.components()],
        }
    }

    pub fn to_rgb(&self, components: &[f64]) -> Option<Rgb> {
        let value = |index: usize| components.get(index).copied().unwrap_or(0.0);
        Some(match self {
            ColorSpace::Gray => Rgb::gray(value(0)),
            ColorSpace::Rgb => Rgb::new(value(0), value(1), value(2)),
            ColorSpace::Cmyk => Rgb::from_cmyk(value(0), value(1), value(2), value(3)),
            ColorSpace::Lab { white } => Rgb::from_lab(value(0), value(1), value(2), *white),
            ColorSpace::Indexed { base, high, lookup } => {
                let index = (value(0).round().max(0.0) as usize).min(*high);
                let width = base.components();
                let entry: Vec<f64> = (0..width)
                    .map(|component| lookup.get(index * width + component).copied().unwrap_or(0) as f64 / 255.0)
                    .collect();
                return base.to_rgb(&entry);
            }
            ColorSpace::Separation { alternate, tint, paints } => {
                if !paints {
                    return None;
                }
                return alternate.to_rgb(&tint.evaluate(&components[..components.len().min(1)]));
            }
            ColorSpace::DeviceN { alternate, tint, .. } => return alternate.to_rgb(&tint.evaluate(components)),
            ColorSpace::Pattern(base) => return base.as_ref().and_then(|base| base.to_rgb(components)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_indexed_entries_through_the_base_space() {
        let space =
            ColorSpace::Indexed { base: Box::new(ColorSpace::Rgb), high: 1, lookup: vec![255, 0, 0, 0, 0, 255] };
        assert_eq!(space.to_rgb(&[1.0]), Some(Rgb::new(0.0, 0.0, 1.0)));
        assert_eq!(space.to_rgb(&[7.0]), Some(Rgb::new(0.0, 0.0, 1.0)));
        assert_eq!(space.components(), 1);
    }

    #[test]
    fn starts_each_space_with_its_initial_color() {
        assert_eq!(ColorSpace::Cmyk.to_rgb(&ColorSpace::Cmyk.initial_color()), Some(Rgb::BLACK));
        assert_eq!(ColorSpace::Gray.initial_color(), vec![0.0]);
    }
}
