use serde::{Deserialize, Serialize};
use tintenfisch::annotations::{AnnotationSummary, Markup, MarkupStyle};
use tintenfisch::text::TextPosition;
use wasm_bindgen::prelude::*;

use crate::engine::{Engine, page_id};
use crate::object_ids::{object_id, object_text};
use crate::replies::{ChangeReply, RectReply, fail, reply};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum MarkupRequest {
    Highlight { anchor: [usize; 2], focus: [usize; 2] },
    Note { at: (f64, f64), text: String },
    Ink { strokes: Vec<Vec<(f64, f64)>> },
    Rectangle { from: (f64, f64), to: (f64, f64) },
    Ellipse { from: (f64, f64), to: (f64, f64) },
    Arrow { from: (f64, f64), to: (f64, f64) },
    Stamp { at: (f64, f64), label: String },
    TextBox { from: (f64, f64), to: (f64, f64), text: String },
    RedactArea { from: (f64, f64), to: (f64, f64) },
    RedactText { anchor: [usize; 2], focus: [usize; 2] },
}

impl From<MarkupRequest> for Markup {
    fn from(request: MarkupRequest) -> Markup {
        let position = |[line, offset]: [usize; 2]| TextPosition { line, offset };
        match request {
            MarkupRequest::Highlight { anchor, focus } => {
                Markup::Highlight { anchor: position(anchor), focus: position(focus) }
            }
            MarkupRequest::Note { at, text } => Markup::Note { at, text },
            MarkupRequest::Ink { strokes } => Markup::Ink { strokes },
            MarkupRequest::Rectangle { from, to } => Markup::Rectangle { from, to },
            MarkupRequest::Ellipse { from, to } => Markup::Ellipse { from, to },
            MarkupRequest::Arrow { from, to } => Markup::Arrow { from, to },
            MarkupRequest::Stamp { at, label } => Markup::Stamp { at, label },
            MarkupRequest::TextBox { from, to, text } => Markup::TextBox { from, to, text },
            MarkupRequest::RedactArea { from, to } => Markup::RedactArea { from, to },
            MarkupRequest::RedactText { anchor, focus } => {
                Markup::RedactText { anchor: position(anchor), focus: position(focus) }
            }
        }
    }
}

#[derive(Deserialize)]
struct StyleRequest {
    color: [f64; 3],
    width: f64,
    author: String,
    date: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CommentReply {
    id: String,
    page_id: String,
    subtype: String,
    contents: String,
    author: String,
    date: String,
    rect: RectReply,
    color: Option<[f64; 3]>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AddedReply {
    id: String,
    dirty_pages: Vec<String>,
}

#[wasm_bindgen]
impl Engine {
    pub fn comments(&self) -> Result<JsValue, JsError> {
        let document = self.document()?;
        let mut comments = Vec::new();
        for page in document.view(&self.library).pages().map_err(fail)?.iter() {
            for summary in document.annotations(page.id).map_err(fail)? {
                comments.push(comment_reply(page.id.to_string(), summary));
            }
        }
        reply(&comments)
    }

    #[wasm_bindgen(js_name = addMarkup)]
    pub fn add_markup(&mut self, page: &str, markup: JsValue, style: JsValue) -> Result<JsValue, JsError> {
        let markup: MarkupRequest =
            serde_wasm_bindgen::from_value(markup).map_err(|error| JsError::new(&error.to_string()))?;
        let style: StyleRequest =
            serde_wasm_bindgen::from_value(style).map_err(|error| JsError::new(&error.to_string()))?;
        let style = MarkupStyle { color: style.color, width: style.width, author: style.author, date: style.date };
        let id = page_id(page)?;
        self.draft = None;
        let library = &self.library;
        let document = self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))?;
        let added = document.add_markup(id, &markup.into(), &style, library).map_err(fail)?;
        reply(&AddedReply { id: object_text(added), dirty_pages: vec![page.to_string()] })
    }

    #[wasm_bindgen(js_name = deleteComment)]
    pub fn delete_comment(&mut self, page: &str, comment: &str) -> Result<JsValue, JsError> {
        let (page_id, comment) = (page_id(page)?, object_id(comment)?);
        self.draft = None;
        self.document_mut()?.delete_annotation(page_id, comment).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: vec![page.to_string()], ui_state: None })
    }

    #[wasm_bindgen(js_name = applyRedactions)]
    pub fn apply_redactions(&mut self) -> Result<JsValue, JsError> {
        self.draft = None;
        let library = &self.library;
        let document = self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))?;
        let pages = document.apply_redactions(library).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: pages.iter().map(ToString::to_string).collect(), ui_state: None })
    }

    #[wasm_bindgen(js_name = setCommentContents)]
    pub fn set_comment_contents(&mut self, comment: &str, contents: &str, date: &str) -> Result<JsValue, JsError> {
        let comment = object_id(comment)?;
        let library = &self.library;
        let document = self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))?;
        let pages = document.set_annotation_contents(comment, contents, date, library).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: pages.iter().map(ToString::to_string).collect(), ui_state: None })
    }
}

fn comment_reply(page_id: String, summary: AnnotationSummary) -> CommentReply {
    let rect = summary.rect;
    CommentReply {
        id: object_text(summary.id),
        page_id,
        subtype: summary.subtype,
        contents: summary.contents,
        author: summary.author,
        date: summary.date,
        rect: RectReply::from(rect),
        color: summary.color,
    }
}
