use serde::Serialize;
use tintenfisch::document::{Document, PageSummary};
use tintenfisch::geometry::Rect;
use tintenfisch::page_images::ImageFrame;
use tintenfisch::text::{TextBox, TextPosition};
use wasm_bindgen::{JsError, JsValue};

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum OpenReply {
    Opened(Opened),
    #[serde(rename_all = "camelCase")]
    Locked {
        wrong_password: bool,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    version: String,
    byte_length: usize,
    object_count: usize,
    page_count: usize,
    repaired: bool,
}

impl Opened {
    pub fn describe(document: &Document) -> tintenfisch::error::Result<Self> {
        let file = document.store().source();
        let header = file.header();
        Ok(Self {
            version: format!("{}.{}", header.major, header.minor),
            byte_length: file.bytes().len(),
            object_count: document.store().ids().len(),
            page_count: tintenfisch::pages::PageTree::new(document.store()).pages()?.len(),
            repaired: file.is_repaired(),
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSummaryReply {
    id: String,
    width: f64,
    height: f64,
    rotate: u16,
}

impl From<&PageSummary> for PageSummaryReply {
    fn from(summary: &PageSummary) -> Self {
        Self { id: summary.id.to_string(), width: summary.width, height: summary.height, rotate: summary.rotate }
    }
}

#[derive(Serialize, Clone, Copy)]
pub struct PositionReply {
    line: usize,
    offset: usize,
}

impl From<TextPosition> for PositionReply {
    fn from(position: TextPosition) -> Self {
        Self { line: position.line, offset: position.offset }
    }
}

#[derive(Serialize)]
pub struct RectReply {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl From<Rect> for RectReply {
    fn from(rect: Rect) -> RectReply {
        RectReply { x: rect.x0, y: rect.y0, width: rect.width(), height: rect.height() }
    }
}

#[derive(Serialize)]
pub struct HighlightReply {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    angle: f64,
}

#[derive(Serialize)]
pub struct SelectionReply {
    pub anchor: PositionReply,
    pub focus: PositionReply,
    pub rects: Vec<HighlightReply>,
    pub text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchReply {
    pub page_id: String,
    pub rects: Vec<HighlightReply>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagesChange {
    pub pages: Vec<PageSummaryReply>,
    pub dirty_pages: Vec<String>,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum InsertReply {
    Inserted {
        pages: Vec<PageSummaryReply>,
        inserted: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    Locked {
        wrong_password: bool,
    },
}

#[derive(Serialize)]
pub struct PreviewReply {
    pub caret: Option<HighlightReply>,
    pub frame: HighlightReply,
    pub selection: Vec<HighlightReply>,
}

#[derive(Serialize)]
pub struct ParagraphReply {
    pub line: usize,
    pub clusters: Vec<String>,
    pub start: usize,
    pub styles: Vec<ClusterStyleReply>,
}

#[derive(Serialize)]
pub struct ClusterStyleReply {
    pub resource: Option<String>,
    pub font: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub color: Option<[f64; 3]>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChangeReply {
    pub dirty_pages: Vec<String>,
    pub ui_state: Option<String>,
}

pub fn frame_reply(frame: &ImageFrame) -> HighlightReply {
    HighlightReply { x: frame.x, y: frame.y, width: frame.width, height: frame.height, angle: frame.angle }
}

pub fn rect_reply(text_box: &TextBox) -> HighlightReply {
    let (x, y) = text_box.display_origin();
    HighlightReply { x, y, width: text_box.rect.width(), height: text_box.rect.height(), angle: text_box.angle }
}

pub fn reply(value: &impl Serialize) -> Result<JsValue, JsError> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
    value.serialize(&serializer).map_err(|error| JsError::new(&error.to_string()))
}

pub fn fail(error: tintenfisch::Error) -> JsError {
    JsError::new(&error.to_string())
}
