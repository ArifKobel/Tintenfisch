use crate::geometry::Rect;
use crate::text::{Caret, LayoutLine, TextBox, TextLayout, to_line_space};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlannedLine {
    pub start: usize,
    pub length: usize,
    pub left: f64,
    pub baseline: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaretLines {
    pub angle: f64,
    pub ascent: f64,
    pub descent: f64,
    pub lines: Vec<PlannedLine>,
    pub frame: Rect,
}

impl CaretLines {
    pub fn frame(&self) -> TextBox {
        TextBox { rect: self.frame, angle: self.angle }
    }

    pub fn selection(&self, layout: &TextLayout, from: usize, to: usize) -> Vec<TextBox> {
        self.lines
            .iter()
            .filter(|line| line.start < to && line.start + line.length > from)
            .filter_map(|line| {
                let start = self.caret(layout, from.max(line.start))?;
                let end = self.caret(layout, to.min(line.start + line.length))?;
                Some(TextBox { rect: Rect::new(start.x, start.top, end.x, start.bottom), angle: self.angle })
            })
            .collect()
    }

    pub fn offset_at(&self, layout: &TextLayout, x: f64, y: f64) -> Option<usize> {
        let (u, v) = to_line_space(self.angle, (x, y));
        let planned = self.lines.iter().min_by(|a, b| {
            let distance = |line: &PlannedLine| (v - line.baseline).abs();
            distance(a).total_cmp(&distance(b))
        })?;
        let within = match self.shown(layout, planned) {
            Some(line) if planned.length > 0 => line.nearest_offset(u).min(planned.length),
            _ => 0,
        };
        Some(planned.start + within)
    }

    fn shown<'l>(&self, layout: &'l TextLayout, planned: &PlannedLine) -> Option<&'l LayoutLine> {
        layout
            .lines()
            .iter()
            .filter(|line| (line.angle - self.angle).abs() < 1e-6 && !line.clusters.is_empty())
            .filter(|line| (line.baseline - planned.baseline).abs() < 0.3 * line.size.max(1.0))
            .min_by(|a, b| (a.left() - planned.left).abs().total_cmp(&(b.left() - planned.left).abs()))
    }

    pub fn caret(&self, layout: &TextLayout, offset: usize) -> Option<Caret> {
        let planned = self.lines.iter().rev().find(|line| line.start <= offset).or(self.lines.first())?;
        let within = offset.saturating_sub(planned.start).min(planned.length);
        Some(match self.shown(layout, planned) {
            Some(line) if planned.length > 0 => {
                Caret { x: line.caret_x(within), top: line.top, bottom: line.bottom, angle: self.angle }
            }
            _ => Caret {
                x: planned.left,
                top: planned.baseline - self.ascent,
                bottom: planned.baseline + self.descent,
                angle: self.angle,
            },
        })
    }
}
