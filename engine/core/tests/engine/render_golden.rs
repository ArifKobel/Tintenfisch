use crate::support;

use std::fs;
use std::path::PathBuf;

use support::fixtures;
use support::rendering::{TestRenderer, from_png, to_png};
use tintenfisch::render::{Raster, Tile};

const ANTIALIASING_STEP: u8 = 16;

fn golden_path(name: &str, page: usize) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{}-{page}.png", name.trim_end_matches(".pdf")))
}

fn differing_pixels(expected: &Raster, actual: &Raster) -> usize {
    expected.rgba.chunks(4).zip(actual.rgba.chunks(4)).filter(|(a, b)| a != b).count()
}

#[test]
fn every_fixture_page_matches_its_golden_image() {
    let update = std::env::var("UPDATE_GOLDEN").is_ok();
    let mut failures = Vec::new();
    for fixture in fixtures::all() {
        let renderer = TestRenderer::new(fixture.bytes);
        for page in 0..renderer.page_count() {
            let actual = renderer.render(page, 1.0, None);
            let path = golden_path(&fixture.name, page);
            if update {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, to_png(&actual)).unwrap();
                continue;
            }
            let Ok(bytes) = fs::read(&path) else {
                failures.push(format!("{}: missing golden {}", fixture.name, path.display()));
                continue;
            };
            let expected = from_png(&bytes);
            if (expected.width, expected.height) != (actual.width, actual.height) {
                failures.push(format!("{} page {page}: size changed", fixture.name));
            } else if differing_pixels(&expected, &actual) > 0 {
                failures.push(format!(
                    "{} page {page}: {} pixels differ",
                    fixture.name,
                    differing_pixels(&expected, &actual)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}\nregenerate deliberately with UPDATE_GOLDEN=1");
}

#[test]
fn four_tiles_reassemble_the_whole_page_without_seams() {
    for fixture in [fixtures::committed().remove(0).bytes, fixtures::render_features()] {
        let renderer = TestRenderer::new(fixture);
        let whole = renderer.render(0, 1.5, None);
        let (split_x, split_y) = (whole.width / 2 + 7, whole.height / 2 - 5);
        let tiles = [
            Tile { x: 0, y: 0, width: split_x, height: split_y },
            Tile { x: split_x, y: 0, width: whole.width - split_x, height: split_y },
            Tile { x: 0, y: split_y, width: split_x, height: whole.height - split_y },
            Tile { x: split_x, y: split_y, width: whole.width - split_x, height: whole.height - split_y },
        ];
        let mut differences = 0;
        for tile in tiles {
            let rendered = renderer.render(0, 1.5, Some(tile));
            assert_eq!((rendered.width, rendered.height), (tile.width, tile.height));
            for y in 0..tile.height {
                for x in 0..tile.width {
                    let (part, full) = (rendered.pixel(x, y), whole.pixel(tile.x + x, tile.y + y));
                    let delta = part.iter().zip(full).map(|(a, b)| a.abs_diff(b)).max().unwrap();
                    assert!(delta <= ANTIALIASING_STEP, "seam at {},{}: {part:?} vs {full:?}", tile.x + x, tile.y + y);
                    differences += usize::from(delta > 0);
                }
            }
        }
        assert!(differences * 1000 < (whole.width * whole.height) as usize, "{differences} pixels differ");
    }
}

#[test]
fn a_rotated_page_equals_the_upright_page_turned_clockwise() {
    let renderer = TestRenderer::new(fixtures::render_features());
    let upright = renderer.render(2, 4.0, None);
    let turned = renderer.render(3, 4.0, None);
    assert_eq!((turned.width, turned.height), (upright.height, upright.width));
    for y in 0..upright.height {
        for x in 0..upright.width {
            assert_eq!(turned.pixel(upright.height - 1 - y, x), upright.pixel(x, y), "pixel {x},{y}");
        }
    }
}

#[test]
fn hidden_no_view_and_popup_annotations_stay_invisible() {
    let renderer = TestRenderer::new(fixtures::annotation_features());
    let page = renderer.render(0, 1.0, None);
    let background = [230, 230, 230, 255];
    assert_eq!(page.pixel(240, 150), background, "hidden square");
    assert_eq!(page.pixel(240, 60), background, "no view and popup");
    assert_eq!(page.pixel(165, 165), [0, 128, 0, 255], "checkbox in its /AS state");
    assert_eq!(page.pixel(70, 178), [255, 0, 0, 255], "square border");
}
