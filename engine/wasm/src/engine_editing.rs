use serde::{Deserialize, Serialize};
use tintenfisch::edit::{EditPlan, FontPick, NewParagraph, ParagraphEdit, StyleChange, TextStyle};
use tintenfisch::fonts::{FallbackFace, FallbackFamily};
use tintenfisch::geometry::Rect;
use tintenfisch::pages::PageId;
use tintenfisch::store::Reverted;
use tintenfisch::syntax::Name;
use tintenfisch::text::TextBox;
use wasm_bindgen::prelude::*;

use crate::engine::{Draft, Engine, page_id};
use crate::replies::{ChangeReply, ClusterStyleReply, ParagraphReply, PreviewReply, fail, rect_reply, reply};

#[derive(Deserialize)]
struct DraftRequest {
    text: String,
    caret: usize,
    dx: f64,
    dy: f64,
    width: Option<f64>,
    #[serde(default)]
    styles: Vec<StyleRequest>,
    selection: Option<[usize; 2]>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum FontRequest {
    Document { name: String },
    Bundled { family: String },
    Catalog { key: String },
}

#[derive(Deserialize)]
struct StyleRequest {
    start: usize,
    end: usize,
    font: Option<FontRequest>,
    bold: Option<bool>,
    italic: Option<bool>,
    size: Option<f64>,
    color: Option<[f64; 3]>,
}

impl From<StyleRequest> for StyleChange {
    fn from(request: StyleRequest) -> StyleChange {
        let font = request.font.map(|font| match font {
            FontRequest::Document { name } => FontPick::Document(Name::from(name.as_str())),
            FontRequest::Bundled { family } => FontPick::Bundled(family_of(&family)),
            FontRequest::Catalog { key } => FontPick::Catalog(key),
        });
        let style =
            TextStyle { font, bold: request.bold, italic: request.italic, size: request.size, color: request.color };
        StyleChange { start: request.start, end: request.end, style }
    }
}

#[derive(Deserialize)]
struct NewTextRequest {
    x: f64,
    y: f64,
    text: String,
    caret: usize,
    selection: Option<[usize; 2]>,
    family: String,
    #[serde(default)]
    catalog: Option<String>,
    bold: bool,
    italic: bool,
    size: f64,
    color: [f64; 3],
}

fn family_of(name: &str) -> FallbackFamily {
    match name {
        "serif" => FallbackFamily::Serif,
        "mono" => FallbackFamily::Mono,
        _ => FallbackFamily::Sans,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageFontReply {
    name: String,
    font: Option<String>,
    base_font: String,
    embedded: bool,
    family: &'static str,
    bold: bool,
    italic: bool,
    weight: Option<u16>,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(js_name = paragraphText)]
    pub fn paragraph_text(&self, page: &str, line: usize) -> Result<JsValue, JsError> {
        let text = self.document()?.paragraph_text(page_id(page)?, line, &self.library).map_err(fail)?;
        let start = text.line_starts.get(line - text.line).copied().unwrap_or(0);
        let styles = text
            .styles
            .into_iter()
            .map(|style| ClusterStyleReply {
                resource: style.resource.map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
                font: style.font,
                size: style.size,
                bold: style.bold,
                italic: style.italic,
                color: style.color,
            })
            .collect();
        reply(&ParagraphReply { line: text.line, clusters: text.clusters, start, styles })
    }

    #[wasm_bindgen(js_name = pageFonts)]
    pub fn page_fonts(&self, page: &str) -> Result<JsValue, JsError> {
        let fonts = self.document()?.view(&self.library).fonts(page_id(page)?).map_err(fail)?;
        let replies: Vec<PageFontReply> = fonts
            .into_iter()
            .map(|font| PageFontReply {
                name: String::from_utf8_lossy(font.name.as_bytes()).into_owned(),
                font: font.family,
                base_font: font.base_font,
                embedded: font.embedded,
                family: match font.face.family {
                    FallbackFamily::Sans => "sans",
                    FallbackFamily::Serif => "serif",
                    FallbackFamily::Mono => "mono",
                },
                bold: font.face.bold,
                italic: font.face.italic,
                weight: font.weight,
            })
            .collect();
        reply(&replies)
    }

    #[wasm_bindgen(js_name = hitDraft)]
    pub fn hit_draft(&mut self, page: &str, x: f64, y: f64) -> Result<JsValue, JsError> {
        let page = page_id(page)?;
        let Some(draft) = self.draft.take() else { return reply(&None::<usize>) };
        let library = &self.library;
        let document = self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))?;
        let offset = if draft.page == page {
            document.preview(&draft.plan, library, |view| {
                Ok::<_, tintenfisch::error::Error>(draft.plan.offset_at(&*view.text_layout(page)?, x, y))
            })
        } else {
            Ok(Ok(None))
        };
        self.draft = Some(draft);
        reply(&offset.and_then(|found| found).map_err(fail)?)
    }

    #[wasm_bindgen(js_name = registerFontReplacement)]
    pub fn register_font_replacement(&mut self, key: &str, bytes: Vec<u8>) {
        self.library.register_replacement(key, bytes);
    }

    #[wasm_bindgen(js_name = previewEdit)]
    pub fn preview_edit(&mut self, page: &str, line: usize, draft: JsValue) -> Result<JsValue, JsError> {
        let draft: DraftRequest =
            serde_wasm_bindgen::from_value(draft).map_err(|error| JsError::new(&error.to_string()))?;
        let offset = draft.caret;
        let selection = draft.selection;
        let styles = draft.styles.into_iter().map(StyleChange::from).collect();
        let mut edit =
            ParagraphEdit::new(page_id(page)?, line, draft.text).moved_by(draft.dx, draft.dy).with_styles(styles);
        if let Some(width) = draft.width {
            edit = edit.with_width(width);
        }
        let library = &self.library;
        let document = self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))?;
        let plan = document.plan_edit(&edit, library).map_err(fail)?;
        self.show_draft(edit.page, plan, offset, selection)
    }

    #[wasm_bindgen(js_name = previewNewText)]
    pub fn preview_new_text(&mut self, page: &str, request: JsValue) -> Result<JsValue, JsError> {
        let request: NewTextRequest = serde_wasm_bindgen::from_value(request)?;
        let face = FallbackFace { family: family_of(&request.family), bold: request.bold, italic: request.italic };
        let new = NewParagraph {
            page: page_id(page)?,
            at: (request.x, request.y),
            face,
            catalog: request.catalog,
            size: request.size,
            color: request.color,
            text: request.text,
        };
        let plan = self.document()?.plan_new_text(&new, &self.library).map_err(fail)?;
        self.show_draft(new.page, plan, request.caret, request.selection)
    }

    #[wasm_bindgen(js_name = cancelEdit)]
    pub fn cancel_edit(&mut self) {
        self.draft = None;
    }

    #[wasm_bindgen(js_name = commitEdit)]
    pub fn commit_edit(&mut self, ui_state: &str) -> Result<JsValue, JsError> {
        let Some(draft) = self.draft.take() else { return reply(&ChangeReply::default()) };
        self.document_mut()?.apply(&draft.plan, ui_state).map_err(fail)?;
        reply(&ChangeReply { dirty_pages: vec![draft.page.to_string()], ui_state: None })
    }

    pub fn undo(&mut self) -> Result<JsValue, JsError> {
        self.draft = None;
        let reverted = self.document_mut()?.undo();
        self.change_reply(reverted)
    }

    pub fn redo(&mut self) -> Result<JsValue, JsError> {
        self.draft = None;
        let reverted = self.document_mut()?.redo();
        self.change_reply(reverted)
    }
}

impl Engine {
    fn show_draft(
        &mut self,
        page: PageId,
        plan: EditPlan,
        offset: usize,
        selection: Option<[usize; 2]>,
    ) -> Result<JsValue, JsError> {
        let library = &self.library;
        let document = self.document.as_mut().ok_or_else(|| JsError::new("no document is open"))?;
        let (caret, selected) = document
            .preview(&plan, library, |view| {
                let layout = view.text_layout(page)?;
                let selected =
                    selection.map(|[from, to]| plan.selection(&layout, from.min(to), from.max(to))).unwrap_or_default();
                Ok((plan.caret(&layout, offset), selected))
            })
            .and_then(|found| found)
            .map_err(fail)?;
        let frame = rect_reply(&plan.frame());
        self.draft = Some(Draft { page, plan });
        reply(&PreviewReply {
            caret: caret.map(|caret| {
                rect_reply(&TextBox { rect: Rect::new(caret.x, caret.top, caret.x, caret.bottom), angle: caret.angle })
            }),
            frame,
            selection: selected.iter().map(rect_reply).collect(),
        })
    }

    fn change_reply(&self, reverted: Option<Reverted>) -> Result<JsValue, JsError> {
        let Some(reverted) = reverted else { return reply(&ChangeReply::default()) };
        let pages = self.document()?.pages_touching(&reverted.touched, &self.library).map_err(fail)?;
        reply(&ChangeReply {
            dirty_pages: pages.iter().map(ToString::to_string).collect(),
            ui_state: Some(reverted.ui_state),
        })
    }
}
