use crate::error::{Error, Result};
use crate::image::jpx::codestream::Codestream;
use crate::image::jpx::jp2_file::{Jp2Color, Jp2File};
use crate::image::jpx::tile_decoder::TileDecoder;

pub struct JpxImage {
    pub width: usize,
    pub height: usize,
    pub channels: Vec<Vec<u32>>,
    pub depths: Vec<u8>,
    pub color: Option<Jp2Color>,
    pub opacity: Option<Vec<u32>>,
}

impl JpxImage {
    pub fn decode(data: &[u8]) -> Result<JpxImage> {
        let file = Jp2File::read(data)?;
        let codestream = Codestream::parse(file.codestream)?;
        let size = &codestream.size;
        let areas: Vec<(u32, u32, usize, usize)> = size
            .components
            .iter()
            .map(|component| {
                let (x0, y0) = (size.x0.div_ceil(component.step_x), size.y0.div_ceil(component.step_y));
                let (x1, y1) = (size.x1.div_ceil(component.step_x), size.y1.div_ceil(component.step_y));
                (x0, y0, (x1 - x0) as usize, (y1 - y0) as usize)
            })
            .collect();
        let mut planes: Vec<Vec<f64>> = areas.iter().map(|(_, _, width, height)| vec![0.0; width * height]).collect();
        for tile in &codestream.tiles {
            let mut decoded = TileDecoder::new(size, tile).decode();
            if tile.progression.multiple_component_transform && decoded.components.len() >= 3 {
                let reversible = tile.coding[0].reversible;
                let (first, rest) = decoded.components.split_at_mut(1);
                let (second, third) = rest.split_at_mut(1);
                let (y, u, v) = (&mut first[0].4, &mut second[0].4, &mut third[0].4);
                for index in 0..y.len().min(u.len()).min(v.len()) {
                    let (luma, blue, red) = (y[index], u[index], v[index]);
                    let (r, g, b) = if reversible {
                        let green = luma - ((blue + red) / 4.0).floor();
                        (red + green, green, blue + green)
                    } else {
                        (luma + 1.402 * red, luma - 0.344_13 * blue - 0.714_14 * red, luma + 1.772 * blue)
                    };
                    (y[index], u[index], v[index]) = (r, g, b);
                }
            }
            for (component, (x0, y0, width, height, samples)) in decoded.components.into_iter().enumerate() {
                let (area_x0, area_y0, plane_width, plane_height) = areas[component];
                for row in 0..height {
                    let target_y = (y0 - area_y0) as usize + row;
                    if target_y >= plane_height {
                        continue;
                    }
                    for column in 0..width {
                        let target_x = (x0 - area_x0) as usize + column;
                        if target_x < plane_width {
                            planes[component][target_y * plane_width + target_x] = samples[row * width + column];
                        }
                    }
                }
            }
        }
        let mut channels: Vec<Vec<u32>> = Vec::new();
        let mut depths: Vec<u8> = Vec::new();
        for (component, plane) in planes.iter().enumerate() {
            let info = size.components[component];
            let maximum = (1u64 << info.depth) as f64 - 1.0;
            let offset = if info.signed { 0.0 } else { (1u64 << (info.depth - 1)) as f64 };
            channels.push(plane.iter().map(|value| (value + offset).round().clamp(0.0, maximum) as u32).collect());
            depths.push(info.depth);
        }
        let (_, _, width, height) = areas[0];
        for (component, (_, _, other_width, other_height)) in areas.iter().enumerate().skip(1) {
            if (*other_width, *other_height) != (width, height) {
                let source = std::mem::take(&mut channels[component]);
                channels[component] = (0..width * height)
                    .map(|index| {
                        let (x, y) = (index % width * other_width / width, index / width * other_height / height);
                        source.get(y * other_width + x).copied().unwrap_or(0)
                    })
                    .collect();
            }
        }
        if let Some(palette) = &file.palette {
            let index = channels.first().ok_or_else(|| Error::new("a JPEG 2000 palette has nothing to index"))?.clone();
            let mapping = if palette.mapping.is_empty() {
                (0..palette.depths.len()).map(|column| (0, Some(column))).collect()
            } else {
                palette.mapping.clone()
            };
            let mut mapped = Vec::new();
            let mut mapped_depths = Vec::new();
            for (component, column) in mapping {
                match column {
                    Some(column) => {
                        mapped.push(
                            index
                                .iter()
                                .map(|value| {
                                    palette
                                        .entries
                                        .get(*value as usize)
                                        .and_then(|entry| entry.get(column))
                                        .copied()
                                        .unwrap_or(0)
                                })
                                .collect(),
                        );
                        mapped_depths.push(palette.depths[column]);
                    }
                    None => {
                        mapped.push(channels.get(component).cloned().unwrap_or_default());
                        mapped_depths.push(depths.get(component).copied().unwrap_or(8));
                    }
                }
            }
            channels = mapped;
            depths = mapped_depths;
        }
        let opacity = file.opacity.filter(|channel| *channel < channels.len()).map(|channel| {
            depths.remove(channel);
            channels.remove(channel)
        });
        if file.color == Some(Jp2Color::Ycc) && channels.len() >= 3 {
            let middle = (1u32 << (depths[0] - 1)) as f64;
            let maximum = ((1u64 << depths[0]) - 1) as f64;
            let [first, second, third, ..] = &mut channels[..] else { unreachable!() };
            for ((luma, blue), red) in first.iter_mut().zip(second.iter_mut()).zip(third.iter_mut()) {
                let (y, u, v) = (*luma as f64, *blue as f64 - middle, *red as f64 - middle);
                let to_level = |value: f64| value.round().clamp(0.0, maximum) as u32;
                (*luma, *blue, *red) =
                    (to_level(y + 1.402 * v), to_level(y - 0.344_136 * u - 0.714_136 * v), to_level(y + 1.772 * u));
            }
        }
        Ok(JpxImage { width, height, channels, depths, color: file.color, opacity })
    }
}
