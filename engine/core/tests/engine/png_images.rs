use std::fs;
use std::path::Path;

use tintenfisch::image::decode_png;

const VARIANTS: [&str; 18] = [
    "rgba8",
    "rgb8",
    "gray8",
    "gray-alpha8",
    "gray1",
    "palette2",
    "palette2-trns",
    "palette4",
    "palette4-trns",
    "palette8",
    "palette8-trns",
    "rgb8-trns",
    "gray16",
    "rgba16-interlaced",
    "rgba8-interlaced",
    "gray2-interlaced-trns",
    "rgb8-filters",
    "rgb8-paeth",
];

#[test]
fn every_png_variant_decodes_to_the_pixels_it_was_written_from() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/png");
    for name in VARIANTS {
        let decoded = decode_png(&fs::read(folder.join(format!("{name}.png"))).unwrap()).unwrap();
        let expected = fs::read(folder.join(format!("{name}.rgba"))).unwrap();
        assert_eq!((decoded.width, decoded.height), (13, 9), "{name}");
        assert!(decoded.rgba == expected, "{name} decodes to other pixels");
    }
}

#[test]
fn files_that_are_not_png_or_are_cut_short_are_refused() {
    assert!(decode_png(b"GIF89a").is_err());
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/png");
    let whole = fs::read(folder.join("rgba8.png")).unwrap();
    assert!(decode_png(&whole[..whole.len() / 2]).is_err());
}
