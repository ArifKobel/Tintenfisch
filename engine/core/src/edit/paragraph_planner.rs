use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::rc::Rc;

use crate::content::{ContentWriter, resource};
use crate::edit::caret_lines::{CaretLines, PlannedLine};
use crate::edit::content_patch::{ContentPatch, EditTarget};
use crate::edit::edit_plan::EditPlan;
use crate::edit::glyph_encoder::GlyphEncoder;
use crate::edit::line_breaker::Piece;
use crate::edit::line_placer::{LinePlacer, positions};
use crate::edit::page_text::PageText;
use crate::edit::paragraph_layout::{LaidOutLine, lay_out};
use crate::edit::paragraph_source::{ClusterRef, ParagraphItem, ParagraphSource, ParagraphText};
use crate::edit::record_edits::RecordEdits;
use crate::edit::show_writer::ShowWriter;
use crate::edit::text_style::{GlyphFormat, StyleChange, TextStyle};
use crate::error::{Error, Result};
use crate::fonts::{FontCache, FontLibrary};
use crate::geometry::Rect;
use crate::pages::{Page, PageId, drawn_forms};
use crate::store::ObjectStore;
use crate::syntax::{Dict, Name, Object};
use crate::text::{GlyphOrigin, ShowRecord, split_graphemes, to_display_space, to_line_space};

const MIN_WIDTH: f64 = 4.0;

#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphEdit {
    pub page: PageId,
    pub line: usize,
    pub text: String,
    pub offset: (f64, f64),
    pub width: Option<f64>,
    pub styles: Vec<StyleChange>,
}

impl ParagraphEdit {
    pub fn new(page: PageId, line: usize, text: impl Into<String>) -> Self {
        Self { page, line, text: text.into(), offset: (0.0, 0.0), width: None, styles: Vec::new() }
    }

    pub fn with_styles(self, styles: Vec<StyleChange>) -> Self {
        Self { styles, ..self }
    }

    pub fn moved_by(self, dx: f64, dy: f64) -> Self {
        Self { offset: (dx, dy), ..self }
    }

    pub fn with_width(self, width: f64) -> Self {
        Self { width: Some(width), ..self }
    }

    fn moves(&self) -> bool {
        self.offset.0.abs() > 1e-9 || self.offset.1.abs() > 1e-9
    }
}

pub struct ParagraphPlanner<'a> {
    store: &'a ObjectStore,
    fonts: &'a FontCache,
    library: &'a FontLibrary,
}

impl<'a> ParagraphPlanner<'a> {
    pub fn new(store: &'a ObjectStore, fonts: &'a FontCache, library: &'a FontLibrary) -> Self {
        Self { store, fonts, library }
    }

    pub fn text(&self, page: PageId, line: usize) -> Result<ParagraphText> {
        let text = PageText::extract(self.store, self.fonts, self.library, page)?;
        let mut encoder = GlyphEncoder::new(self.library, HashSet::new());
        ParagraphSource::new(&text, line, &mut encoder)?.text(&text)
    }

    pub fn plan(&self, edit: &ParagraphEdit) -> Result<EditPlan> {
        let text = PageText::extract(self.store, self.fonts, self.library, edit.page)?;
        let mut encoder = GlyphEncoder::new(self.library, font_names_in_use(self.store, &text.page)?);
        let source = ParagraphSource::new(&text, edit.line, &mut encoder)?;
        let old = &source.items;
        let typed = split_graphemes(&edit.text.replace("\r\n", "\n"));
        let prefix = old.iter().zip(&typed).take_while(|(item, grapheme)| item.text == **grapheme).count();
        let limit = old.len().min(typed.len()) - prefix;
        let suffix = old
            .iter()
            .rev()
            .zip(typed.iter().rev())
            .take(limit)
            .take_while(|(item, grapheme)| item.text == **grapheme)
            .count();
        let (kept_prefix, kept_suffix) = (prefix, suffix);
        let styled_start = edit.styles.iter().map(|change| change.start).min().unwrap_or(usize::MAX);
        let prefix = prefix.min(styled_start);
        let retyped = prefix < old.len() || prefix < typed.len();
        let right = edit.width.map_or(source.right, |width| source.box_left() + width.max(MIN_WIDTH));

        let mut items = Vec::with_capacity(typed.len());
        if retyped {
            let style = style_record(&text, &source, prefix)?;
            let document_font = |name: &Name| {
                let font = resource(self.store, &text.page.resources, "Font", &Object::Name(name.clone()))?;
                self.fonts.font(&font, self.store, self.library).ok()
            };
            for (index, grapheme) in typed.iter().enumerate() {
                let kept = if index < kept_prefix {
                    Some(&old[index])
                } else if index >= typed.len() - kept_suffix {
                    Some(&old[index + old.len() - typed.len()])
                } else {
                    None
                };
                let record = match kept {
                    Some(item) => text.record(source.scope, item.style)?,
                    None => style,
                };
                let format = style_at(&edit.styles, index)
                    .map(|change| Rc::new(GlyphFormat::resolve(&change, record.font.as_deref(), document_font)));
                items.push(match (kept, format) {
                    (Some(item), None) => item.clone(),
                    (_, format) => typed_item(&mut encoder, record, grapheme, format)?,
                });
            }
        } else {
            items.clone_from(old);
        }
        let first = if edit.width.is_some() {
            0
        } else if retyped {
            source.line_of(prefix.min(old.len().saturating_sub(1))).saturating_sub(1)
        } else {
            source.line_count()
        };
        let laid = if first < source.line_count() { lay_out(&source, &items, first, right) } else { Vec::new() };

        let mut edits = RecordEdits::default();
        let mut placer = LinePlacer::new(&text, &mut encoder, source.space, source.scope);
        let mut appended = Vec::new();
        for line in &laid {
            let shown = &items[line.start..line.end];
            match source.layout_line(line.number) {
                Some(_) => {
                    let layout_index = source.lines.start + line.number;
                    let restretched = line.justify_to.is_some() && edit.width.is_some();
                    if restretched || !shows_own(shown, layout_index, &source) {
                        placer.place(layout_index, shown, line.justify_to, &mut edits)?;
                    }
                }
                None => appended.push(*line),
            }
        }
        for number in first + laid.len()..source.line_count() {
            placer.place(source.lines.start + number, &[], None, &mut edits)?;
        }
        if edit.moves() {
            for ordinal in paragraph_records(&text, &source)? {
                edits.touch(ordinal);
            }
        }

        let mut replacements = BTreeMap::new();
        for ordinal in edits.touched() {
            let record = text.record(source.scope, ordinal)?;
            let font = record.font_name.as_ref().ok_or_else(|| Error::new("the edited text has no font resource"))?;
            let shift = edit.moves().then(|| text_space_offset(record, edit.offset)).transpose()?;
            replacements.insert(ordinal, ShowWriter::new(record, font).rewrite(record, &edits.glyphs(record), shift));
        }
        if !appended.is_empty() {
            let last = last_record(&text, &source)?;
            let mut bytes = match replacements.remove(&last.ordinal) {
                Some(bytes) => bytes,
                None => text.operation_bytes(source.scope, last.ordinal)?,
            };
            bytes.push(b'\n');
            bytes.extend(append_lines(&mut placer, &source, last, &items, &appended, edit.offset)?);
            replacements.insert(last.ordinal, bytes);
        }
        let lines = caret_lines(&source, &items, first, &laid, right, edit.offset);
        let fallback = encoder.into_fallbacks();
        if source.scope.is_some_and(|form| text.draws_repeatedly(form)) {
            return Err(Error::unsupported("this form XObject is drawn more than once on the page"));
        }
        let target = EditTarget::for_scope(self.store, &text.page, source.scope)?;
        let patch = ContentPatch {
            page: text.page.clone(),
            target,
            replacements: replacements.into_iter().collect(),
            appended: None,
        };
        Ok(EditPlan::new(patch, fallback, lines))
    }
}

fn style_record<'t>(text: &'t PageText, source: &ParagraphSource, prefix: usize) -> Result<&'t ShowRecord> {
    let split = prefix.min(source.items.len());
    let near = source.items[..split]
        .iter()
        .rev()
        .chain(source.items[split..].iter())
        .filter_map(|item| item.cluster)
        .find_map(|cluster| text.layout.lines()[cluster.line].clusters[cluster.index].origins.first().copied())
        .ok_or_else(|| Error::new("the paragraph has no glyphs to edit"))?;
    Ok(text.glyph(&near)?.0)
}

fn shows_own(items: &[ParagraphItem], line: usize, source: &ParagraphSource) -> bool {
    let count = source.layout_line(line - source.lines.start).map_or(0, |layout_line| layout_line.clusters.len());
    items.len() == count
        && items.iter().enumerate().all(|(index, item)| item.cluster == Some(ClusterRef { line, index }))
}

fn style_at(changes: &[StyleChange], index: usize) -> Option<TextStyle> {
    changes.iter().filter(|change| (change.start..change.end).contains(&index)).fold(
        None,
        |merged: Option<TextStyle>, change| {
            let base = merged.unwrap_or_default();
            Some(TextStyle {
                font: change.style.font.clone().or(base.font),
                bold: change.style.bold.or(base.bold),
                italic: change.style.italic.or(base.italic),
                size: change.style.size.or(base.size),
                color: change.style.color.or(base.color),
            })
        },
    )
}

fn typed_item(
    encoder: &mut GlyphEncoder,
    record: &ShowRecord,
    grapheme: &str,
    format: Option<Rc<GlyphFormat>>,
) -> Result<ParagraphItem> {
    let piece = if grapheme == "\n" {
        Piece::Break
    } else {
        let mut width = 0.0;
        for character in grapheme.chars() {
            width += encoder.typed(record, record, character, format.as_deref())?.advance * record.display_scale();
        }
        if grapheme.chars().all(char::is_whitespace) { Piece::Space(width) } else { Piece::Glyph(width) }
    };
    Ok(ParagraphItem { text: grapheme.to_string(), cluster: None, piece, style: record.ordinal, format })
}

fn paragraph_origins(text: &PageText, source: &ParagraphSource) -> HashSet<GlyphOrigin> {
    source
        .lines
        .clone()
        .flat_map(|line| &text.layout.lines()[line].clusters)
        .flat_map(|cluster| cluster.origins.iter().copied())
        .collect()
}

fn paragraph_records(text: &PageText, source: &ParagraphSource) -> Result<BTreeSet<usize>> {
    let origins = paragraph_origins(text, source);
    let ordinals: BTreeSet<usize> = origins.iter().map(|origin| origin.ordinal).collect();
    for &ordinal in &ordinals {
        let record = text.record(source.scope, ordinal)?;
        let exclusive =
            (0..record.glyphs.len()).all(|index| origins.contains(&GlyphOrigin { form: record.form, ordinal, index }));
        if !exclusive {
            return Err(Error::unsupported("the paragraph shares a show operation with other text"));
        }
    }
    Ok(ordinals)
}

fn last_record<'t>(text: &'t PageText, source: &ParagraphSource) -> Result<&'t ShowRecord> {
    let ordinal = paragraph_origins(text, source)
        .iter()
        .map(|origin| origin.ordinal)
        .max()
        .ok_or_else(|| Error::new("the paragraph has no glyphs to edit"))?;
    text.record(source.scope, ordinal)
}

fn text_space_offset(record: &ShowRecord, (dx, dy): (f64, f64)) -> Result<(f64, f64)> {
    let inverse = record.to_display.invert().ok_or_else(|| Error::new("the text is drawn degenerate"))?;
    let (u0, v0) = inverse.apply(0.0, 0.0);
    let (u, v) = inverse.apply(dx, dy);
    Ok((u - u0, v - v0))
}

fn append_lines(
    placer: &mut LinePlacer,
    source: &ParagraphSource,
    record: &ShowRecord,
    items: &[ParagraphItem],
    lines: &[LaidOutLine],
    (dx, dy): (f64, f64),
) -> Result<Vec<u8>> {
    let to_text = record.to_display.invert().ok_or_else(|| Error::new("the text is drawn degenerate"))?;
    let font = record.font_name.clone().ok_or_else(|| Error::new("the edited text has no font resource"))?;
    let writer_state = ShowWriter::new(record, &font);
    let mut writer = ContentWriter::new();
    for line in lines {
        let shown = &items[line.start..line.end];
        let left = source.left(line.number);
        let starts = positions(shown, left, line.justify_to, source.space);
        let glyphs = placer.glyphs_at(record, shown, &starts, left)?;
        let (x, y) = to_display_space(source.angle, (left, source.baseline(line.number)));
        let (u, v) = to_text.apply(x + dx, y + dy);
        writer_state.show_at(&mut writer, record, (u, v - record.state.rise), &glyphs);
    }
    ShowWriter::restore(&mut writer, record);
    Ok(writer.finish())
}

fn caret_lines(
    source: &ParagraphSource,
    items: &[ParagraphItem],
    first: usize,
    laid: &[LaidOutLine],
    right: f64,
    offset: (f64, f64),
) -> CaretLines {
    let (du, dv) = to_line_space(source.angle, offset);
    let mut lines = Vec::new();
    let mut rights = Vec::new();
    for number in 0..first.min(source.line_count()) {
        let layout_line = source.layout_line(number).expect("kept lines exist");
        lines.push(PlannedLine {
            start: source.line_starts[number],
            length: layout_line.clusters.len(),
            left: layout_line.left() + du,
            baseline: layout_line.baseline + dv,
        });
        rights.push(layout_line.right());
    }
    for line in laid {
        lines.push(PlannedLine {
            start: line.start,
            length: line.end - line.start,
            left: source.left(line.number) + du,
            baseline: source.baseline(line.number) + dv,
        });
        rights.push(line.right(items, source));
    }
    let (ascent, descent) = source.extent();
    let box_right = if right.is_finite() { right } else { rights.iter().copied().fold(f64::NEG_INFINITY, f64::max) };
    let top = lines.first().map_or(0.0, |line| line.baseline) - ascent;
    let bottom = lines.last().map_or(0.0, |line| line.baseline) + descent;
    let frame = Rect::new(source.box_left() + du, top, box_right + du, bottom);
    CaretLines { angle: source.angle, ascent, descent, lines, frame }
}

pub(crate) fn font_names_in_use(store: &ObjectStore, page: &Page) -> Result<HashSet<Name>> {
    let mut taken = HashSet::new();
    let mut fonts_of = |resources: &Dict| -> Result<()> {
        if let Some(fonts) = resources.get("Font") {
            taken.extend(
                store
                    .resolve(fonts)?
                    .as_dict()
                    .map(|fonts| fonts.iter().map(|(name, _)| name.clone()).collect::<Vec<_>>())
                    .unwrap_or_default(),
            );
        }
        Ok(())
    };
    fonts_of(&page.resources)?;
    for form in drawn_forms(store, &page.resources) {
        if let Some(own) = store.object(form)?.as_dict().and_then(|dict| dict.get("Resources")).cloned()
            && let Some(own) = store.resolve(&own)?.as_dict()
        {
            fonts_of(own)?;
        }
    }
    Ok(taken)
}
