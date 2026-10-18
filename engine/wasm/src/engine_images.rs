use serde::{Deserialize, Serialize};
use tintenfisch::page_images::ImageFrame;
use wasm_bindgen::prelude::*;

use crate::engine::{Engine, page_id};
use crate::replies::{ChangeReply, HighlightReply, fail, frame_reply, reply};

#[derive(Deserialize)]
struct FrameRequest {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    angle: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InsertedImageReply {
    index: usize,
    dirty_pages: Vec<String>,
}

#[wasm_bindgen]
impl Engine {
    pub fn images(&self, page: &str) -> Result<JsValue, JsError> {
        let frames = self.document()?.images(page_id(page)?).map_err(fail)?;
        reply(&frames.iter().map(frame_reply).collect::<Vec<HighlightReply>>())
    }

    #[wasm_bindgen(js_name = insertImage)]
    pub fn insert_image(&mut self, page: &str, data: &[u8], x: f64, y: f64) -> Result<JsValue, JsError> {
        self.draft = None;
        let index = self.document_mut()?.insert_image(page_id(page)?, data, (x, y)).map_err(fail)?;
        reply(&InsertedImageReply { index, dirty_pages: vec![page.to_string()] })
    }

    #[wasm_bindgen(js_name = moveImage)]
    pub fn move_image(&mut self, page: &str, index: usize, frame: JsValue) -> Result<JsValue, JsError> {
        let FrameRequest { x, y, width, height, angle } = serde_wasm_bindgen::from_value(frame)?;
        self.draft = None;
        let frame = ImageFrame { x, y, width, height, angle };
        self.document_mut()?.move_image(page_id(page)?, index, &frame).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: vec![page.to_string()], ui_state: None })
    }

    #[wasm_bindgen(js_name = replaceImage)]
    pub fn replace_image(&mut self, page: &str, index: usize, data: &[u8]) -> Result<JsValue, JsError> {
        self.draft = None;
        self.document_mut()?.replace_image(page_id(page)?, index, data).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: vec![page.to_string()], ui_state: None })
    }

    #[wasm_bindgen(js_name = deleteImage)]
    pub fn delete_image(&mut self, page: &str, index: usize) -> Result<JsValue, JsError> {
        self.draft = None;
        self.document_mut()?.delete_image(page_id(page)?, index).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: vec![page.to_string()], ui_state: None })
    }
}
