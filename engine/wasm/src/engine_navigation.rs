use serde::{Deserialize, Serialize};
use tintenfisch::document::Document;
use tintenfisch::navigation::{OutlineItem, Target};
use wasm_bindgen::prelude::*;

use crate::engine::{Engine, page_id};
use crate::engine_forms::RectRequest;
use crate::object_ids::{object_id, object_text};
use crate::replies::{ChangeReply, RectReply, fail, reply};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum TargetRequest {
    #[serde(rename_all = "camelCase")]
    Page {
        page_id: String,
        top: Option<f64>,
    },
    Uri {
        uri: String,
    },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum TargetReply {
    #[serde(rename_all = "camelCase")]
    Page {
        page_id: String,
        top: Option<f64>,
    },
    Uri {
        uri: String,
    },
}

#[derive(Serialize)]
struct BookmarkReply {
    id: String,
    title: String,
    target: Option<TargetReply>,
    open: bool,
    children: Vec<BookmarkReply>,
}

#[derive(Serialize)]
struct LinkReply {
    id: String,
    rect: RectReply,
    target: Option<TargetReply>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AddedLinkReply {
    id: String,
    dirty_pages: Vec<String>,
}

fn target_of(document: &Document, request: TargetRequest) -> Result<Target, JsError> {
    Ok(match request {
        TargetRequest::Page { page_id: page, top } => {
            let page = page_id(&page)?;
            let top = top.map(|top| document.top_to_user(page, top)).transpose().map_err(fail)?;
            Target::Page { page, top }
        }
        TargetRequest::Uri { uri } => Target::Uri(uri),
    })
}

fn target_reply(document: &Document, target: Option<Target>) -> Result<Option<TargetReply>, JsError> {
    Ok(match target {
        Some(Target::Page { page, top }) => {
            let top = top.map(|top| document.top_to_display(page, top)).transpose().map_err(fail)?;
            Some(TargetReply::Page { page_id: page.to_string(), top })
        }
        Some(Target::Uri(uri)) => Some(TargetReply::Uri { uri }),
        None => None,
    })
}

fn bookmark_reply(document: &Document, item: OutlineItem) -> Result<BookmarkReply, JsError> {
    Ok(BookmarkReply {
        id: object_text(item.id),
        title: item.title,
        target: target_reply(document, item.target)?,
        open: item.open,
        children: item.children.into_iter().map(|child| bookmark_reply(document, child)).collect::<Result<_, _>>()?,
    })
}

fn parent_of(parent: Option<String>) -> Result<Option<tintenfisch::syntax::ObjId>, JsError> {
    parent.as_deref().map(object_id).transpose()
}

#[wasm_bindgen]
impl Engine {
    pub fn outline(&self) -> Result<JsValue, JsError> {
        let document = self.document()?;
        let items = document.outline().map_err(fail)?;
        reply(&items.into_iter().map(|item| bookmark_reply(document, item)).collect::<Result<Vec<_>, _>>()?)
    }

    #[wasm_bindgen(js_name = addBookmark)]
    pub fn add_bookmark(
        &mut self,
        parent: Option<String>,
        index: usize,
        title: &str,
        target: JsValue,
    ) -> Result<String, JsError> {
        let request: Option<TargetRequest> = serde_wasm_bindgen::from_value(target)?;
        let parent = parent_of(parent)?;
        let document = self.document_mut()?;
        let target = request.map(|request| target_of(document, request)).transpose()?;
        let id = document.add_bookmark(parent, index, title, target.as_ref()).map_err(fail)?;
        Ok(object_text(id))
    }

    #[wasm_bindgen(js_name = renameBookmark)]
    pub fn rename_bookmark(&mut self, id: &str, title: &str) -> Result<(), JsError> {
        let id = object_id(id)?;
        self.document_mut()?.rename_bookmark(id, title).map_err(fail)
    }

    #[wasm_bindgen(js_name = retargetBookmark)]
    pub fn retarget_bookmark(&mut self, id: &str, target: JsValue) -> Result<(), JsError> {
        let request: TargetRequest = serde_wasm_bindgen::from_value(target)?;
        let id = object_id(id)?;
        let document = self.document_mut()?;
        let target = target_of(document, request)?;
        document.retarget_bookmark(id, &target).map_err(fail)
    }

    #[wasm_bindgen(js_name = moveBookmark)]
    pub fn move_bookmark(&mut self, id: &str, parent: Option<String>, index: usize) -> Result<(), JsError> {
        let (id, parent) = (object_id(id)?, parent_of(parent)?);
        self.document_mut()?.move_bookmark(id, parent, index).map_err(fail)
    }

    #[wasm_bindgen(js_name = deleteBookmark)]
    pub fn delete_bookmark(&mut self, id: &str) -> Result<(), JsError> {
        let id = object_id(id)?;
        self.document_mut()?.delete_bookmark(id).map_err(fail)
    }

    pub fn links(&self, page: &str) -> Result<JsValue, JsError> {
        let document = self.document()?;
        let links = document.links(page_id(page)?).map_err(fail)?;
        let replies = links
            .into_iter()
            .map(|link| {
                Ok(LinkReply {
                    id: object_text(link.id),
                    rect: RectReply::from(link.rect),
                    target: target_reply(document, link.target)?,
                })
            })
            .collect::<Result<Vec<_>, JsError>>()?;
        reply(&replies)
    }

    #[wasm_bindgen(js_name = addLink)]
    pub fn add_link(&mut self, page: &str, rect: JsValue, target: JsValue) -> Result<JsValue, JsError> {
        let rect: RectRequest = serde_wasm_bindgen::from_value(rect)?;
        let request: TargetRequest = serde_wasm_bindgen::from_value(target)?;
        self.draft = None;
        let document = self.document_mut()?;
        let target = target_of(document, request)?;
        let id = document.add_link(page_id(page)?, rect.rect(), &target).map_err(fail)?;
        reply(&AddedLinkReply { id: object_text(id), dirty_pages: vec![page.to_string()] })
    }

    #[wasm_bindgen(js_name = updateLink)]
    pub fn update_link(&mut self, page: &str, id: &str, rect: JsValue, target: JsValue) -> Result<JsValue, JsError> {
        let rect: RectRequest = serde_wasm_bindgen::from_value(rect)?;
        let request: TargetRequest = serde_wasm_bindgen::from_value(target)?;
        let id = object_id(id)?;
        self.draft = None;
        let document = self.document_mut()?;
        let target = target_of(document, request)?;
        document.update_link(page_id(page)?, id, rect.rect(), &target).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: vec![page.to_string()], ui_state: None })
    }
}
