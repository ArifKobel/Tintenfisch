use crate::support;

use support::fixtures::{self, FAX_VARIANTS, JBIG2_COMBINATIONS, JBIG2_SHIFT};
use support::jbig2_huffman_scenarios;
use support::rendering::library;
use tintenfisch::document::Document;
use tintenfisch::image::{ImageDecoder, ImageDescription};
use tintenfisch::syntax::Object;

fn decoded_images(bytes: Vec<u8>) -> Vec<(usize, usize, Vec<u8>)> {
    let library = library();
    let document = Document::open(bytes, "").unwrap();
    let page = document.view(&library).pages().unwrap()[0].clone();
    let store = document.store();
    let xobjects = page.resources.get("XObject").and_then(Object::as_dict).cloned().unwrap();
    let mut names: Vec<_> = xobjects.iter().map(|(name, _)| name.clone()).collect();
    names.sort_by_key(|name| (name.as_bytes().len(), name.as_bytes().to_vec()));
    names
        .iter()
        .map(|name| {
            let object = store.object(xobjects.get_name(name).and_then(Object::as_reference).unwrap()).unwrap();
            let stream = object.as_stream().unwrap();
            let description = ImageDescription::read(&stream.dict, &page.resources, store).unwrap();
            let image = ImageDecoder::new(store, &page.resources).decode(&description, &stream.raw).unwrap();
            (image.width, image.height, image.rgba)
        })
        .collect()
}

fn source_pixels(name: &str) -> (usize, usize, Vec<bool>) {
    let (width, height, bits) = fixtures::scan(name);
    let row_bytes = width.div_ceil(8);
    let black = (0..width * height)
        .map(|index| {
            let (x, y) = (index % width, index / width);
            (bits[y * row_bytes + x / 8] >> (7 - x % 8)) & 1 == 1
        })
        .collect();
    (width, height, black)
}

#[test]
fn every_fax_variant_decodes_to_the_pixels_libtiff_encoded() {
    for name in ["shapes", "wide", "odd", "stripes"] {
        let (width, height, black) = source_pixels(name);
        let decoded = decoded_images(fixtures::scan_features(name));
        assert_eq!(decoded.len(), FAX_VARIANTS.len());
        for ((decoded_width, decoded_height, rgba), (variant, params)) in decoded.into_iter().zip(FAX_VARIANTS) {
            assert_eq!((decoded_width, decoded_height), (width, height), "{name} {variant}");
            let inverted = params.contains("BlackIs1");
            let wrong =
                rgba.chunks(4).zip(&black).filter(|(pixel, black)| (pixel[0] == 0) != (**black != inverted)).count();
            assert_eq!(wrong, 0, "{name} {variant} {params}: {wrong} pixels differ");
        }
    }
}

#[test]
fn every_jbig2_coding_decodes_to_the_pixels_it_was_made_from() {
    for name in ["shapes", "wide", "odd", "stripes"] {
        let (width, height, black) = source_pixels(name);
        let variants = fixtures::jbig2_variants(name);
        let decoded = decoded_images(fixtures::jbig2_features(name));
        assert_eq!(decoded.len(), variants.len());
        for ((decoded_width, decoded_height, rgba), (label, _, margin)) in decoded.into_iter().zip(variants) {
            assert_eq!((decoded_width, decoded_height), (width + 2 * margin, height + 2 * margin), "{name} {label}");
            let source = |x: i64, y: i64| {
                x >= 0
                    && y >= 0
                    && (x as usize) < width
                    && (y as usize) < height
                    && black[y as usize * width + x as usize]
            };
            let combination =
                JBIG2_COMBINATIONS.iter().find(|(name, _)| *name == label).map(|(_, combination)| *combination);
            let wrong = rgba
                .chunks(4)
                .enumerate()
                .filter(|(index, pixel)| {
                    let (x, y) = (
                        (index % decoded_width) as i64 - margin as i64,
                        (index / decoded_width) as i64 - margin as i64,
                    );
                    let (shift_x, shift_y) = (JBIG2_SHIFT.0 as i64, JBIG2_SHIFT.1 as i64);
                    let expected = match combination {
                        None => source(x, y),
                        Some(_) if x < shift_x || y < shift_y => source(x, y),
                        Some(1) => source(x, y) && source(x - shift_x, y - shift_y),
                        Some(2) => source(x, y) != source(x - shift_x, y - shift_y),
                        Some(3) => source(x, y) == source(x - shift_x, y - shift_y),
                        Some(_) => source(x - shift_x, y - shift_y),
                    };
                    (pixel[0] == 0) != expected
                })
                .count();
            assert_eq!(wrong, 0, "{name} {label}: {wrong} pixels differ");
        }
    }
}

#[test]
fn symbol_text_refinement_and_halftone_streams_decode_to_their_layout() {
    let scenarios = fixtures::jbig2_symbol_scenarios();
    let decoded = decoded_images(fixtures::jbig2_symbol_features());
    assert_eq!(decoded.len(), scenarios.len());
    for ((width, height, rgba), scenario) in decoded.into_iter().zip(&scenarios) {
        let expected = &scenario.expected;
        assert_eq!((width, height), (expected.width, expected.height), "{}", scenario.label);
        let wrong = rgba.chunks(4).zip(&expected.black).filter(|(pixel, black)| (pixel[0] == 0) != **black).count();
        assert_eq!(wrong, 0, "{}: {wrong} pixels differ", scenario.label);
    }
}

#[test]
fn huffman_coded_symbols_text_and_mmr_halftones_decode_to_their_layout() {
    let scenarios = jbig2_huffman_scenarios::jbig2_huffman_scenarios();
    let decoded = decoded_images(fixtures::jbig2_scenario_features(&scenarios));
    assert_eq!(decoded.len(), scenarios.len());
    for ((width, height, rgba), scenario) in decoded.into_iter().zip(&scenarios) {
        let expected = &scenario.expected;
        assert_eq!((width, height), (expected.width, expected.height), "{}", scenario.label);
        let wrong = rgba.chunks(4).zip(&expected.black).filter(|(pixel, black)| (pixel[0] == 0) != **black).count();
        assert_eq!(wrong, 0, "{}: {wrong} pixels differ", scenario.label);
    }
}

#[test]
#[ignore = "writes the bitmaps the Huffman scenarios MMR code as PBM files to tests/scans/huffman"]
fn write_huffman_mmr_sources() {
    let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/scans/huffman");
    std::fs::create_dir_all(&directory).unwrap();
    for (name, picture) in jbig2_huffman_scenarios::mmr_sources() {
        std::fs::write(directory.join(format!("{name}.pbm")), pbm(&picture)).unwrap();
    }
}

fn pbm(picture: &support::jbig2_writer::Picture) -> Vec<u8> {
    let row_bytes = picture.width.div_ceil(8);
    let mut bytes = format!("P4\n{} {}\n", picture.width, picture.height).into_bytes();
    let mut rows = vec![0u8; row_bytes * picture.height];
    for (pixel, black) in picture.black.iter().enumerate() {
        if *black {
            let (x, y) = (pixel % picture.width, pixel / picture.width);
            rows[y * row_bytes + x / 8] |= 0x80 >> (x % 8);
        }
    }
    bytes.extend(rows);
    bytes
}

#[test]
#[ignore = "renders the first page of RENDER_PDF to RENDER_PNG at RENDER_SCALE for comparing with other viewers"]
fn render_a_file() {
    let bytes = std::fs::read(std::env::var("RENDER_PDF").unwrap()).unwrap();
    let scale: f64 = std::env::var("RENDER_SCALE").map(|scale| scale.parse().unwrap()).unwrap_or(1.0);
    let library = library();
    let document = Document::open(bytes, "").unwrap();
    let page = document.view(&library).pages().unwrap()[0].id;
    let raster = document.view(&library).render(page, scale, None).unwrap();
    std::fs::write(std::env::var("RENDER_PNG").unwrap(), support::rendering::to_png(&raster)).unwrap();
}

#[test]
#[ignore = "writes the JBIG2 test streams as PDF files to PREVIEW_DIR for decoding with other viewers"]
fn write_jbig2_previews() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    for name in ["shapes", "wide", "odd", "stripes"] {
        std::fs::write(format!("{directory}/jbig2-{name}.pdf"), fixtures::jbig2_features(name)).unwrap();
    }
    std::fs::write(format!("{directory}/jbig2-symbols.pdf"), fixtures::jbig2_symbol_features()).unwrap();
    let huffman = jbig2_huffman_scenarios::jbig2_huffman_scenarios();
    std::fs::write(format!("{directory}/jbig2-huffman.pdf"), fixtures::jbig2_scenario_features(&huffman)).unwrap();
}

#[test]
#[ignore = "writes the CCITT test images as PDF files to PREVIEW_DIR for decoding with other viewers"]
fn write_fax_previews() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    for name in ["shapes", "wide", "odd", "stripes"] {
        std::fs::write(format!("{directory}/fax-{name}.pdf"), fixtures::scan_features(name)).unwrap();
    }
}

#[test]
#[ignore = "writes the expected pages of the JBIG2 symbol scenarios as PBM files to PREVIEW_DIR"]
fn write_jbig2_expectations() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    for (index, scenario) in fixtures::jbig2_symbol_scenarios().iter().enumerate() {
        std::fs::write(format!("{directory}/expected-{index}.pbm"), pbm(&scenario.expected)).unwrap();
    }
    for (index, scenario) in jbig2_huffman_scenarios::jbig2_huffman_scenarios().iter().enumerate() {
        std::fs::write(format!("{directory}/huffman-expected-{index}.pbm"), pbm(&scenario.expected)).unwrap();
    }
}
