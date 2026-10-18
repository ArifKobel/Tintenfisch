use crate::support;

use std::fs;
use std::path::PathBuf;

use support::synthetic_pdf::{Row, SyntheticPdf};
use tintenfisch::document::Document;
use tintenfisch::image::{ImageDecoder, ImageDescription, JpxImage};
use tintenfisch::syntax::Object;

fn read_pnm(bytes: &[u8]) -> (usize, usize, usize, u32, Vec<u32>) {
    let mut tokens = Vec::new();
    let mut index = 2;
    while tokens.len() < 3 {
        while bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes[index] == b'#' {
            while bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        let start = index;
        while !bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        tokens.push(String::from_utf8_lossy(&bytes[start..index]).parse::<u32>().unwrap());
    }
    let channels = if &bytes[..2] == b"P6" { 3 } else { 1 };
    let data = &bytes[index + 1..];
    let samples = if tokens[2] > 255 {
        data.chunks(2).map(|pair| u32::from(pair[0]) << 8 | u32::from(pair[1])).collect()
    } else {
        data.iter().map(|byte| u32::from(*byte)).collect()
    };
    (tokens[0] as usize, tokens[1] as usize, channels, tokens[2], samples)
}

fn jpx_names() -> Vec<String> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/jpx");
    let mut names: Vec<String> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".j2k") || name.ends_with(".jp2"))
        .collect();
    names.sort();
    names
}

fn jp2(codestream: &[u8], boxes: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let wrap = |kind: &[u8; 4], content: &[u8]| {
        let mut bytes = ((content.len() + 8) as u32).to_be_bytes().to_vec();
        bytes.extend(kind);
        bytes.extend(content);
        bytes
    };
    let header: Vec<u8> = boxes.iter().flat_map(|(kind, content)| wrap(kind, content)).collect();
    let mut file = wrap(b"jP  ", &[0x0d, 0x0a, 0x87, 0x0a]);
    file.extend(wrap(b"ftyp", b"jp2 \0\0\0\0jp2 "));
    file.extend(wrap(b"jp2h", &header));
    file.extend(wrap(b"jp2c", codestream));
    file
}

fn enumerated_color(space: u32) -> Vec<u8> {
    let mut content = vec![1, 0, 0];
    content.extend(space.to_be_bytes());
    content
}

#[test]
fn palettes_and_ycc_headers_turn_the_components_into_color() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/jpx");
    let (_, _, _, _, gray) = read_pnm(&fs::read(directory.join("gray-lossless.reference.pgm")).unwrap());
    let entry = |index: u32| [index, 255 - index, index / 2];
    let mut palette = vec![1, 0, 3, 7, 7, 7];
    for index in 0..256u32 {
        palette.extend(entry(index).iter().map(|value| *value as u8));
    }
    let mapping = vec![0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 1, 2];
    let file = jp2(
        &fs::read(directory.join("gray-lossless.j2k")).unwrap(),
        &[(b"colr", enumerated_color(16)), (b"pclr", palette), (b"cmap", mapping)],
    );
    let image = JpxImage::decode(&file).unwrap();
    assert_eq!(image.channels.len(), 3);
    for (index, value) in gray.iter().enumerate() {
        assert_eq!(
            [image.channels[0][index], image.channels[1][index], image.channels[2][index]],
            entry(*value),
            "pixel {index}"
        );
    }
    let (_, _, _, _, rgb) = read_pnm(&fs::read(directory.join("rgb-no-mct.reference.ppm")).unwrap());
    let ycc = JpxImage::decode(&jp2(
        &fs::read(directory.join("rgb-no-mct.j2k")).unwrap(),
        &[(b"colr", enumerated_color(18))],
    ))
    .unwrap();
    for index in 0..ycc.width * ycc.height {
        let (y, u, v) = (rgb[index * 3] as f64, rgb[index * 3 + 1] as f64 - 128.0, rgb[index * 3 + 2] as f64 - 128.0);
        let expected = [y + 1.402 * v, y - 0.344_136 * u - 0.714_136 * v, y + 1.772 * u]
            .map(|value| value.round().clamp(0.0, 255.0) as u32);
        assert_eq!([ycc.channels[0][index], ycc.channels[1][index], ycc.channels[2][index]], expected, "pixel {index}");
    }
}

fn jpx_document(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.5");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
        .object(3, 0, &format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Contents 4 0 R /Resources << /XObject << /Jpx 5 0 R >> >> >>"))
        .stream(4, "", format!("q {width} 0 0 {height} 0 0 cm /Jpx Do Q").as_bytes())
        .stream(5, &format!("/Type /XObject /Subtype /Image /Width {width} /Height {height} /BitsPerComponent 8 /SMaskInData 1 /Filter /JPXDecode"), data);
    let rows: Vec<(u32, Row)> =
        std::iter::once((0, Row::Free(65535))).chain((1..=5).map(|num| (num, Row::InUse(num)))).collect();
    let start = pdf.xref_table(&rows, "/Size 6 /Root 1 0 R");
    pdf.startxref(start).finish()
}

fn decode(bytes: Vec<u8>) -> (usize, usize, Vec<u8>) {
    let document = Document::open(bytes, "").unwrap();
    let library = support::rendering::library();
    let page = document.view(&library).pages().unwrap()[0].clone();
    let store = document.store();
    let reference = page
        .resources
        .get("XObject")
        .and_then(Object::as_dict)
        .and_then(|xobjects| xobjects.get("Jpx"))
        .and_then(Object::as_reference)
        .unwrap();
    let object = store.object(reference).unwrap();
    let stream = object.as_stream().unwrap();
    let description = ImageDescription::read(&stream.dict, &page.resources, store).unwrap();
    let image = ImageDecoder::new(store, &page.resources).decode(&description, &stream.raw).unwrap();
    (image.width, image.height, image.rgba)
}

#[test]
fn every_openjpeg_variant_decodes_like_opj_decompress() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/jpx");
    let mut failures = Vec::new();
    for name in jpx_names() {
        let stem = name.rsplit_once('.').unwrap().0;
        let reference = fs::read(directory.join(format!("{stem}.reference.ppm")))
            .or_else(|_| fs::read(directory.join(format!("{stem}.reference.pgm"))))
            .unwrap();
        let (width, height, channels, maximum, samples) = read_pnm(&reference);
        let data = fs::read(directory.join(&name)).unwrap();
        let cod = data.windows(2).position(|pair| pair == [0xff, 0x52]).unwrap();
        let reversible = data[cod + 13] == 1;
        let image = match std::panic::catch_unwind(|| JpxImage::decode(&data)) {
            Ok(Ok(image)) => image,
            other => {
                failures.push(format!("{stem}: {:?}", other.map(|result| result.err())));
                continue;
            }
        };
        if (image.width, image.height, image.channels.len()) != (width, height, channels) {
            failures.push(format!("{stem}: {}x{} with {} channels", image.width, image.height, image.channels.len()));
            continue;
        }
        let (mut worst, mut squared) = (0i64, 0f64);
        for (index, expected) in samples.iter().enumerate() {
            let difference = (image.channels[index % channels][index / channels] as i64 - *expected as i64).abs();
            worst = worst.max(difference);
            squared += (difference * difference) as f64;
        }
        if let Ok(alpha) = fs::read(directory.join(format!("{stem}.alpha.pgm"))) {
            let (_, _, _, _, expected) = read_pnm(&alpha);
            if image.opacity.as_deref() != Some(expected.as_slice()) {
                failures.push(format!("{stem}: the opacity channel differs"));
            }
        }
        let error = (squared / samples.len() as f64).sqrt();
        let close = if reversible {
            worst == 0
        } else {
            worst <= 2 * (maximum as i64 / 255).max(1)
                && (error == 0.0 || 20.0 * (maximum as f64 / error).log10() >= 50.0)
        };
        if !close {
            failures.push(format!("{stem}: worst difference {worst}, error {error:.3}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn an_image_without_a_color_space_takes_it_from_its_components() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/jpx");
    for (name, reference) in
        [("rgb-lossless.j2k", "rgb-lossless.reference.ppm"), ("gray-jp2.jp2", "gray-jp2.reference.pgm")]
    {
        let (width, height, channels, _, samples) = read_pnm(&fs::read(directory.join(reference)).unwrap());
        let (decoded_width, decoded_height, rgba) =
            decode(jpx_document(&fs::read(directory.join(name)).unwrap(), width, height));
        assert_eq!((decoded_width, decoded_height), (width, height));
        let wrong = rgba
            .chunks(4)
            .enumerate()
            .filter(|(index, pixel)| {
                (0..3).any(|channel| {
                    pixel[channel] as u32 != samples[index * channels + if channels == 1 { 0 } else { channel }]
                })
            })
            .count();
        assert_eq!(wrong, 0, "{name}");
    }
    let (width, height, _, _, alpha) = read_pnm(&fs::read(directory.join("rgba-jp2.alpha.pgm")).unwrap());
    let (_, _, rgba) = decode(jpx_document(&fs::read(directory.join("rgba-jp2.jp2")).unwrap(), width, height));
    assert!(
        rgba.chunks(4).zip(&alpha).all(|(pixel, expected)| pixel[3] as u32 == *expected),
        "the alpha channel becomes the soft mask"
    );
}

#[test]
#[ignore = "writes every JPX variant decoded by the engine as PNM files to PREVIEW_DIR"]
fn write_jpx_decodes() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/jpx");
    let output = std::env::var("PREVIEW_DIR").unwrap();
    for name in jpx_names() {
        let stem = name.rsplit_once('.').unwrap().0;
        let reference = fs::read(directory.join(format!("{stem}.reference.ppm")))
            .or_else(|_| fs::read(directory.join(format!("{stem}.reference.pgm"))))
            .unwrap();
        let (width, height, channels, _, _) = read_pnm(&reference);
        let (_, _, rgba) = decode(jpx_document(&fs::read(directory.join(&name)).unwrap(), width, height));
        let mut bytes = format!("{}\n{width} {height}\n255\n", if channels == 3 { "P6" } else { "P5" }).into_bytes();
        for pixel in rgba.chunks(4) {
            bytes.extend(&pixel[..channels]);
        }
        fs::write(format!("{output}/{stem}.ours.{}", if channels == 3 { "ppm" } else { "pgm" }), bytes).unwrap();
    }
}

#[test]
#[ignore = "writes a page of JPX images to PREVIEW_DIR for comparing with other viewers"]
fn write_jpx_page() {
    let output = std::env::var("PREVIEW_DIR").unwrap();
    let bytes = support::fixtures::jpx_features();
    fs::write(format!("{output}/jpx-page.pdf"), &bytes).unwrap();
    let document = Document::open(bytes, "").unwrap();
    let library = support::rendering::library();
    let page = document.view(&library).pages().unwrap()[0].id;
    let raster = document.view(&library).render(page, 2.0, None).unwrap();
    fs::write(format!("{output}/jpx-page.png"), support::rendering::to_png(&raster)).unwrap();
}
