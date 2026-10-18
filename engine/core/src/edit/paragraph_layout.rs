use crate::edit::line_breaker::{Piece, break_line};
use crate::edit::line_placer::positions;
use crate::edit::paragraph_source::{ParagraphItem, ParagraphSource};
use crate::text::Cluster;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaidOutLine {
    pub number: usize,
    pub start: usize,
    pub end: usize,
    pub justify_to: Option<f64>,
}

impl LaidOutLine {
    pub fn right(&self, items: &[ParagraphItem], source: &ParagraphSource) -> f64 {
        if let Some(right) = self.justify_to {
            return right;
        }
        let shown = &items[self.start..self.end];
        let left = source.left(self.number);
        let starts = positions(shown, left, None, source.space);
        shown.iter().zip(starts).fold(left, |right, (item, start)| match item.piece {
            Piece::Glyph(width) => right.max(start + width),
            _ => right,
        })
    }
}

pub fn lay_out(source: &ParagraphSource, items: &[ParagraphItem], first: usize, right: f64) -> Vec<LaidOutLine> {
    let pieces: Vec<Piece> = items.iter().map(|item| item.piece).collect();
    let mut lines = Vec::new();
    let mut start = source.line_starts[first];
    loop {
        let number = first + lines.len();
        let line = break_line(&pieces, start, right - source.left(number));
        let last = line.next >= pieces.len() && !line.ended_by_break();
        let justify_to = (source.justified && !last && !line.ended_by_break()).then_some(right);
        let end = trimmed_end(items, line.start, line.end, source, number);
        lines.push(LaidOutLine { number, start: line.start, end, justify_to });
        if line.next >= pieces.len() {
            if line.ended_by_break() {
                lines.push(LaidOutLine {
                    number: number + 1,
                    start: pieces.len(),
                    end: pieces.len(),
                    justify_to: None,
                });
            }
            return lines;
        }
        start = line.next;
    }
}

fn trimmed_end(items: &[ParagraphItem], start: usize, end: usize, source: &ParagraphSource, number: usize) -> usize {
    let own_line = source.lines.start + number;
    let ended_with = |item: &ParagraphItem| {
        let Some(cluster) = item.cluster.filter(|cluster| cluster.line == own_line) else { return false };
        source.layout_line(number).is_some_and(|line| line.clusters[cluster.index..].iter().all(Cluster::is_space))
    };
    let mut end = end;
    while end > start && matches!(items[end - 1].piece, Piece::Space(_)) && !ended_with(&items[end - 1]) {
        end -= 1;
    }
    end
}
