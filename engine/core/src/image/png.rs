use std::io::Read;

use flate2::read::ZlibDecoder;

use crate::error::{Error, Result};
use crate::image::DecodedImage;

const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const MAX_PIXELS: usize = 1 << 26;
const ADAM7: [(usize, usize, usize, usize); 7] =
    [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)];

struct Header {
    width: usize,
    height: usize,
    depth: usize,
    color_type: u8,
    interlaced: bool,
}

impl Header {
    fn channels(&self) -> usize {
        match self.color_type {
            2 => 3,
            4 => 2,
            6 => 4,
            _ => 1,
        }
    }

    fn bits_per_pixel(&self) -> usize {
        self.channels() * self.depth
    }
}

struct Colors {
    palette: Vec<[u8; 3]>,
    palette_alpha: Vec<u8>,
    transparent: Option<Vec<u16>>,
}

pub fn decode_png(data: &[u8]) -> Result<DecodedImage> {
    let body = data.strip_prefix(SIGNATURE).ok_or_else(|| Error::new("not a PNG file"))?;
    let mut header = None;
    let mut colors = Colors { palette: Vec::new(), palette_alpha: Vec::new(), transparent: None };
    let mut compressed = Vec::new();
    let mut rest = body;
    while rest.len() >= 12 {
        let length = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
        let kind = &rest[4..8];
        let chunk = rest.get(8..8 + length).ok_or_else(|| Error::new("PNG chunk runs past the file"))?;
        match kind {
            b"IHDR" => header = Some(read_header(chunk)?),
            b"PLTE" => colors.palette = chunk.as_chunks::<3>().0.to_vec(),
            b"tRNS" => match header.as_ref().map(|header| header.color_type) {
                Some(3) => colors.palette_alpha = chunk.to_vec(),
                Some(0 | 2) => {
                    colors.transparent =
                        Some(chunk.as_chunks::<2>().0.iter().map(|pair| u16::from_be_bytes(*pair)).collect())
                }
                _ => {}
            },
            b"IDAT" => compressed.extend_from_slice(chunk),
            b"IEND" => break,
            _ => {}
        }
        rest = rest.get(12 + length..).unwrap_or_default();
    }
    let header = header.ok_or_else(|| Error::new("PNG without a header"))?;
    let mut filtered = Vec::new();
    ZlibDecoder::new(compressed.as_slice())
        .read_to_end(&mut filtered)
        .map_err(|error| Error::new(format!("PNG data: {error}")))?;
    let mut rgba = vec![0u8; header.width * header.height * 4];
    let passes: Vec<(usize, usize, usize, usize)> = if header.interlaced { ADAM7.to_vec() } else { vec![(0, 0, 1, 1)] };
    let mut offset = 0;
    for (x0, y0, step_x, step_y) in passes {
        let columns = header.width.saturating_sub(x0).div_ceil(step_x);
        let rows = header.height.saturating_sub(y0).div_ceil(step_y);
        if columns == 0 || rows == 0 {
            continue;
        }
        let row_bytes = (columns * header.bits_per_pixel()).div_ceil(8);
        let pass =
            filtered.get(offset..offset + rows * (row_bytes + 1)).ok_or_else(|| Error::new("PNG data ends early"))?;
        offset += rows * (row_bytes + 1);
        let unfiltered = unfilter(pass, row_bytes, header.bits_per_pixel().div_ceil(8))?;
        for (row, line) in unfiltered.chunks_exact(row_bytes).enumerate() {
            for column in 0..columns {
                let (x, y) = (x0 + column * step_x, y0 + row * step_y);
                let target = (y * header.width + x) * 4;
                rgba[target..target + 4].copy_from_slice(&pixel(&header, &colors, line, column));
            }
        }
    }
    Ok(DecodedImage { width: header.width, height: header.height, rgba })
}

fn read_header(chunk: &[u8]) -> Result<Header> {
    let [w0, w1, w2, w3, h0, h1, h2, h3, depth, color_type, _, _, interlace, ..] = *chunk else {
        return Err(Error::new("PNG header is too short"));
    };
    let header = Header {
        width: u32::from_be_bytes([w0, w1, w2, w3]) as usize,
        height: u32::from_be_bytes([h0, h1, h2, h3]) as usize,
        depth: depth as usize,
        color_type,
        interlaced: interlace == 1,
    };
    let valid_depth = match color_type {
        0 => [1, 2, 4, 8, 16].contains(&header.depth),
        3 => [1, 2, 4, 8].contains(&header.depth),
        2 | 4 | 6 => [8, 16].contains(&header.depth),
        _ => false,
    };
    if !valid_depth {
        return Err(Error::unsupported(format!("PNG color type {color_type} at {depth} bits")));
    }
    if header.width == 0 || header.height == 0 || header.width.saturating_mul(header.height) > MAX_PIXELS {
        return Err(Error::unsupported(format!("a PNG of {} by {} pixels", header.width, header.height)));
    }
    Ok(header)
}

fn unfilter(pass: &[u8], row_bytes: usize, pixel_bytes: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(pass.len());
    let mut previous = vec![0u8; row_bytes];
    for row in pass.chunks_exact(row_bytes + 1) {
        let (filter, data) = (row[0], &row[1..]);
        let mut current = data.to_vec();
        for index in 0..row_bytes {
            let left = if index >= pixel_bytes { current[index - pixel_bytes] } else { 0 };
            let up = previous[index];
            let up_left = if index >= pixel_bytes { previous[index - pixel_bytes] } else { 0 };
            let prediction = match filter {
                0 => 0,
                1 => left,
                2 => up,
                3 => ((left as u16 + up as u16) / 2) as u8,
                4 => paeth(left, up, up_left),
                _ => return Err(Error::new(format!("PNG row filter {filter}"))),
            };
            current[index] = current[index].wrapping_add(prediction);
        }
        out.extend_from_slice(&current);
        previous = current;
    }
    Ok(out)
}

fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let estimate = left as i16 + up as i16 - up_left as i16;
    let (to_left, to_up, to_up_left) =
        ((estimate - left as i16).abs(), (estimate - up as i16).abs(), (estimate - up_left as i16).abs());
    if to_left <= to_up && to_left <= to_up_left {
        left
    } else if to_up <= to_up_left {
        up
    } else {
        up_left
    }
}

fn sample(line: &[u8], depth: usize, index: usize) -> u16 {
    match depth {
        16 => u16::from_be_bytes([line[index * 2], line[index * 2 + 1]]),
        8 => line[index] as u16,
        _ => {
            let bit = index * depth;
            ((line[bit / 8] >> (8 - depth - bit % 8)) & ((1 << depth) - 1) as u8) as u16
        }
    }
}

fn pixel(header: &Header, colors: &Colors, line: &[u8], column: usize) -> [u8; 4] {
    let channels = header.channels();
    let raw: Vec<u16> = (0..channels).map(|channel| sample(line, header.depth, column * channels + channel)).collect();
    let max = (1u32 << header.depth) - 1;
    let scaled = |value: u16| if header.depth == 16 { (value >> 8) as u8 } else { (value as u32 * 255 / max) as u8 };
    let opaque = |samples: &[u16]| if colors.transparent.as_deref() == Some(samples) { 0 } else { 255 };
    match header.color_type {
        0 => {
            let gray = scaled(raw[0]);
            [gray, gray, gray, opaque(&raw)]
        }
        2 => [scaled(raw[0]), scaled(raw[1]), scaled(raw[2]), opaque(&raw)],
        3 => {
            let index = raw[0] as usize;
            let [red, green, blue] = colors.palette.get(index).copied().unwrap_or_default();
            [red, green, blue, colors.palette_alpha.get(index).copied().unwrap_or(255)]
        }
        4 => {
            let gray = scaled(raw[0]);
            [gray, gray, gray, scaled(raw[1])]
        }
        _ => [scaled(raw[0]), scaled(raw[1]), scaled(raw[2]), scaled(raw[3])],
    }
}
