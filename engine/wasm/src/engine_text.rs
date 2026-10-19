use std::rc::Rc;

use tintenfisch::text::{TextLayout, TextPosition};
use wasm_bindgen::prelude::*;

use crate::engine::{Engine, page_id};
use crate::replies::{MatchReply, PositionReply, SelectionReply, fail, rect_reply, reply};

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(js_name = hitText)]
    pub fn hit_text(&self, page: &str, x: f64, y: f64) -> Result<JsValue, JsError> {
        let hit = self.layout(page)?.hit(x, y);
        reply(&hit.map(PositionReply::from))
    }

    #[wasm_bindgen(js_name = selectText)]
    pub fn select_text(
        &self,
        page: &str,
        anchor: Vec<usize>,
        focus: Vec<usize>,
        unit: &str,
    ) -> Result<JsValue, JsError> {
        let layout = self.layout(page)?;
        let position = |pair: &[usize]| TextPosition {
            line: pair.first().copied().unwrap_or(0),
            offset: pair.get(1).copied().unwrap_or(0),
        };
        let (mut anchor, mut focus) = (position(&anchor), position(&focus));
        match unit {
            "word" => (anchor, focus) = widen(&layout, anchor, focus, TextLayout::word_at),
            "paragraph" => (anchor, focus) = widen(&layout, anchor, focus, TextLayout::paragraph_at),
            _ => {}
        }
        let rects = layout.selection_boxes(anchor, focus).iter().map(rect_reply).collect();
        reply(&SelectionReply {
            anchor: anchor.into(),
            focus: focus.into(),
            rects,
            text: layout.selected_text(anchor, focus),
        })
    }

    #[wasm_bindgen(js_name = textLines)]
    pub fn text_lines(&self, page: &str) -> Result<JsValue, JsError> {
        reply(&self.layout(page)?.line_boxes().iter().map(rect_reply).collect::<Vec<_>>())
    }

    pub fn find(&self, query: &str) -> Result<JsValue, JsError> {
        let view = self.document()?.view(&self.library);
        let mut matches = Vec::new();
        for page in view.pages().map_err(fail)?.iter() {
            let layout = view.text_layout(page.id).map_err(fail)?;
            for (start, end) in layout.find(query) {
                let rects = layout.selection_boxes(start, end).iter().map(rect_reply).collect();
                matches.push(MatchReply { page_id: page.id.to_string(), rects });
            }
        }
        reply(&matches)
    }
}

impl Engine {
    fn layout(&self, page: &str) -> Result<Rc<TextLayout>, JsError> {
        self.document()?.view(&self.library).text_layout(page_id(page)?).map_err(fail)
    }
}

fn widen(
    layout: &TextLayout,
    anchor: TextPosition,
    focus: TextPosition,
    unit: fn(&TextLayout, TextPosition) -> (TextPosition, TextPosition),
) -> (TextPosition, TextPosition) {
    let (anchor_start, anchor_end) = unit(layout, anchor);
    let (focus_start, focus_end) = unit(layout, focus);
    if focus < anchor { (anchor_end, focus_start) } else { (anchor_start, focus_end) }
}
