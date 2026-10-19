use crate::geometry::Rect;
use crate::text::{LayoutLine, TextGlyph, group_lines, to_display_space, to_line_space};

const MARGIN_LINES: f64 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextPosition {
    pub line: usize,
    pub offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    CharLeft,
    CharRight,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    LineUp,
    LineDown,
    DocumentStart,
    DocumentEnd,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caret {
    pub x: f64,
    pub top: f64,
    pub bottom: f64,
    pub angle: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextBox {
    pub rect: Rect,
    pub angle: f64,
}

impl TextBox {
    pub fn display_origin(&self) -> (f64, f64) {
        to_display_space(self.angle, (self.rect.x0, self.rect.y0))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextLayout {
    lines: Vec<LayoutLine>,
}

impl TextLayout {
    pub fn new(glyphs: &[TextGlyph]) -> Self {
        Self { lines: group_lines(glyphs).into_iter().filter(|line| !line.clusters.is_empty()).collect() }
    }

    pub fn lines(&self) -> &[LayoutLine] {
        &self.lines
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn hit(&self, x: f64, y: f64) -> Option<TextPosition> {
        let typical = self.lines.iter().map(LayoutLine::height).fold(0.0, f64::max).max(1.0);
        let margin = MARGIN_LINES * typical;
        let (line, vertical, horizontal) = self
            .lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let (u, v) = to_line_space(line.angle, (x, y));
                (index, line.vertical_distance(v), line.horizontal_distance(u))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)))?;
        if vertical > margin || horizontal > 4.0 * margin {
            return None;
        }
        let target = &self.lines[line];
        let (u, _) = to_line_space(target.angle, (x, y));
        Some(TextPosition { line, offset: target.nearest_offset(u) })
    }

    pub fn caret(&self, position: TextPosition) -> Option<Caret> {
        let line = self.lines.get(position.line)?;
        Some(Caret { x: line.caret_x(position.offset), top: line.top, bottom: line.bottom, angle: line.angle })
    }

    fn ordered(a: TextPosition, b: TextPosition) -> (TextPosition, TextPosition) {
        if a <= b { (a, b) } else { (b, a) }
    }

    pub fn selection_boxes(&self, anchor: TextPosition, focus: TextPosition) -> Vec<TextBox> {
        let (start, end) = Self::ordered(anchor, focus);
        if start == end {
            return Vec::new();
        }
        (start.line..=end.line.min(self.lines.len().saturating_sub(1)))
            .filter_map(|index| {
                let line = &self.lines[index];
                let from = if index == start.line { start.offset } else { 0 };
                let to = if index == end.line { end.offset } else { line.clusters.len() };
                (to > from).then(|| TextBox {
                    rect: Rect::new(line.caret_x(from), line.top, line.caret_x(to), line.bottom),
                    angle: line.angle,
                })
            })
            .collect()
    }

    pub fn line_boxes(&self) -> Vec<TextBox> {
        self.lines
            .iter()
            .map(|line| TextBox {
                rect: Rect::new(line.left(), line.top, line.right(), line.bottom),
                angle: line.angle,
            })
            .collect()
    }

    pub fn selected_text(&self, anchor: TextPosition, focus: TextPosition) -> String {
        let (start, end) = Self::ordered(anchor, focus);
        let mut text = String::new();
        for index in start.line..=end.line.min(self.lines.len().saturating_sub(1)) {
            let line = &self.lines[index];
            let from = if index == start.line { start.offset } else { 0 };
            let to = if index == end.line { end.offset } else { line.clusters.len() };
            if index > start.line {
                text.push('\n');
            }
            text.push_str(&line.text(from, to));
        }
        text
    }

    pub fn moved(&self, position: TextPosition, motion: Motion, goal: Option<f64>) -> (TextPosition, Option<f64>) {
        let Some(line) = self.lines.get(position.line) else { return (position, None) };
        let last = self.lines.len() - 1;
        let end_of = |index: usize| TextPosition { line: index, offset: self.lines[index].clusters.len() };
        let moved = match motion {
            Motion::CharRight if position.offset < line.clusters.len() => {
                TextPosition { offset: position.offset + 1, ..position }
            }
            Motion::CharRight if position.line < last => TextPosition { line: position.line + 1, offset: 0 },
            Motion::CharLeft if position.offset > 0 => TextPosition { offset: position.offset - 1, ..position },
            Motion::CharLeft if position.line > 0 => end_of(position.line - 1),
            Motion::CharRight | Motion::CharLeft => position,
            Motion::WordRight => self.word_right(position),
            Motion::WordLeft => self.word_left(position),
            Motion::LineStart => TextPosition { offset: 0, ..position },
            Motion::LineEnd => end_of(position.line),
            Motion::DocumentStart => TextPosition { line: 0, offset: 0 },
            Motion::DocumentEnd => end_of(last),
            Motion::LineUp | Motion::LineDown => {
                let x = goal.unwrap_or_else(|| line.caret_x(position.offset));
                let target = match motion {
                    Motion::LineUp => position.line.checked_sub(1),
                    _ => (position.line < last).then_some(position.line + 1),
                };
                let moved = match target {
                    Some(target) => TextPosition { line: target, offset: self.lines[target].nearest_offset(x) },
                    None if motion == Motion::LineUp => TextPosition { offset: 0, ..position },
                    None => end_of(position.line),
                };
                return (moved, Some(x));
            }
        };
        (moved, None)
    }

    fn word_right(&self, position: TextPosition) -> TextPosition {
        let mut current = position;
        let line = &self.lines[current.line];
        if current.offset >= line.clusters.len() {
            return if current.line + 1 < self.lines.len() {
                TextPosition { line: current.line + 1, offset: 0 }
            } else {
                current
            };
        }
        while current.offset < line.clusters.len() && !line.clusters[current.offset].is_word() {
            current.offset += 1;
        }
        while current.offset < line.clusters.len() && line.clusters[current.offset].is_word() {
            current.offset += 1;
        }
        current
    }

    fn word_left(&self, position: TextPosition) -> TextPosition {
        let mut current = position;
        if current.offset == 0 {
            return match current.line.checked_sub(1) {
                Some(previous) => TextPosition { line: previous, offset: self.lines[previous].clusters.len() },
                None => current,
            };
        }
        let line = &self.lines[current.line];
        while current.offset > 0 && !line.clusters[current.offset - 1].is_word() {
            current.offset -= 1;
        }
        while current.offset > 0 && line.clusters[current.offset - 1].is_word() {
            current.offset -= 1;
        }
        current
    }

    pub fn word_at(&self, position: TextPosition) -> (TextPosition, TextPosition) {
        let Some(line) = self.lines.get(position.line) else { return (position, position) };
        let index = position.offset.min(line.clusters.len().saturating_sub(1));
        let word = line.clusters.get(index).is_some_and(|cluster| cluster.is_word());
        let mut start = index;
        let mut end = index + 1;
        if word {
            while start > 0 && line.clusters[start - 1].is_word() {
                start -= 1;
            }
            while end < line.clusters.len() && line.clusters[end].is_word() {
                end += 1;
            }
        }
        (
            TextPosition { line: position.line, offset: start },
            TextPosition { line: position.line, offset: end.min(line.clusters.len()) },
        )
    }

    pub fn paragraph_at(&self, position: TextPosition) -> (TextPosition, TextPosition) {
        let Some(paragraph) = self.lines.get(position.line).map(|line| line.paragraph) else {
            return (position, position);
        };
        let first = self.lines.iter().position(|line| line.paragraph == paragraph).unwrap_or(position.line);
        let last = self.lines.iter().rposition(|line| line.paragraph == paragraph).unwrap_or(position.line);
        (TextPosition { line: first, offset: 0 }, TextPosition { line: last, offset: self.lines[last].clusters.len() })
    }

    pub fn find(&self, query: &str) -> Vec<(TextPosition, TextPosition)> {
        let needle: Vec<String> = query.chars().map(|character| character.to_lowercase().collect()).collect();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut matches = Vec::new();
        for (index, line) in self.lines.iter().enumerate() {
            let haystack: Vec<(usize, String)> = line
                .clusters
                .iter()
                .enumerate()
                .flat_map(|(cluster, text)| {
                    text.text.chars().map(move |character| (cluster, character.to_lowercase().collect::<String>()))
                })
                .collect();
            let mut start = 0;
            while start + needle.len() <= haystack.len() {
                if haystack[start..start + needle.len()].iter().zip(&needle).all(|((_, a), b)| a == b) {
                    let from = haystack[start].0;
                    let to = haystack[start + needle.len() - 1].0 + 1;
                    matches
                        .push((TextPosition { line: index, offset: from }, TextPosition { line: index, offset: to }));
                    start += needle.len();
                } else {
                    start += 1;
                }
            }
        }
        matches
    }
}
