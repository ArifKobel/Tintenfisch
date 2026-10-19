use crate::content::ContentWriter;
use crate::edit::caret_lines::{CaretLines, PlannedLine};
use crate::edit::content_patch::{ContentPatch, EditTarget};
use crate::edit::edit_plan::EditPlan;
use crate::edit::glyph_encoder::GlyphEncoder;
use crate::edit::page_text::PageText;
use crate::edit::paragraph_planner::font_names_in_use;
use crate::error::{Error, Result};
use crate::fonts::{FallbackFace, FontCache, FontLibrary};
use crate::geometry::{Matrix, Rect};
use crate::pages::{PageId, PageTree};
use crate::render::DisplaySpace;
use crate::store::ObjectStore;
use crate::syntax::{Dict, Name, Object};
use crate::text::{BLOCK_TAG, split_graphemes};

const LEADING: f64 = 1.2;

#[derive(Clone, Debug, PartialEq)]
pub struct NewParagraph {
    pub page: PageId,
    pub at: (f64, f64),
    pub face: FallbackFace,
    pub catalog: Option<String>,
    pub size: f64,
    pub color: [f64; 3],
    pub text: String,
}

impl NewParagraph {
    pub fn plan(&self, store: &ObjectStore, library: &FontLibrary) -> Result<EditPlan> {
        let page = PageTree::new(store)
            .pages()?
            .into_iter()
            .find(|candidate| candidate.id == self.page)
            .ok_or_else(|| Error::new(format!("no page {}", self.page)))?;
        let face = self.face;
        let mut encoder = GlyphEncoder::new(library, font_names_in_use(store, &page)?);
        let (name, glyphs) = encoder.new_text(face, self.catalog.as_deref())?;
        let (ascent, descent) = glyphs.vertical_extent();
        let leading = self.size * LEADING;
        let (left, top) = self.at;
        let graphemes = split_graphemes(&self.text.replace("\r\n", "\n"));
        let mut lines: Vec<(usize, Vec<String>)> = vec![(0, Vec::new())];
        for (index, grapheme) in graphemes.iter().enumerate() {
            if grapheme == "\n" {
                lines.push((index + 1, Vec::new()));
            } else if let Some((_, line)) = lines.last_mut() {
                line.push(grapheme.clone());
            }
        }
        let mut shown_lines = Vec::new();
        let mut widest: f64 = 0.0;
        for (_, line) in &lines {
            let mut codes = Vec::new();
            let mut width = 0.0;
            for character in line.iter().flat_map(|grapheme| grapheme.chars()) {
                let (_, glyphs) = encoder.new_text(face, self.catalog.as_deref())?;
                let glyph = glyphs.glyph(character)?;
                width += glyphs.advance(glyph) * self.size;
                codes.extend(glyph.to_be_bytes());
            }
            widest = widest.max(width);
            shown_lines.push(codes);
        }
        let baseline = top + ascent * self.size;
        let text_matrix =
            Matrix::new(1.0, 0.0, 0.0, -1.0, left, baseline).then(&DisplaySpace::of(&page)?.display_to_user());
        let block = PageText::extract(store, &FontCache::new(), library, self.page)?.next_block();
        let properties: Dict = [(Name::from("Id"), Object::Int(block))].into_iter().collect();
        let mut content = ContentWriter::new();
        let numbers = |values: &[f64]| values.iter().map(|value| Object::Real(*value)).collect::<Vec<_>>();
        content
            .operation("q", &[])
            .operation("BDC", &[Object::Name(Name::from(BLOCK_TAG)), Object::Dict(properties)])
            .operation("rg", &numbers(&self.color))
            .operation("BT", &[])
            .operation("Tf", &[Object::Name(name.clone()), Object::Real(self.size)])
            .operation("Tc", &[Object::Int(0)])
            .operation("Tw", &[Object::Int(0)])
            .operation("Tz", &[Object::Int(100)])
            .operation("Ts", &[Object::Int(0)])
            .operation("Tr", &[Object::Int(0)])
            .operation(
                "Tm",
                &numbers(&[text_matrix.a, text_matrix.b, text_matrix.c, text_matrix.d, text_matrix.e, text_matrix.f]),
            );
        for (index, codes) in shown_lines.iter().enumerate() {
            if index > 0 {
                content.operation("Td", &[Object::Int(0), Object::Real(-leading)]);
            }
            if !codes.is_empty() {
                content.operation("Tj", &[Object::String(codes.clone())]);
            }
        }
        content.operation("ET", &[]).operation("EMC", &[]).operation("Q", &[]);
        let caret_lines = CaretLines {
            angle: 0.0,
            ascent: ascent * self.size,
            descent: descent * self.size,
            lines: lines
                .iter()
                .enumerate()
                .map(|(index, (start, line))| PlannedLine {
                    start: *start,
                    length: line.len(),
                    left,
                    baseline: baseline + index as f64 * leading,
                })
                .collect(),
            frame: Rect::new(left, top, left + widest.max(self.size), top + lines.len() as f64 * leading),
        };
        let shows_something = shown_lines.iter().any(|codes| !codes.is_empty());
        let patch = ContentPatch {
            page,
            target: EditTarget::PageContent,
            replacements: Vec::new(),
            appended: shows_something.then(|| content.finish()),
        };
        let fallbacks = if shows_something { encoder.into_fallbacks() } else { Vec::new() };
        Ok(EditPlan::new(patch, fallbacks, caret_lines))
    }
}
