use std::collections::{HashMap, HashSet};

use crate::edit::glyph_encoder::GlyphEncoder;
use crate::edit::line_breaker::Piece;
use crate::edit::page_text::PageText;
use crate::edit::paragraph_source::{ClusterRef, ParagraphItem};
use crate::edit::record_edits::RecordEdits;
use crate::edit::show_writer::PlacedGlyph;
use crate::error::{Error, Result};
use crate::syntax::ObjId;
use crate::text::{GlyphOrigin, LayoutLine, ShowRecord, to_line_space};

struct Anchor<'r> {
    record: &'r ShowRecord,
    u0: f64,
}

impl Anchor<'_> {
    fn pen(&self, u: f64) -> f64 {
        (u - self.u0) / self.record.display_scale()
    }
}

pub struct LinePlacer<'t, 'e, 'l> {
    text: &'t PageText,
    encoder: &'e mut GlyphEncoder<'l>,
    space: f64,
    scope: Option<ObjId>,
}

impl<'t, 'e, 'l> LinePlacer<'t, 'e, 'l> {
    pub fn new(text: &'t PageText, encoder: &'e mut GlyphEncoder<'l>, space: f64, scope: Option<ObjId>) -> Self {
        Self { text, encoder, space, scope }
    }

    pub fn place(
        &mut self,
        line: usize,
        items: &[ParagraphItem],
        justify_to: Option<f64>,
        edits: &mut RecordEdits,
    ) -> Result<()> {
        let layout_line = self.text.line(line)?;
        let clusters = &layout_line.clusters;
        if clusters.iter().flat_map(|cluster| &cluster.origins).any(|origin| origin.form != self.scope) {
            return Err(Error::unsupported("the paragraph spans the page and a form XObject"));
        }
        let own = |item: &ParagraphItem, index: usize| item.cluster == Some(ClusterRef { line, index });
        let prefix = items.iter().enumerate().take_while(|(index, item)| own(item, *index)).count();
        let limit = clusters.len().min(items.len()) - prefix;
        let suffix =
            (0..limit).take_while(|&back| own(&items[items.len() - 1 - back], clusters.len() - 1 - back)).count();
        if justify_to.is_none() && prefix == clusters.len() && prefix == items.len() {
            return Ok(());
        }
        let (anchor_origin, after) = clusters[..prefix]
            .iter()
            .rev()
            .find_map(|cluster| cluster.origins.last().map(|origin| (*origin, 1)))
            .or_else(|| {
                clusters[prefix..].iter().find_map(|cluster| cluster.origins.first().map(|origin| (*origin, 0)))
            })
            .ok_or_else(|| Error::new("the line has no glyphs to anchor the edit"))?;
        match justify_to {
            None => {
                let anchor = self.anchor(layout_line, &anchor_origin)?;
                let inserted = self.natural(layout_line, items, (prefix, suffix), &anchor, edits)?;
                edits.insert(anchor_origin.ordinal, anchor_origin.index + after, inserted);
                Ok(())
            }
            Some(right) => self.justified(layout_line, line, items, right, (anchor_origin, after), edits),
        }
    }

    fn anchor(&self, layout_line: &LayoutLine, origin: &GlyphOrigin) -> Result<Anchor<'t>> {
        let record = self.text.record(origin.form, origin.ordinal)?;
        Ok(Anchor { record, u0: to_line_space(layout_line.angle, record.to_display.apply(0.0, 0.0)).0 })
    }

    fn natural(
        &mut self,
        layout_line: &LayoutLine,
        items: &[ParagraphItem],
        (prefix, suffix): (usize, usize),
        anchor: &Anchor,
        edits: &mut RecordEdits,
    ) -> Result<Vec<PlacedGlyph>> {
        let clusters = &layout_line.clusters;
        let insertion_u = if prefix > 0 {
            clusters[prefix - 1].right
        } else {
            clusters.get(prefix).map_or(layout_line.right(), |cluster| cluster.left)
        };
        let start = anchor.pen(insertion_u);
        let mut pen = start;
        let mut inserted = self.glyphs_for(anchor.record, &items[prefix..items.len() - suffix])?;
        for glyph in &mut inserted {
            glyph.x = pen;
            pen += glyph.advance;
        }
        let inserted_width = (pen - start) * anchor.record.display_scale();
        let shift = match clusters.get(clusters.len() - suffix) {
            Some(first_suffix) if suffix > 0 => insertion_u + inserted_width - first_suffix.left,
            _ => 0.0,
        };
        for (index, cluster) in clusters.iter().enumerate() {
            for origin in &cluster.origins {
                if index >= clusters.len() - suffix {
                    if shift.abs() > 1e-9 {
                        let (record, glyph) = self.text.glyph(origin)?;
                        edits.place(*origin, glyph.x + shift / record.display_scale());
                    }
                } else if index >= prefix {
                    edits.delete(*origin);
                }
            }
        }
        Ok(inserted)
    }

    fn justified(
        &mut self,
        layout_line: &LayoutLine,
        line: usize,
        items: &[ParagraphItem],
        right: f64,
        fallback: (GlyphOrigin, usize),
        edits: &mut RecordEdits,
    ) -> Result<()> {
        let targets = positions(items, layout_line.left(), Some(right), self.space);
        let own =
            |item: &ParagraphItem| item.cluster.filter(|cluster| cluster.line == line).map(|cluster| cluster.index);
        let shown: HashMap<usize, usize> =
            items.iter().enumerate().filter_map(|(item, entry)| own(entry).map(|index| (index, item))).collect();
        for (index, cluster) in layout_line.clusters.iter().enumerate() {
            for origin in &cluster.origins {
                match shown.get(&index) {
                    Some(&item) => {
                        let (record, glyph) = self.text.glyph(origin)?;
                        edits.place(*origin, glyph.x + (targets[item] - cluster.left) / record.display_scale());
                    }
                    None => edits.delete(*origin),
                }
            }
        }
        let shown_origin = |item: &ParagraphItem, last: bool| {
            own(item).and_then(|index| {
                let origins = &layout_line.clusters[index].origins;
                if last { origins.last() } else { origins.first() }.copied()
            })
        };
        let mut seen = HashSet::new();
        let mut before: Option<GlyphOrigin> = None;
        let mut run = Vec::new();
        for (index, item) in items.iter().enumerate() {
            if own(item).is_none() {
                run.push(index);
                if index + 1 < items.len() {
                    continue;
                }
            }
            if !run.is_empty() {
                let after_run = items[index..].iter().find_map(|item| shown_origin(item, false));
                let (origin, after) = match (before, after_run) {
                    (Some(origin), _) => (origin, 1),
                    (None, Some(origin)) => (origin, 0),
                    (None, None) => fallback,
                };
                let anchor = self.anchor(layout_line, &origin)?;
                let mut glyphs = Vec::new();
                for &item in &run {
                    let mut pen = anchor.pen(targets[item]);
                    for mut glyph in self.item_glyphs(anchor.record, &items[item], &mut seen)? {
                        glyph.x = pen;
                        pen += glyph.advance;
                        glyphs.push(glyph);
                    }
                }
                edits.insert(origin.ordinal, origin.index + after, glyphs);
                run.clear();
            }
            if let Some(origin) = shown_origin(item, true) {
                before = Some(origin);
            }
        }
        Ok(())
    }

    pub fn glyphs_at(
        &mut self,
        target: &ShowRecord,
        items: &[ParagraphItem],
        starts: &[f64],
        origin: f64,
    ) -> Result<Vec<PlacedGlyph>> {
        let mut seen = HashSet::new();
        let mut glyphs = Vec::new();
        for (item, start) in items.iter().zip(starts) {
            let mut pen = (start - origin) / target.display_scale();
            for mut glyph in self.item_glyphs(target, item, &mut seen)? {
                glyph.x = pen;
                pen += glyph.advance;
                glyphs.push(glyph);
            }
        }
        Ok(glyphs)
    }

    pub fn glyphs_for(&mut self, target: &ShowRecord, items: &[ParagraphItem]) -> Result<Vec<PlacedGlyph>> {
        let mut seen = HashSet::new();
        let mut glyphs = Vec::new();
        for item in items {
            glyphs.extend(self.item_glyphs(target, item, &mut seen)?);
        }
        Ok(glyphs)
    }

    fn item_glyphs(
        &mut self,
        target: &ShowRecord,
        item: &ParagraphItem,
        seen: &mut HashSet<GlyphOrigin>,
    ) -> Result<Vec<PlacedGlyph>> {
        let origins = match item.cluster {
            Some(cluster) => &self.text.line(cluster.line)?.clusters[cluster.index].origins,
            None => &Vec::new(),
        };
        let mut glyphs = Vec::new();
        if origins.is_empty() {
            let style = self.text.record(self.scope, item.style)?;
            for character in item.text.chars().filter(|character| *character != '\n') {
                glyphs.push(self.encoder.typed(style, target, character, item.format.as_deref())?);
            }
            return Ok(glyphs);
        }
        for origin in origins.iter().filter(|origin| seen.insert(**origin)) {
            let (source, glyph) = self.text.glyph(origin)?;
            glyphs.push(GlyphEncoder::moved(target, source, glyph)?);
        }
        Ok(glyphs)
    }
}

pub fn positions(items: &[ParagraphItem], left: f64, justify_to: Option<f64>, space: f64) -> Vec<f64> {
    let width = |item: &ParagraphItem| match item.piece {
        Piece::Glyph(width) => width,
        Piece::Space(_) => space,
        Piece::Break => 0.0,
    };
    let content = items.iter().rposition(|item| matches!(item.piece, Piece::Glyph(_))).map_or(0, |last| last + 1);
    let natural: f64 = items[..content].iter().map(width).sum();
    let spaces = items[..content].iter().filter(|item| matches!(item.piece, Piece::Space(_))).count();
    let extra = match justify_to {
        Some(right) if spaces > 0 && right - left > natural => (right - left - natural) / spaces as f64,
        _ => 0.0,
    };
    let mut u = left;
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let start = u;
            u += width(item) + if index < content && matches!(item.piece, Piece::Space(_)) { extra } else { 0.0 };
            start
        })
        .collect()
}
