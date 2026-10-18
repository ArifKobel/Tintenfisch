use crate::text::grapheme::joins;
use crate::text::{Cluster, LayoutLine, TextGlyph};

const SAME_BASELINE: f64 = 0.35;
const BACKWARD_TOLERANCE: f64 = 0.5;
const COLUMN_GAP: f64 = 2.5;
const WORD_GAP: f64 = 0.2;
const LOOKBACK: usize = 8;
const PARAGRAPH_STEP: f64 = 2.0;

const SHARED_BAND: f64 = 0.5;
const SPACING_FACTOR: f64 = 1.6;

const SAME_ANGLE: f64 = 0.01;

struct LineDraft {
    glyphs: Vec<usize>,
    angle: f64,
    baseline: f64,
    size: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl LineDraft {
    fn accepts(&self, glyph: &TextGlyph) -> bool {
        let size = self.size.max(glyph.size).max(1.0);
        let forward = glyph.start >= self.right - BACKWARD_TOLERANCE * size;
        let gap = glyph.start - self.right;
        let on_baseline = (glyph.baseline - self.baseline).abs() <= SAME_BASELINE * size && gap <= COLUMN_GAP * size;
        let overlap = self.bottom.min(glyph.bounds.y1) - self.top.max(glyph.bounds.y0);
        let smaller = (self.bottom - self.top).min(glyph.bounds.height()).max(0.01);
        let in_band = overlap > SHARED_BAND * smaller && gap <= BACKWARD_TOLERANCE * size;
        (glyph.angle - self.angle).abs() < SAME_ANGLE && forward && (on_baseline || in_band)
    }
}

pub fn group_lines(glyphs: &[TextGlyph]) -> Vec<LayoutLine> {
    let mut drafts: Vec<LineDraft> = Vec::new();
    for (index, glyph) in glyphs.iter().enumerate() {
        let start = drafts.len().saturating_sub(LOOKBACK);
        let target = (start..drafts.len()).rev().find(|&line| drafts[line].accepts(glyph));
        match target {
            Some(line) => {
                let draft = &mut drafts[line];
                draft.glyphs.push(index);
                draft.right = draft.right.max(glyph.end);
                draft.size = draft.size.max(glyph.size);
                draft.top = draft.top.min(glyph.bounds.y0);
                draft.bottom = draft.bottom.max(glyph.bounds.y1);
            }
            None => drafts.push(LineDraft {
                glyphs: vec![index],
                angle: glyph.angle,
                baseline: glyph.baseline,
                size: glyph.size,
                right: glyph.end,
                top: glyph.bounds.y0,
                bottom: glyph.bounds.y1,
            }),
        }
    }
    let mut lines: Vec<LayoutLine> = drafts.into_iter().map(|draft| line_of(glyphs, draft)).collect();
    assign_paragraphs(&mut lines);
    lines
}

fn line_of(glyphs: &[TextGlyph], draft: LineDraft) -> LayoutLine {
    let mut members: Vec<&TextGlyph> = draft.glyphs.iter().map(|&index| &glyphs[index]).collect();
    members.sort_by(|a, b| a.start.total_cmp(&b.start));
    let typical_gap = median_gap(&members);
    let mut clusters: Vec<Cluster> = Vec::new();
    for glyph in &members {
        if let Some(previous) = clusters.last_mut() {
            if joins(&previous.text, &glyph.text) {
                previous.text.push_str(&glyph.text);
                previous.origins.push(glyph.origin);
                previous.right = previous.right.max(glyph.end);
                continue;
            }
            let gap = glyph.start - previous.right;
            let spaced = previous.is_space() || glyph.text.chars().all(char::is_whitespace);
            if gap > (WORD_GAP * glyph.size.max(1.0)).max(SPACING_FACTOR * typical_gap) && !spaced {
                let left = previous.right;
                clusters.push(Cluster { text: " ".to_string(), left, right: glyph.start, origins: Vec::new() });
            }
        }
        clusters.push(Cluster {
            text: glyph.text.clone(),
            left: glyph.start,
            right: glyph.end.max(glyph.start),
            origins: vec![glyph.origin],
        });
    }
    let first = members.first().map(|glyph| glyph.origin).unwrap_or_default();
    LayoutLine {
        id: format!("{}:{}", first.ordinal, first.index),
        paragraph: 0,
        angle: draft.angle,
        top: members.iter().map(|glyph| glyph.bounds.y0).fold(f64::INFINITY, f64::min),
        bottom: members.iter().map(|glyph| glyph.bounds.y1).fold(f64::NEG_INFINITY, f64::max),
        baseline: draft.baseline,
        size: draft.size,
        clusters,
    }
}

fn median_gap(members: &[&TextGlyph]) -> f64 {
    let mut gaps: Vec<f64> =
        members.windows(2).map(|pair| pair[1].start - pair[0].end).filter(|gap| *gap > 0.0).collect();
    if gaps.len() < members.len().saturating_sub(1) / 2 {
        return 0.0;
    }
    gaps.sort_by(f64::total_cmp);
    gaps.get(gaps.len() / 2).copied().unwrap_or(0.0)
}

fn assign_paragraphs(lines: &mut [LayoutLine]) {
    let mut paragraph = 0;
    let mut span = (f64::INFINITY, f64::NEG_INFINITY);
    for index in 0..lines.len() {
        if index > 0 {
            let (previous, current) = (&lines[index - 1], &lines[index]);
            let ratio = current.size / previous.size.max(0.01);
            let step = current.baseline - previous.baseline;
            let overlaps = current.left() < span.1 && current.right() > span.0;
            let continues =
                (0.8..=1.25).contains(&ratio) && step > 0.0 && step <= PARAGRAPH_STEP * previous.size && overlaps;
            if !continues {
                paragraph += 1;
                span = (f64::INFINITY, f64::NEG_INFINITY);
            }
        }
        let line = &mut lines[index];
        line.paragraph = paragraph;
        span = (span.0.min(line.left()), span.1.max(line.right()));
    }
}
