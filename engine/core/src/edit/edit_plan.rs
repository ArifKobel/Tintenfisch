use crate::edit::caret_lines::CaretLines;
use crate::edit::content_patch::ContentPatch;
use crate::edit::fallback_font::FallbackGlyphs;
use crate::error::{Error, Result};
use crate::store::Transaction;
use crate::syntax::{Name, Object};
use crate::text::{Caret, TextBox, TextLayout};

pub struct EditPlan {
    patch: ContentPatch,
    fallbacks: Vec<(Name, FallbackGlyphs)>,
    lines: CaretLines,
}

impl EditPlan {
    pub(crate) fn new(patch: ContentPatch, fallbacks: Vec<(Name, FallbackGlyphs)>, lines: CaretLines) -> Self {
        Self { patch, fallbacks, lines }
    }

    pub fn is_empty(&self) -> bool {
        self.patch.is_empty()
    }

    pub fn caret(&self, layout: &TextLayout, offset: usize) -> Option<Caret> {
        self.lines.caret(layout, offset)
    }

    pub fn offset_at(&self, layout: &TextLayout, x: f64, y: f64) -> Option<usize> {
        self.lines.offset_at(layout, x, y)
    }

    pub fn selection(&self, layout: &TextLayout, from: usize, to: usize) -> Vec<TextBox> {
        self.lines.selection(layout, from, to)
    }

    pub fn frame(&self) -> TextBox {
        self.lines.frame()
    }

    pub fn apply(&self, transaction: &mut Transaction) -> Result<()> {
        if self.patch.is_empty() {
            return Ok(());
        }
        let page_id = self.patch.page.id.0;
        let mut page =
            transaction.object(page_id)?.as_dict().cloned().ok_or_else(|| Error::new("page is not a dictionary"))?;
        let mut resources = self.patch.page.resources.clone();
        let fonts = self
            .fallbacks
            .iter()
            .map(|(name, glyphs)| Ok::<_, Error>((name.clone(), glyphs.embed(transaction)?)))
            .collect::<Result<Vec<_>>>()?;
        self.patch.apply(transaction, &mut page, &mut resources, &fonts)?;
        if resources != self.patch.page.resources {
            page.insert("Resources", Object::Dict(resources));
        }
        transaction.put(page_id, Object::Dict(page))
    }
}
