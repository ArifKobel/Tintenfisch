use std::ops::Range;
use std::rc::Rc;

use crate::edit::glyph_encoder::GlyphEncoder;
use crate::edit::line_breaker::Piece;
use crate::edit::page_text::PageText;
use crate::edit::text_style::{ClusterStyle, GlyphFormat};
use crate::error::{Error, Result};
use crate::syntax::ObjId;
use crate::text::{Cluster, LayoutLine, ShowRecord};

const BREAK_TOLERANCE: f64 = 0.01;
const JUSTIFIED_TOLERANCE: f64 = 0.5;
const STRETCHED_SPACE: f64 = 1.1;
const DEFAULT_LEADING: f64 = 1.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClusterRef {
    pub line: usize,
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphItem {
    pub text: String,
    pub cluster: Option<ClusterRef>,
    pub piece: Piece,
    pub style: usize,
    pub format: Option<Rc<GlyphFormat>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphText {
    pub line: usize,
    pub clusters: Vec<String>,
    pub line_starts: Vec<usize>,
    pub styles: Vec<ClusterStyle>,
}

pub struct ParagraphSource<'t> {
    pub lines: Range<usize>,
    pub items: Vec<ParagraphItem>,
    pub line_starts: Vec<usize>,
    pub right: f64,
    pub angle: f64,
    pub space: f64,
    pub justified: bool,
    pub scope: Option<ObjId>,
    layout_lines: &'t [LayoutLine],
}

impl<'t> ParagraphSource<'t> {
    pub fn new(text: &'t PageText, line: usize, encoder: &mut GlyphEncoder) -> Result<Self> {
        let all = text.layout.lines();
        let clicked = text.line(line)?;
        let form_of = |line: &LayoutLine| {
            line.clusters.iter().flat_map(|cluster| &cluster.origins).next().and_then(|origin| origin.form)
        };
        let belongs = |index: usize| {
            let candidate = &all[index];
            candidate.paragraph == clicked.paragraph
                && candidate.angle == clicked.angle
                && form_of(candidate) == form_of(clicked)
        };
        let first = (0..=line).rev().take_while(|&index| belongs(index)).last().unwrap_or(line);
        let end = (line..all.len()).take_while(|&index| belongs(index)).last().map_or(line + 1, |last| last + 1);
        let lines = &all[first..end];
        let scope = Self::scope(lines)?;
        let anchor = Self::first_record(text, lines)?;
        let space = encoder.typed(anchor, anchor, ' ', None)?.advance * anchor.display_scale();
        let right = if lines.len() > 1 {
            lines.iter().map(LayoutLine::right).fold(f64::NEG_INFINITY, f64::max)
        } else {
            f64::INFINITY
        };
        let mut items = Vec::new();
        let mut line_starts = Vec::new();
        let mut style = anchor.ordinal;
        for (offset, layout_line) in lines.iter().enumerate() {
            line_starts.push(items.len());
            for (index, cluster) in layout_line.clusters.iter().enumerate() {
                if let Some(origin) = cluster.origins.first() {
                    style = origin.ordinal;
                }
                let width = cluster_width(&layout_line.clusters, index);
                let piece = if cluster.is_space() { Piece::Space(width) } else { Piece::Glyph(width) };
                let cluster_ref = ClusterRef { line: first + offset, index };
                items.push(ParagraphItem {
                    text: cluster.text.clone(),
                    cluster: Some(cluster_ref),
                    piece,
                    style,
                    format: None,
                });
            }
            if let Some(next) = lines.get(offset + 1) {
                items.extend(separator(layout_line, next, space, right, style));
            }
        }
        let justified = is_justified(lines, right, space);
        Ok(Self {
            lines: first..end,
            items,
            line_starts,
            right,
            angle: clicked.angle,
            space,
            justified,
            scope,
            layout_lines: lines,
        })
    }

    fn scope(lines: &[LayoutLine]) -> Result<Option<ObjId>> {
        let mut forms =
            lines.iter().flat_map(|line| &line.clusters).flat_map(|cluster| &cluster.origins).map(|origin| origin.form);
        let first = forms.next().flatten();
        if forms.any(|form| form != first) {
            return Err(Error::unsupported("the paragraph spans the page and a form XObject"));
        }
        Ok(first)
    }

    fn first_record<'r>(text: &'r PageText, lines: &[LayoutLine]) -> Result<&'r ShowRecord> {
        let origin = lines
            .iter()
            .flat_map(|line| &line.clusters)
            .flat_map(|cluster| &cluster.origins)
            .next()
            .ok_or_else(|| Error::new("the paragraph has no glyphs to edit"))?;
        Ok(text.glyph(origin)?.0)
    }

    pub fn text(&self, text: &PageText) -> Result<ParagraphText> {
        let styles = self
            .items
            .iter()
            .map(|item| text.record(self.scope, item.style).map(ClusterStyle::of))
            .collect::<Result<Vec<_>>>()?;
        Ok(ParagraphText {
            line: self.lines.start,
            clusters: self.items.iter().map(|item| item.text.clone()).collect(),
            line_starts: self.line_starts.clone(),
            styles,
        })
    }

    pub fn line_count(&self) -> usize {
        self.layout_lines.len()
    }

    pub fn line_of(&self, item: usize) -> usize {
        self.line_starts.iter().rposition(|&start| start <= item).unwrap_or(0)
    }

    pub fn layout_line(&self, line: usize) -> Option<&LayoutLine> {
        self.layout_lines.get(line)
    }

    pub fn box_left(&self) -> f64 {
        self.layout_lines.iter().map(LayoutLine::left).fold(f64::INFINITY, f64::min)
    }

    pub fn left(&self, line: usize) -> f64 {
        self.layout_lines[line.min(self.layout_lines.len() - 1)].left()
    }

    pub fn baseline(&self, line: usize) -> f64 {
        let lines = self.layout_lines;
        let last = lines.len() - 1;
        if line <= last {
            return lines[line].baseline;
        }
        let step = if last > 0 {
            (lines[last].baseline - lines[0].baseline) / last as f64
        } else {
            lines[0].size * DEFAULT_LEADING
        };
        lines[last].baseline + step * (line - last) as f64
    }

    pub fn extent(&self) -> (f64, f64) {
        let line = &self.layout_lines[0];
        (line.baseline - line.top, line.bottom - line.baseline)
    }
}

fn cluster_width(clusters: &[Cluster], index: usize) -> f64 {
    let cluster = &clusters[index];
    clusters.get(index + 1).map_or(cluster.right, |next| next.left) - cluster.left
}

fn separator(line: &LayoutLine, next: &LayoutLine, space: f64, right: f64, style: usize) -> Option<ParagraphItem> {
    let spaced =
        line.clusters.last().is_some_and(Cluster::is_space) || next.clusters.first().is_some_and(Cluster::is_space);
    if spaced {
        return None;
    }
    let word: f64 = (0..next.clusters.len())
        .take_while(|&index| !next.clusters[index].is_space())
        .map(|index| cluster_width(&next.clusters, index))
        .sum();
    let fits = line.right() + space + word <= right + BREAK_TOLERANCE;
    Some(if fits {
        ParagraphItem { text: "\n".to_string(), cluster: None, piece: Piece::Break, style, format: None }
    } else {
        ParagraphItem { text: " ".to_string(), cluster: None, piece: Piece::Space(space), style, format: None }
    })
}

fn is_justified(lines: &[LayoutLine], right: f64, space: f64) -> bool {
    let wrapped: Vec<&LayoutLine> = lines
        .windows(2)
        .filter(|pair| separator(&pair[0], &pair[1], space, right, 0).is_none_or(|item| item.piece != Piece::Break))
        .map(|pair| &pair[0])
        .collect();
    if wrapped.is_empty() || wrapped.iter().any(|line| line.right() < right - JUSTIFIED_TOLERANCE) {
        return false;
    }
    let spaces: Vec<f64> = wrapped
        .iter()
        .flat_map(|line| {
            (0..line.clusters.len().saturating_sub(1))
                .filter(|&index| line.clusters[index].is_space())
                .map(|index| cluster_width(&line.clusters, index))
        })
        .collect();
    let stretched = !spaces.is_empty() && spaces.iter().sum::<f64>() / spaces.len() as f64 > space * STRETCHED_SPACE;
    wrapped.len() >= 2 || stretched
}
