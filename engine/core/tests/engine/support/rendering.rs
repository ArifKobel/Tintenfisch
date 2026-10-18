use std::fs;
use std::path::PathBuf;

use tintenfisch::document::Document;
use tintenfisch::fonts::{FallbackFace, FontLibrary};
use tintenfisch::render::{Raster, Tile};
use tiny_skia::{IntSize, Pixmap};

pub fn library() -> FontLibrary {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../public/fonts");
    let mut library = FontLibrary::new();
    for face in FallbackFace::ALL {
        library.register(face, fs::read(directory.join(face.file_name())).unwrap());
    }
    library
}

pub struct TestRenderer {
    document: Document,
    library: FontLibrary,
}

impl TestRenderer {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self::with_password(bytes, "")
    }

    pub fn with_password(bytes: Vec<u8>, password: &str) -> Self {
        Self { document: Document::open(bytes, password).unwrap(), library: library() }
    }

    pub fn page_count(&self) -> usize {
        self.document.view(&self.library).pages().unwrap().len()
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn library(&self) -> &FontLibrary {
        &self.library
    }

    pub fn layout(&self, page: usize) -> std::rc::Rc<tintenfisch::text::TextLayout> {
        let view = self.document.view(&self.library);
        view.text_layout(view.pages().unwrap()[page].id).unwrap()
    }

    pub fn render(&self, page: usize, scale: f64, tile: Option<Tile>) -> Raster {
        let view = self.document.view(&self.library);
        view.render(view.pages().unwrap()[page].id, scale, tile).unwrap()
    }
}

pub fn to_png(raster: &Raster) -> Vec<u8> {
    let mut pixmap =
        Pixmap::from_vec(raster.rgba.clone(), IntSize::from_wh(raster.width, raster.height).unwrap()).unwrap();
    for pixel in pixmap.pixels_mut() {
        let color = pixel.demultiply();
        *pixel = tiny_skia::ColorU8::from_rgba(color.red(), color.green(), color.blue(), color.alpha()).premultiply();
    }
    pixmap.encode_png().unwrap()
}

pub fn from_png(bytes: &[u8]) -> Raster {
    let pixmap = Pixmap::decode_png(bytes).unwrap();
    Raster::from_pixmap(&pixmap)
}
