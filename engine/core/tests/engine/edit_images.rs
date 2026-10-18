use crate::support;

use std::fs;
use std::path::Path;

use support::fixtures;
use support::rendering::library;
use tintenfisch::document::Document;
use tintenfisch::fonts::FontLibrary;
use tintenfisch::page_images::ImageFrame;
use tintenfisch::pages::PageId;
use tintenfisch::syntax::Object;

struct Desk {
    document: Document,
    library: FontLibrary,
    pages: Vec<PageId>,
}

impl Desk {
    fn open(bytes: Vec<u8>) -> Self {
        let library = library();
        let document = Document::open(bytes, "").unwrap();
        let pages = document.view(&library).pages().unwrap().iter().map(|page| page.id).collect();
        Self { document, library, pages }
    }

    fn images(&self, page: usize) -> Vec<ImageFrame> {
        self.document.images(self.pages[page]).unwrap()
    }

    fn render(&self, page: usize) -> Raster {
        let raster = self.document.view(&self.library).render(self.pages[page], 1.0, None).unwrap();
        Raster { width: raster.width as usize, rgba: raster.rgba }
    }

    fn content(&self) -> Vec<u8> {
        let store = self.document.store();
        let page = self.document.view(&self.library).page(self.pages[0]).unwrap();
        page.contents
            .iter()
            .flat_map(|id| store.decode_stream(store.object(*id).unwrap().as_stream().unwrap()).unwrap().data)
            .collect()
    }
}

struct Raster {
    width: usize,
    rgba: Vec<u8>,
}

impl Raster {
    fn pixel(&self, (x, y): (f64, f64)) -> [u8; 3] {
        let at = (y as usize * self.width + x as usize) * 4;
        [self.rgba[at], self.rgba[at + 1], self.rgba[at + 2]]
    }

    fn changes_outside(&self, other: &Raster, frames: &[ImageFrame]) -> usize {
        let height = self.rgba.len() / 4 / self.width;
        let near = |x: usize, y: usize| {
            frames.iter().any(|frame| {
                let (x, y) = (x as f64, y as f64);
                x > frame.x - 2.0
                    && x < frame.x + frame.width + 2.0
                    && y > frame.y - 2.0
                    && y < frame.y + frame.height + 2.0
            })
        };
        (0..height)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .filter(|&(x, y)| !near(x, y))
            .filter(|&(x, y)| {
                let at = (y * self.width + x) * 4;
                self.rgba[at..at + 4] != other.rgba[at..at + 4]
            })
            .count()
    }
}

fn frame(x: f64, y: f64, width: f64, height: f64) -> ImageFrame {
    ImageFrame { x, y, width, height, angle: 0.0 }
}

fn assert_frame(found: &ImageFrame, expected: &ImageFrame) {
    let close = [
        (found.x, expected.x),
        (found.y, expected.y),
        (found.width, expected.width),
        (found.height, expected.height),
        (found.angle, expected.angle),
    ]
    .iter()
    .all(|(found, expected)| (found - expected).abs() < 1e-3);
    assert!(close, "{found:?} is not {expected:?}");
}

fn picture(path: &str) -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join(path)).unwrap()
}

fn near(found: [u8; 3], expected: [u8; 3], tolerance: u8) -> bool {
    found.iter().zip(expected).all(|(found, expected)| found.abs_diff(expected) <= tolerance)
}

#[test]
fn lists_every_image_where_it_shows_through_forms_inline_and_turned_pages() {
    let desk = Desk::open(fixtures::image_placement_features());
    let images = desk.images(0);
    assert_eq!(images.len(), 4);
    assert_frame(&images[0], &frame(40.0, 100.0, 80.0, 60.0));
    assert_frame(&images[1], &frame(200.0, 200.0, 50.0, 50.0));
    assert_frame(&images[2], &frame(300.0, 300.0, 40.0, 40.0));
    assert_frame(&images[3], &frame(120.0, 330.0, 60.0, 30.0));
    let turned = desk.images(1);
    assert_eq!(turned.len(), 1);
    assert_frame(&turned[0], &frame(150.0, 200.0, 50.0, 50.0));
}

#[test]
fn moving_an_image_changes_one_operation_and_nothing_else_on_the_page() {
    let original = fixtures::image_placement_features();
    let mut desk = Desk::open(original.clone());
    let before = desk.render(0);
    let content = desk.content();
    let moved = frame(60.0, 120.0, 160.0, 120.0);
    desk.document.move_image(desk.pages[0], 0, &moved).unwrap();
    assert_frame(&desk.images(0)[0], &moved);
    let after = desk.render(0);
    assert_eq!(after.changes_outside(&before, &[frame(40.0, 100.0, 80.0, 60.0), moved]), 0);
    assert!(near(after.pixel((140.0, 180.0)), before.pixel((80.0, 130.0)), 12), "the photo shows in its new place");
    assert_eq!(after.pixel((50.0, 105.0)), [255, 255, 255], "the old place is empty");
    let edited = desk.content();
    let matrix = b"80 0 0 60 40 240 cm";
    let at = content.windows(matrix.len()).position(|window| window == matrix).unwrap();
    assert!(edited.starts_with(&content[..at]), "the bytes before the matrix stay");
    assert!(edited.ends_with(&content[at + matrix.len()..]), "the bytes after the matrix stay");
    let reopened = Desk::open(desk.document.save(false).unwrap());
    assert_frame(&reopened.images(0)[0], &moved);
    desk.document.undo();
    assert_eq!(desk.document.save(false).unwrap(), original);
}

#[test]
fn an_image_without_its_own_save_and_restore_gets_one_and_keeps_it_on_the_next_move() {
    let mut desk = Desk::open(fixtures::image_placement_features());
    let before = desk.render(0);
    let page = desk.pages[0];
    desk.document.move_image(page, 3, &frame(200.0, 300.0, 90.0, 45.0)).unwrap();
    let once = desk.content();
    desk.document.move_image(page, 3, &frame(210.0, 310.0, 120.0, 60.0)).unwrap();
    let twice = desk.content();
    let saves = |content: &[u8]| content.windows(2).filter(|pair| pair == b"q\n").count();
    assert_eq!(saves(&once), saves(&twice), "the second move changes the first one's matrix");
    assert_frame(&desk.images(0)[3], &frame(210.0, 310.0, 120.0, 60.0));
    let after = desk.render(0);
    assert!(near(after.pixel((150.0, 316.0)), [0, 255, 0], 1), "the rectangle drawn after the image stays");
    assert_eq!(after.changes_outside(&before, &[frame(120.0, 330.0, 60.0, 30.0), frame(210.0, 310.0, 120.0, 60.0)]), 0);
}

#[test]
fn inline_images_and_images_in_shared_forms_move_on_this_page_only() {
    let mut desk = Desk::open(fixtures::image_placement_features());
    let turned_before = desk.render(1);
    let page = desk.pages[0];
    desk.document.move_image(page, 2, &frame(320.0, 320.0, 60.0, 60.0)).unwrap();
    desk.document.move_image(page, 1, &frame(150.0, 180.0, 30.0, 30.0)).unwrap();
    let images = desk.images(0);
    assert_frame(&images[2], &frame(320.0, 320.0, 60.0, 60.0));
    assert_frame(&images[1], &frame(150.0, 180.0, 30.0, 30.0));
    assert_eq!(desk.render(1).changes_outside(&turned_before, &[]), 0, "the other page's form is untouched");
    let reopened = Desk::open(desk.document.save(false).unwrap());
    assert_frame(&reopened.images(0)[1], &frame(150.0, 180.0, 30.0, 30.0));
    assert_eq!(reopened.images(1).len(), 1);
}

#[test]
fn deleting_an_image_leaves_the_rest_as_it_was() {
    let mut desk = Desk::open(fixtures::image_placement_features());
    let before = desk.render(0);
    desk.document.delete_image(desk.pages[0], 0).unwrap();
    assert_eq!(desk.images(0).len(), 3);
    let after = desk.render(0);
    assert_eq!(after.pixel((80.0, 130.0)), [255, 255, 255]);
    assert_eq!(after.changes_outside(&before, &[frame(40.0, 100.0, 80.0, 60.0)]), 0);
    desk.document.undo();
    assert_eq!(desk.images(0).len(), 4);
}

#[test]
fn a_replacement_fits_into_the_old_box_with_its_own_proportions() {
    let mut desk = Desk::open(fixtures::image_placement_features());
    let before = desk.render(0);
    desk.document.replace_image(desk.pages[0], 0, &picture("png/rgb8.png")).unwrap();
    let height = 80.0 * 9.0 / 13.0;
    let fitted = frame(40.0, 100.0 + (60.0 - height) / 2.0, 80.0, height);
    assert_frame(&desk.images(0)[0], &fitted);
    let after = desk.render(0);
    assert_eq!(after.changes_outside(&before, &[frame(40.0, 100.0, 80.0, 60.0)]), 0);
    let source = picture("png/rgb8.rgba");
    let (column, row) = (6, 4);
    let at = (row * 13 + column) * 4;
    let expected = [source[at], source[at + 1], source[at + 2]];
    let center = (40.0 + (column as f64 + 0.5) * 80.0 / 13.0, fitted.y + (row as f64 + 0.5) * height / 9.0);
    assert!(near(after.pixel(center), expected, 8), "{:?} is not {expected:?}", after.pixel(center));
    let reopened = Desk::open(desk.document.save(false).unwrap());
    assert_frame(&reopened.images(0)[0], &fitted);
    desk.document.replace_image(desk.pages[0], 3, &picture("png/rgb8.png")).unwrap();
    let width = 30.0 * 13.0 / 9.0;
    assert_frame(&desk.images(0)[3], &frame(120.0 + (60.0 - width) / 2.0, 330.0, width, 30.0));
}

#[test]
fn an_inserted_jpeg_keeps_its_bytes_and_turns_as_its_exif_orientation_says() {
    let mut desk = Desk::open(fixtures::image_placement_features());
    let jpeg = picture("jpeg/oriented.jpg");
    let index = desk.document.insert_image(desk.pages[0], &jpeg, (200.0, 200.0)).unwrap();
    assert_eq!(index, 4);
    assert_frame(&desk.images(0)[4], &frame(192.5, 185.0, 15.0, 30.0));
    let store = desk.document.store();
    let kept = store
        .ids()
        .into_iter()
        .any(|id| store.object(id).unwrap().as_stream().is_some_and(|stream| stream.raw.as_ref() == jpeg.as_slice()));
    assert!(kept, "the JPEG is stored as it came");
    let shown = desk.render(0);
    assert!(near(shown.pixel((200.0, 190.0)), [220, 30, 30], 30), "{:?}", shown.pixel((200.0, 190.0)));
    assert!(near(shown.pixel((200.0, 210.0)), [30, 30, 220], 30), "{:?}", shown.pixel((200.0, 210.0)));
    desk.document.move_image(desk.pages[0], 4, &frame(180.0, 170.0, 30.0, 60.0)).unwrap();
    assert_frame(&desk.images(0)[4], &frame(180.0, 170.0, 30.0, 60.0));
    let turned = desk.render(0);
    assert!(
        near(turned.pixel((195.0, 180.0)), [220, 30, 30], 30) && near(turned.pixel((195.0, 220.0)), [30, 30, 220], 30)
    );
    desk.document.replace_image(desk.pages[0], 4, &picture("png/rgb8.png")).unwrap();
    let height = 30.0 * 9.0 / 13.0;
    assert_frame(&desk.images(0)[4], &frame(180.0, 200.0 - height / 2.0, 30.0, height));
    let source = picture("png/rgb8.rgba");
    let corner = desk.render(0).pixel((181.0, 200.0 - height / 2.0 + 1.0));
    assert!(near(corner, [source[0], source[1], source[2]], 8), "the replacement stands upright: {corner:?}");
    desk.document.undo();
    desk.document.undo();
    desk.document.undo();
    assert_eq!(desk.images(0).len(), 4);
}

#[test]
fn inserted_pictures_stand_upright_on_turned_pages_and_cmyk_jpegs_keep_their_color() {
    let mut desk = Desk::open(fixtures::image_placement_features());
    let index = desk.document.insert_image(desk.pages[1], &picture("jpeg/cmyk.jpg"), (100.0, 100.0)).unwrap();
    assert_frame(&desk.images(1)[index], &frame(94.0, 94.0, 12.0, 12.0));
    assert!(
        near(desk.render(1).pixel((100.0, 100.0)), [255, 25, 25], 30),
        "{:?}",
        desk.render(1).pixel((100.0, 100.0))
    );
    let index = desk.document.insert_image(desk.pages[1], &picture("png/rgba8.png"), (200.0, 300.0)).unwrap();
    let placed = desk.images(1)[index];
    assert_frame(&placed, &frame(200.0 - 4.875, 300.0 - 3.375, 9.75, 6.75));
    let store = desk.document.store();
    let masked = store.ids().into_iter().any(|id| {
        store
            .object(id)
            .unwrap()
            .as_stream()
            .is_some_and(|stream| matches!(stream.dict.get("SMask"), Some(Object::Ref(_))))
    });
    assert!(masked, "a PNG with transparency brings a soft mask");
}

#[test]
fn a_large_picture_shrinks_to_fit_the_page() {
    let mut desk = Desk::open(fixtures::image_placement_features());
    let index = desk.document.insert_image(desk.pages[0], &picture("png/wide.png"), (200.0, 200.0)).unwrap();
    assert_frame(&desk.images(0)[index], &frame(40.0, 160.0, 320.0, 80.0));
}

#[test]
#[ignore = "writes edited images to PREVIEW_DIR for comparing with other viewers"]
fn write_image_preview() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let mut desk = Desk::open(fixtures::image_placement_features());
    let page = desk.pages[0];
    desk.document.move_image(page, 0, &frame(60.0, 120.0, 160.0, 120.0)).unwrap();
    desk.document.move_image(page, 1, &frame(250.0, 150.0, 40.0, 40.0)).unwrap();
    desk.document.move_image(page, 3, &frame(200.0, 300.0, 90.0, 45.0)).unwrap();
    desk.document.replace_image(page, 2, &picture("png/rgba8-interlaced.png")).unwrap();
    desk.document.insert_image(page, &picture("jpeg/oriented.jpg"), (330.0, 200.0)).unwrap();
    desk.document.insert_image(desk.pages[1], &picture("jpeg/cmyk.jpg"), (100.0, 100.0)).unwrap();
    std::fs::write(format!("{directory}/edited-images.pdf"), desk.document.save(false).unwrap()).unwrap();
    for index in 0..2 {
        let raster = desk.document.view(&desk.library).render(desk.pages[index], 2.0, None).unwrap();
        std::fs::write(format!("{directory}/edited-images-{index}.png"), support::rendering::to_png(&raster)).unwrap();
    }
}
