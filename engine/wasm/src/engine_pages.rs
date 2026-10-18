use tintenfisch::document::Document;
use tintenfisch::error::ErrorKind;
use tintenfisch::pages::PageId;
use wasm_bindgen::prelude::*;

use crate::engine::{Engine, page_id};
use crate::replies::{InsertReply, PageSummaryReply, PagesChange, fail, reply};

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(js_name = rotatePages)]
    pub fn rotate_pages(&mut self, pages: Vec<String>, quarter_turns: i32) -> Result<JsValue, JsError> {
        let ids = page_ids(&pages)?;
        self.organize(|document| document.rotate_pages(&ids, quarter_turns as i64))?;
        self.pages_change(pages)
    }

    #[wasm_bindgen(js_name = deletePages)]
    pub fn delete_pages(&mut self, pages: Vec<String>) -> Result<JsValue, JsError> {
        let ids = page_ids(&pages)?;
        self.organize(|document| document.delete_pages(&ids))?;
        self.pages_change(Vec::new())
    }

    #[wasm_bindgen(js_name = movePages)]
    pub fn move_pages(&mut self, pages: Vec<String>, before: Option<String>) -> Result<JsValue, JsError> {
        let (ids, before) = (page_ids(&pages)?, before.as_deref().map(page_id).transpose()?);
        self.organize(|document| document.move_pages(&ids, before))?;
        self.pages_change(Vec::new())
    }

    #[wasm_bindgen(js_name = insertBlankPage)]
    pub fn insert_blank_page(&mut self, before: Option<String>, width: f64, height: f64) -> Result<JsValue, JsError> {
        let before = before.as_deref().map(page_id).transpose()?;
        self.organize(|document| document.insert_blank_page(before, (width, height)))?;
        self.pages_change(Vec::new())
    }

    #[wasm_bindgen(js_name = insertDocument)]
    pub fn insert_document(
        &mut self,
        bytes: Vec<u8>,
        password: &str,
        before: Option<String>,
    ) -> Result<JsValue, JsError> {
        let before = before.as_deref().map(page_id).transpose()?;
        let other = match Document::open(bytes, password) {
            Ok(other) => other,
            Err(error) if error.kind == ErrorKind::NeedsPassword => {
                return reply(&InsertReply::Locked { wrong_password: !password.is_empty() });
            }
            Err(error) => return Err(fail(error)),
        };
        let inserted = self.organize(|document| document.insert_document(&other, before))?;
        reply(&InsertReply::Inserted {
            pages: self.summaries()?,
            inserted: inserted.iter().map(ToString::to_string).collect(),
        })
    }

    #[wasm_bindgen(js_name = extractPages)]
    pub fn extract_pages(&self, pages: Vec<String>) -> Result<Vec<u8>, JsError> {
        self.document()?.extract_pages(&page_ids(&pages)?).map_err(fail)
    }
}

impl Engine {
    fn organize<T>(
        &mut self,
        operation: impl FnOnce(&mut Document) -> tintenfisch::error::Result<T>,
    ) -> Result<T, JsError> {
        self.draft = None;
        operation(self.document_mut()?).map_err(fail)
    }

    fn summaries(&self) -> Result<Vec<PageSummaryReply>, JsError> {
        let summaries = self.document()?.view(&self.library).summaries().map_err(fail)?;
        Ok(summaries.iter().map(PageSummaryReply::from).collect())
    }

    fn pages_change(&self, dirty_pages: Vec<String>) -> Result<JsValue, JsError> {
        reply(&PagesChange { pages: self.summaries()?, dirty_pages })
    }
}

fn page_ids(pages: &[String]) -> Result<Vec<PageId>, JsError> {
    pages.iter().map(|page| page_id(page)).collect()
}
