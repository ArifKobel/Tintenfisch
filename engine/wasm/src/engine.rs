use tintenfisch::document::Document;
use tintenfisch::edit::EditPlan;
use tintenfisch::error::ErrorKind;
use tintenfisch::fonts::{FallbackFace, FontLibrary};
use tintenfisch::pages::PageId;
use tintenfisch::render::Tile;
use wasm_bindgen::prelude::*;

use crate::rendered_tile::RenderedTile;
use crate::replies::{OpenReply, Opened, PageSummaryReply, fail, reply};

#[wasm_bindgen]
#[derive(Default)]
pub struct Engine {
    pub(crate) document: Option<Document>,
    pub(crate) locked: Option<Vec<u8>>,
    pub(crate) library: FontLibrary,
    pub(crate) draft: Option<Draft>,
}

pub struct Draft {
    pub page: PageId,
    pub plan: EditPlan,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        Engine::default()
    }

    #[wasm_bindgen(js_name = fallbackFontFiles)]
    pub fn fallback_font_files() -> Vec<String> {
        FallbackFace::ALL.iter().map(FallbackFace::file_name).collect()
    }

    #[wasm_bindgen(js_name = registerFont)]
    pub fn register_font(&mut self, file_name: &str, bytes: Vec<u8>) -> Result<(), JsError> {
        let face = FallbackFace::ALL
            .into_iter()
            .find(|face| face.file_name() == file_name)
            .ok_or_else(|| JsError::new(&format!("{file_name} is not a fallback font")))?;
        self.library.register(face, bytes);
        Ok(())
    }

    pub fn open(&mut self, bytes: Vec<u8>) -> Result<JsValue, JsError> {
        self.document = None;
        self.locked = None;
        self.draft = None;
        self.try_open(bytes, "", false)
    }

    pub fn unlock(&mut self, password: &str) -> Result<JsValue, JsError> {
        let bytes = self.locked.take().ok_or_else(|| JsError::new("no locked document is waiting"))?;
        self.try_open(bytes, password, true)
    }

    fn try_open(&mut self, bytes: Vec<u8>, password: &str, retried: bool) -> Result<JsValue, JsError> {
        match Document::open(bytes.clone(), password) {
            Ok(document) => {
                let opened = OpenReply::Opened(Opened::describe(&document).map_err(fail)?);
                self.document = Some(document);
                reply(&opened)
            }
            Err(error) if error.kind == ErrorKind::NeedsPassword => {
                self.locked = Some(bytes);
                reply(&OpenReply::Locked { wrong_password: retried })
            }
            Err(error) => Err(fail(error)),
        }
    }

    pub fn pages(&self) -> Result<JsValue, JsError> {
        let summaries = self.document()?.view(&self.library).summaries().map_err(fail)?;
        reply(&summaries.iter().map(PageSummaryReply::from).collect::<Vec<_>>())
    }

    #[wasm_bindgen(js_name = renderTile)]
    pub fn render_tile(
        &mut self,
        page: &str,
        scale: f64,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<RenderedTile, JsError> {
        let id = page_id(page)?;
        let tile = Some(Tile { x, y, width, height });
        let library = &self.library;
        let document = self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))?;
        let raster = match &self.draft {
            Some(draft) if draft.page == id => {
                document.preview(&draft.plan, library, |view| view.render(id, scale, tile)).and_then(|raster| raster)
            }
            _ => document.view(library).render(id, scale, tile),
        }
        .map_err(fail)?;
        Ok(RenderedTile { width: raster.width, height: raster.height, rgba: raster.rgba })
    }

    pub fn save(&self, optimize: bool) -> Result<Vec<u8>, JsError> {
        self.document()?.save(optimize).map_err(fail)
    }

    pub(crate) fn document(&self) -> Result<&Document, JsError> {
        self.document.as_ref().ok_or_else(|| JsError::new("no document is open"))
    }

    pub(crate) fn document_mut(&mut self) -> Result<&mut Document, JsError> {
        self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))
    }
}

pub fn page_id(page: &str) -> Result<PageId, JsError> {
    page.parse().map_err(|_| JsError::new(&format!("{page} is not a page id")))
}
