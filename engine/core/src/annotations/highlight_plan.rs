use std::collections::BTreeSet;

use crate::annotations::Annotation;
use crate::annotations::markup_builder::bounds;
use crate::geometry::{Matrix, Rect};
use crate::syntax::{ObjId, Object, decode_text};
use crate::text::{TextLayout, TextPosition, to_display_space};

const SAME_COLOR: f64 = 0.01;

#[derive(Clone, Debug, PartialEq)]
pub enum HighlightPlan {
    Unchanged(ObjId),
    Draw { anchor: TextPosition, focus: TextPosition, replaced: Vec<ObjId>, notes: Vec<String> },
}

struct Existing {
    id: ObjId,
    color: Option<[f64; 3]>,
    covered: BTreeSet<TextPosition>,
    note: Option<String>,
}

impl HighlightPlan {
    pub fn new(
        layout: &TextLayout,
        to_display: &Matrix,
        annotations: &[Annotation],
        (anchor, focus): (TextPosition, TextPosition),
        color: [f64; 3],
    ) -> HighlightPlan {
        let selected = covered_between(layout, anchor, focus);
        let existing: Vec<Existing> = annotations
            .iter()
            .filter(|annotation| annotation.subtype.is("Highlight"))
            .filter_map(|annotation| Existing::read(layout, to_display, annotation))
            .filter(|existing| !existing.covered.is_disjoint(&selected))
            .collect();
        let same = |existing: &Existing| {
            existing.color.is_some_and(|known| known.iter().zip(color).all(|(a, b)| (a - b).abs() < SAME_COLOR))
        };
        if let Some(enclosing) =
            existing.iter().find(|existing| same(existing) && existing.covered.is_superset(&selected))
        {
            return HighlightPlan::Unchanged(enclosing.id);
        }
        let absorbed: Vec<&Existing> =
            existing.iter().filter(|existing| same(existing) || existing.covered.is_subset(&selected)).collect();
        let covered: BTreeSet<TextPosition> =
            absorbed.iter().flat_map(|existing| existing.covered.iter().copied()).chain(selected).collect();
        let (anchor, focus) = match (covered.first(), covered.last()) {
            (Some(first), Some(last)) => (*first, TextPosition { line: last.line, offset: last.offset + 1 }),
            _ => (anchor, focus),
        };
        HighlightPlan::Draw {
            anchor,
            focus,
            replaced: absorbed.iter().map(|existing| existing.id).collect(),
            notes: absorbed.iter().filter_map(|existing| existing.note.clone()).collect(),
        }
    }
}

impl Existing {
    fn read(layout: &TextLayout, to_display: &Matrix, annotation: &Annotation) -> Option<Existing> {
        let numbers: Vec<f64> =
            annotation.dict.get("QuadPoints")?.as_array()?.iter().filter_map(Object::as_number).collect();
        let quads: Vec<Rect> = numbers
            .as_chunks::<8>()
            .0
            .iter()
            .map(|quad| {
                let points: Vec<(f64, f64)> =
                    quad.as_chunks::<2>().0.iter().map(|&[x, y]| to_display.apply(x, y)).collect();
                bounds(&points, 0.0)
            })
            .collect();
        let mut covered = BTreeSet::new();
        for (index, line) in layout.lines().iter().enumerate() {
            for (offset, cluster) in line.clusters.iter().enumerate().filter(|(_, cluster)| !cluster.is_space()) {
                let (x, y) = to_display_space(
                    line.angle,
                    ((cluster.left + cluster.right) / 2.0, (line.top + line.bottom) / 2.0),
                );
                if quads.iter().any(|quad| x >= quad.x0 && x <= quad.x1 && y >= quad.y0 && y <= quad.y1) {
                    covered.insert(TextPosition { line: index, offset });
                }
            }
        }
        let (first, last) = (*covered.first()?, *covered.last()?);
        let quoted = layout.selected_text(first, TextPosition { line: last.line, offset: last.offset + 1 });
        let contents = annotation.dict.get("Contents").and_then(Object::as_string).map(decode_text).unwrap_or_default();
        let compact = |text: &str| text.split_whitespace().collect::<String>();
        let note = (!contents.trim().is_empty() && compact(&contents) != compact(&quoted)).then_some(contents);
        let color = annotation.dict.get("C").and_then(Object::as_array).and_then(|components| {
            <[f64; 3]>::try_from(components.iter().filter_map(Object::as_number).collect::<Vec<_>>()).ok()
        });
        Some(Existing { id: annotation.id?, color, covered, note })
    }
}

fn covered_between(layout: &TextLayout, anchor: TextPosition, focus: TextPosition) -> BTreeSet<TextPosition> {
    let (start, end) = if anchor <= focus { (anchor, focus) } else { (focus, anchor) };
    layout
        .lines()
        .iter()
        .enumerate()
        .flat_map(|(index, line)| {
            line.clusters
                .iter()
                .enumerate()
                .filter(|(_, cluster)| !cluster.is_space())
                .map(move |(offset, _)| TextPosition { line: index, offset })
        })
        .filter(|position| *position >= start && *position < end)
        .collect()
}
