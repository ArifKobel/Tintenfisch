use crate::support;

use support::fixtures;
use support::rendering::library;
use tintenfisch::document::Document;
use tintenfisch::error::ErrorKind;
use tintenfisch::fonts::FontLibrary;
use tintenfisch::forms::{Field, FieldKind};
use tintenfisch::geometry::Rect;
use tintenfisch::image::DecodedImage;
use tintenfisch::pages::PageId;
use tintenfisch::render::DisplaySpace;
use tintenfisch::signing::SignatureMark;
use tintenfisch::syntax::{ObjId, Object};

const BLUE: [f64; 3] = [0.1, 0.2, 0.7];

struct Desk {
    document: Document,
    library: FontLibrary,
    page: PageId,
}

impl Desk {
    fn open(bytes: Vec<u8>) -> Self {
        let library = library();
        let document = Document::open(bytes, "").unwrap();
        let page = document.view(&library).pages().unwrap()[0].id;
        Self { document, library, page }
    }

    fn sign(&mut self, rect: Rect, mark: &SignatureMark) -> ObjId {
        self.document.add_signature(self.page, rect, "Signature", mark).unwrap()
    }

    fn field(&self, id: ObjId) -> Field {
        self.document.fields().unwrap().into_iter().find(|field| field.id == id).unwrap()
    }

    fn render(&self) -> (u32, Vec<u8>) {
        let raster = self.document.view(&self.library).render(self.page, 1.0, None).unwrap();
        (raster.width, raster.rgba)
    }

    fn widget_box(&self, id: ObjId) -> Rect {
        let page = self.document.view(&self.library).pages().unwrap()[0].clone();
        DisplaySpace::of(&page).unwrap().rect_to_display(self.field(id).widgets[0].rect)
    }
}

fn changed_area((width, before): &(u32, Vec<u8>), (_, after): &(u32, Vec<u8>)) -> Option<Rect> {
    let mut area: Option<Rect> = None;
    for (index, (a, b)) in before.chunks(4).zip(after.chunks(4)).enumerate() {
        if a != b {
            let (x, y) = ((index as u32 % width) as f64, (index as u32 / width) as f64);
            area = Some(match area {
                Some(area) => Rect::new(area.x0.min(x), area.y0.min(y), area.x1.max(x + 1.0), area.y1.max(y + 1.0)),
                None => Rect::new(x, y, x + 1.0, y + 1.0),
            });
        }
    }
    area
}

fn scribble() -> SignatureMark {
    SignatureMark::Ink {
        strokes: vec![vec![(10.0, 30.0), (40.0, 10.0), (70.0, 30.0), (100.0, 10.0)], vec![(110.0, 20.0)]],
        width: 2.0,
        color: BLUE,
    }
}

fn picture() -> SignatureMark {
    let (width, height) = (40, 10);
    let rgba = (0..width * height)
        .flat_map(|index| if index % width < width / 2 { [200, 30, 30, 255] } else { [0, 0, 0, 0] })
        .collect();
    SignatureMark::Image(DecodedImage { width, height, rgba })
}

#[test]
fn an_ink_signature_becomes_an_unsigned_signature_field_that_shows_the_strokes() {
    let original = fixtures::editing_features();
    let mut desk = Desk::open(original.clone());
    let before = desk.render();
    let id = desk.sign(Rect::new(200.0, 250.0, 380.0, 330.0), &scribble());
    let field = desk.field(id);
    assert_eq!((field.name.as_str(), &field.kind), ("Signature", &FieldKind::Signature));
    assert!(field.widgets[0].dict.get("V").is_none(), "placing a signature does not sign the document");
    let widget = desk.widget_box(id);
    assert!((widget.width() - 180.0).abs() < 1e-6, "the strokes fill the width: {widget:?}");
    assert!((widget.height() - 180.0 * 22.0 / 102.0).abs() < 1e-6, "and keep their proportions: {widget:?}");
    assert!((widget.y0 + widget.height() / 2.0 - 290.0).abs() < 1e-6, "centred in the rectangle drawn");
    let changed = changed_area(&before, &desk.render()).expect("the signature shows");
    assert!(
        changed.x0 >= widget.x0 - 1.0
            && changed.x1 <= widget.x1 + 1.0
            && changed.y0 >= widget.y0 - 1.0
            && changed.y1 <= widget.y1 + 1.0,
        "{changed:?} lies within {widget:?}"
    );
    assert!(changed.width() > 170.0 && changed.height() > 30.0, "{changed:?} spans the strokes");
    let reopened = Desk::open(desk.document.save(false).unwrap());
    assert_eq!(reopened.render(), desk.render());
    assert!(reopened.field(id).widgets[0].dict.get("AP").is_some());
    desk.document.undo().unwrap();
    assert!(desk.document.save(false).unwrap() == original);
}

#[test]
fn a_picture_keeps_its_transparency_and_names_stay_unique() {
    let mut desk = Desk::open(fixtures::editing_features());
    let before = desk.render();
    let first = desk.sign(Rect::new(200.0, 250.0, 280.0, 290.0), &picture());
    let second = desk.sign(Rect::new(200.0, 300.0, 280.0, 340.0), &scribble());
    assert_eq!((desk.field(first).name, desk.field(second).name), ("Signature".to_string(), "Signature2".to_string()));
    let widget = desk.widget_box(first);
    assert!((widget.width() - 80.0).abs() < 1e-6 && (widget.height() - 20.0).abs() < 1e-6, "{widget:?}");
    let (width, after) = desk.render();
    let pixel = |x: f64, y: f64| {
        let index = (y as usize * width as usize + x as usize) * 4;
        (after[index..index + 4].to_vec(), before.1[index..index + 4].to_vec())
    };
    let (painted, _) = pixel(widget.x0 + 20.0, widget.y0 + 10.0);
    assert_eq!(painted, [200, 30, 30, 255], "the opaque half shows the picture");
    let (clear, underneath) = pixel(widget.x0 + 60.0, widget.y0 + 10.0);
    assert_eq!(clear, underneath, "the transparent half shows the page");
    let store = desk.document.store();
    let appearance = desk.field(first).widgets[0]
        .dict
        .get("AP")
        .and_then(Object::as_dict)
        .and_then(|ap| ap.get("N"))
        .and_then(Object::as_reference)
        .unwrap();
    let appearance = store.object(appearance).unwrap();
    let resources = appearance.as_stream().unwrap().dict.get("Resources").and_then(Object::as_dict).cloned().unwrap();
    let image = resources
        .get("XObject")
        .and_then(Object::as_dict)
        .and_then(|xobjects| xobjects.get("Signature"))
        .and_then(Object::as_reference)
        .unwrap();
    assert!(store.object(image).unwrap().as_stream().unwrap().dict.get("SMask").is_some());
}

#[test]
fn a_signature_stands_upright_on_a_turned_page() {
    let mut desk = Desk::open(fixtures::nested_page_tree());
    let before = desk.render();
    assert!(before.0 > 600, "the first page is shown turned");
    let stroke = SignatureMark::Ink { strokes: vec![vec![(0.0, 0.0), (100.0, 0.0)]], width: 4.0, color: BLUE };
    desk.sign(Rect::new(100.0, 100.0, 300.0, 300.0), &stroke);
    let changed = changed_area(&before, &desk.render()).unwrap();
    assert!(changed.width() > 150.0 && changed.height() < 12.0, "a horizontal stroke stays horizontal: {changed:?}");
}

#[test]
fn a_moved_signature_keeps_its_appearance_and_a_mark_without_area_is_refused() {
    let mut desk = Desk::open(fixtures::editing_features());
    let id = desk.sign(Rect::new(200.0, 250.0, 380.0, 330.0), &scribble());
    let appearance = desk.field(id).widgets[0].dict.get("AP").cloned();
    desk.document.move_widget(desk.page, id, Rect::new(20.0, 300.0, 110.0, 320.0), &desk.library).unwrap();
    assert_eq!(desk.field(id).widgets[0].dict.get("AP").cloned(), appearance);
    let empty = SignatureMark::Ink { strokes: vec![Vec::new()], width: 2.0, color: BLUE };
    let error = desk.document.add_signature(desk.page, Rect::new(0.0, 0.0, 50.0, 50.0), "", &empty).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
    let tiny = desk.document.add_signature(desk.page, Rect::new(10.0, 10.0, 10.0, 60.0), "", &scribble()).unwrap_err();
    assert_eq!(tiny.kind, ErrorKind::Unsupported);
}

#[test]
#[ignore = "writes signed previews to PREVIEW_DIR for comparing with other viewers"]
fn write_signature_preview() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let mut desk = Desk::open(fixtures::editing_features());
    desk.sign(Rect::new(200.0, 250.0, 380.0, 330.0), &scribble());
    desk.sign(Rect::new(200.0, 340.0, 280.0, 380.0), &picture());
    std::fs::write(format!("{directory}/signed.pdf"), desk.document.save(false).unwrap()).unwrap();
    let raster = desk.document.view(&desk.library).render(desk.page, 2.0, None).unwrap();
    std::fs::write(format!("{directory}/signed.png"), support::rendering::to_png(&raster)).unwrap();
}
