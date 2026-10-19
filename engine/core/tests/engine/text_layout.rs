use crate::support;

use support::fixtures;
use support::rendering::TestRenderer;
use tintenfisch::geometry::Rect;
use tintenfisch::text::{LayoutLine, Motion, TextGlyph, TextLayout, TextPosition, to_display_space};

fn line_text(line: &LayoutLine) -> String {
    line.clusters.iter().map(|cluster| cluster.text.as_str()).collect()
}

fn texts(layout: &TextLayout) -> Vec<String> {
    layout.lines().iter().map(line_text).collect()
}

fn all_layouts() -> Vec<(String, TextLayout)> {
    let mut layouts = Vec::new();
    for fixture in fixtures::all() {
        let renderer = TestRenderer::new(fixture.bytes);
        for page in 0..renderer.page_count() {
            layouts.push((format!("{} page {page}", fixture.name), (*renderer.layout(page)).clone()));
        }
    }
    layouts
}

fn same_row(a: &LayoutLine, b: &LayoutLine) -> bool {
    a.angle == b.angle && a.top < b.bottom && b.top < a.bottom
}

fn hit_along(layout: &TextLayout, line: &LayoutLine, x: f64, y: f64) -> Option<TextPosition> {
    let (x, y) = to_display_space(line.angle, (x, y));
    layout.hit(x, y)
}

#[test]
fn extraction_reads_lines_as_they_are_printed() {
    let article = TestRenderer::new(fixtures::committed().remove(0).bytes).layout(0);
    let lines = texts(&article);
    assert_eq!(lines[0], "On the Structure of Portable Documents");
    assert!(lines.iter().any(|line| line.contains("the full filter pipeline")), "ligatures read as letters");

    let invoice = TestRenderer::new(fixtures::committed().remove(1).bytes).layout(0);
    let lines = texts(&invoice);
    assert_eq!(lines[0], "Invoice 2026-0142");
    for cell in ["Item", "Qty", "Unit price", "Amount", "1080.00"] {
        assert!(lines.iter().any(|line| line == cell), "table cell {cell} is a line of its own");
    }

    let features = TestRenderer::new(fixtures::render_features()).layout(1);
    let lines = texts(&features);
    for expected in ["Été über", "spaced out words", "wideraised", "Ker ning"] {
        assert!(lines.iter().any(|line| line == expected), "missing {expected:?} in {lines:?}");
    }
    assert!(lines.iter().any(|line| line.contains("Hidden")), "invisible text stays selectable");
}

#[test]
fn every_cluster_hit_at_its_center_lands_on_its_own_line() {
    for (name, layout) in all_layouts() {
        for (index, line) in layout.lines().iter().enumerate() {
            for (offset, cluster) in line.clusters.iter().enumerate().filter(|(_, cluster)| !cluster.is_space()) {
                let x = (cluster.left + cluster.right) / 2.0;
                let y = (line.top + line.bottom) / 2.0;
                let hit = hit_along(&layout, line, x, y).unwrap_or_else(|| panic!("{name}: nothing at {x},{y}"));
                assert_eq!(hit.line, index, "{name}: {:?} of {:?}", cluster.text, line_text(line));
                assert!(hit.offset == offset || hit.offset == offset + 1, "{name}: offset {} for {offset}", hit.offset);
            }
        }
    }
}

#[test]
fn margins_keep_the_row_and_clamp_to_its_edge() {
    for (name, layout) in all_layouts() {
        let lines = layout.lines();
        for (index, line) in lines.iter().enumerate() {
            let y = (line.top + line.bottom) / 2.0;
            let leftmost = lines.iter().filter(|other| same_row(line, other)).all(|other| other.left() >= line.left());
            if leftmost {
                assert_eq!(
                    hit_along(&layout, line, line.left() - 8.0, y),
                    Some(TextPosition { line: index, offset: 0 }),
                    "{name}: left margin of {:?}",
                    line_text(line)
                );
            }
            let rightmost =
                lines.iter().filter(|other| same_row(line, other)).all(|other| other.right() <= line.right());
            if rightmost {
                let end = TextPosition { line: index, offset: line.clusters.len() };
                assert_eq!(
                    hit_along(&layout, line, line.right() + 8.0, y),
                    Some(end),
                    "{name}: right margin of {:?}",
                    line_text(line)
                );
            }
        }
    }
}

#[test]
fn a_click_between_stacked_lines_takes_the_nearer_one() {
    for (name, layout) in all_layouts() {
        let lines = layout.lines();
        for index in 1..lines.len() {
            let (upper, lower) = (&lines[index - 1], &lines[index]);
            let gap = lower.top - upper.bottom;
            if gap <= 1.0 || lower.paragraph != upper.paragraph {
                continue;
            }
            let x = upper.left() + 2.0;
            assert_eq!(layout.hit(x, upper.bottom + gap * 0.25).map(|hit| hit.line), Some(index - 1), "{name}");
            assert_eq!(layout.hit(x, lower.top - gap * 0.25).map(|hit| hit.line), Some(index), "{name}");
        }
    }
}

#[test]
fn a_click_between_table_columns_takes_the_nearer_cell() {
    let invoice = TestRenderer::new(fixtures::committed().remove(1).bytes).layout(0);
    let lines = invoice.lines();
    let item = lines.iter().position(|line| line_text(line) == "Item").unwrap();
    let quantity = lines.iter().position(|line| line_text(line) == "Qty").unwrap();
    let (left, right) = (lines[item].right(), lines[quantity].left());
    let y = (lines[item].top + lines[item].bottom) / 2.0;
    assert_eq!(invoice.hit(left + (right - left) * 0.3, y).map(|hit| hit.line), Some(item));
    assert_eq!(invoice.hit(left + (right - left) * 0.7, y).map(|hit| hit.line), Some(quantity));
}

#[test]
fn clicking_empty_page_area_selects_nothing() {
    let article = TestRenderer::new(fixtures::committed().remove(0).bytes).layout(0);
    assert_eq!(article.hit(590.0, 780.0), None);
    assert_eq!(article.hit(300.0, 5.0), None);
}

#[test]
fn selections_cover_their_lines_and_read_as_their_text() {
    for (name, layout) in all_layouts() {
        let lines = layout.lines();
        if lines.is_empty() {
            continue;
        }
        let start = TextPosition { line: 0, offset: 0 };
        assert!(layout.selection_boxes(start, start).is_empty(), "{name}: a collapsed selection paints nothing");
        let end = TextPosition { line: lines.len() - 1, offset: lines[lines.len() - 1].clusters.len() };
        for selected in layout.selection_boxes(end, start) {
            let covered =
                lines.iter().any(|line| line.angle == selected.angle && contains(&line.bounds(), &selected.rect));
            assert!(covered, "{name}: {selected:?} leaves every line");
        }
        let whole: Vec<String> = texts(&layout);
        assert_eq!(layout.selected_text(start, end), whole.join("\n"), "{name}");
    }
}

fn contains(outer: &Rect, inner: &Rect) -> bool {
    inner.x0 >= outer.x0 - 0.01
        && inner.x1 <= outer.x1 + 0.01
        && inner.y0 >= outer.y0 - 0.01
        && inner.y1 <= outer.y1 + 0.01
}

#[test]
fn word_motions_take_one_step_per_word_in_both_directions() {
    for (name, layout) in all_layouts() {
        for (index, line) in layout.lines().iter().enumerate() {
            let text = line_text(line);
            let words =
                text.split(|character: char| !character.is_alphanumeric()).filter(|word| !word.is_empty()).count();
            if words == 0 || text.contains('\u{FFFD}') {
                continue;
            }
            let end = line.clusters.len();
            let mut position = TextPosition { line: index, offset: 0 };
            let mut steps = 0;
            while position.offset < end {
                position = layout.moved(position, Motion::WordRight, None).0;
                steps += 1;
                assert_eq!(position.line, index, "{name}");
            }
            let trailing = line.clusters.last().is_some_and(|cluster| !cluster.is_word());
            assert_eq!(steps, words + usize::from(trailing), "{name}: {text:?}");
            let mut back = 0;
            while position.offset > 0 {
                position = layout.moved(position, Motion::WordLeft, None).0;
                back += 1;
            }
            let leading = line.clusters.first().is_some_and(|cluster| !cluster.is_word());
            assert_eq!(back, words + usize::from(leading), "{name}: {text:?}");
        }
    }
}

#[test]
fn vertical_motion_keeps_its_column_through_a_shorter_line() {
    let article = TestRenderer::new(fixtures::committed().remove(0).bytes).layout(0);
    let lines = article.lines();
    let long = lines.iter().position(|line| line_text(line).starts_with("The portable document")).unwrap();
    let start = TextPosition { line: long, offset: lines[long].clusters.len() - 5 };
    let (second, goal) = article.moved(start, Motion::LineDown, None);
    let (third, goal) = article.moved(second, Motion::LineDown, goal);
    assert_eq!(third.line, long + 2);
    assert!(third.offset <= lines[long + 2].clusters.len());
    let (back, goal) = article.moved(third, Motion::LineUp, goal);
    let (top, _) = article.moved(back, Motion::LineUp, goal);
    assert_eq!(back, second, "the goal column survives the shorter line");
    assert_eq!(top.line, start.line);
    assert!(top.offset.abs_diff(start.offset) <= 1);
}

#[test]
fn character_motion_crosses_line_ends_and_stops_at_the_document_edges() {
    let article = TestRenderer::new(fixtures::committed().remove(0).bytes).layout(0);
    let first_end = TextPosition { line: 0, offset: article.lines()[0].clusters.len() };
    assert_eq!(article.moved(first_end, Motion::CharRight, None).0, TextPosition { line: 1, offset: 0 });
    assert_eq!(article.moved(TextPosition { line: 1, offset: 0 }, Motion::CharLeft, None).0, first_end);
    let start = TextPosition { line: 0, offset: 0 };
    assert_eq!(article.moved(start, Motion::CharLeft, None).0, start);
    let end = article.moved(start, Motion::DocumentEnd, None).0;
    assert_eq!(article.moved(end, Motion::CharRight, None).0, end);
}

#[test]
fn double_and_triple_clicks_pick_words_and_paragraphs() {
    let article = TestRenderer::new(fixtures::committed().remove(0).bytes).layout(0);
    let (start, end) = article.word_at(TextPosition { line: 0, offset: 9 });
    assert_eq!(article.selected_text(start, end), "Structure");
    let (start, end) = article.paragraph_at(TextPosition { line: 2, offset: 3 });
    let text = article.selected_text(start, end);
    assert!(text.starts_with("The portable document") && text.ends_with("white rectangles."));
    assert_eq!(text.lines().count(), 3);
}

#[test]
fn find_matches_regardless_of_case() {
    let article = TestRenderer::new(fixtures::committed().remove(0).bytes).layout(0);
    let matches = article.find("CONTENT STREAM");
    assert_eq!(matches.len(), 2);
    assert_eq!(article.selected_text(matches[0].0, matches[0].1), "Content stream");
    assert_eq!(article.find("filter").len(), 1);
    assert!(article.find("").is_empty());
}

fn glyph(text: &str, start: f64, end: f64, ordinal: usize) -> TextGlyph {
    TextGlyph {
        origin: tintenfisch::text::GlyphOrigin { form: None, ordinal, index: 0 },
        text: text.to_string(),
        angle: 0.0,
        bounds: Rect::new(start, 90.0, end, 102.0),
        baseline: 100.0,
        start,
        end,
        size: 12.0,
        hidden: false,
        block: None,
    }
}

#[test]
fn combining_marks_and_joined_emoji_stay_one_caret_stop() {
    let glyphs = [
        glyph("e", 0.0, 6.0, 0),
        glyph("\u{0301}", 6.0, 6.0, 1),
        glyph("\u{1F468}\u{200D}", 7.0, 19.0, 2),
        glyph("\u{1F469}", 19.0, 31.0, 3),
        glyph("x", 31.0, 37.0, 4),
    ];
    let layout = TextLayout::new(&glyphs);
    let clusters: Vec<&str> = layout.lines()[0].clusters.iter().map(|cluster| cluster.text.as_str()).collect();
    assert_eq!(clusters, ["e\u{0301}", "\u{1F468}\u{200D}\u{1F469}", "x"]);
    let after_first = layout.moved(TextPosition { line: 0, offset: 0 }, Motion::CharRight, None).0;
    assert_eq!(layout.selected_text(TextPosition { line: 0, offset: 0 }, after_first), "e\u{0301}");
}

fn line_glyph(text: &str, start: f64, baseline: f64, size: f64, ordinal: usize) -> TextGlyph {
    let end = start + text.chars().count() as f64 * size * 0.5;
    TextGlyph {
        bounds: Rect::new(start, baseline - size * 0.8, end, baseline + size * 0.2),
        baseline,
        size,
        ..glyph(text, start, end, ordinal)
    }
}

#[test]
fn lines_close_enough_to_overlap_are_separate_blocks_not_one_paragraph() {
    let paragraph =
        TextLayout::new(&[line_glyph("Subtitle", 54.0, 96.7, 12.0, 0), line_glyph("next line", 54.0, 111.0, 12.0, 1)]);
    let paragraphs: Vec<usize> = paragraph.lines().iter().map(|line| line.paragraph).collect();
    assert_eq!(paragraphs, [0, 0]);
    let pushed = TextLayout::new(&[
        line_glyph("Subtitle", 54.0, 119.7, 12.0, 0),
        line_glyph("Smaller line below", 54.0, 124.5, 10.0, 1),
    ]);
    let paragraphs: Vec<usize> = pushed.lines().iter().map(|line| line.paragraph).collect();
    assert_eq!(paragraphs, [0, 1]);
}

#[test]
fn every_paragraph_has_one_box_around_all_its_lines() {
    let layout = TextLayout::new(&[
        line_glyph("First line", 54.0, 100.0, 12.0, 0),
        line_glyph("Second", 54.0, 114.0, 12.0, 1),
        line_glyph("Apart", 54.0, 160.0, 12.0, 2),
    ]);
    let boxes: Vec<(f64, f64, f64, f64)> =
        layout.block_boxes().iter().map(|block| (block.rect.x0, block.rect.y0, block.rect.x1, block.rect.y1)).collect();
    assert_eq!(boxes, [(54.0, 90.4, 114.0, 116.4), (54.0, 150.4, 84.0, 162.4)]);
}

#[test]
fn lines_of_different_blocks_never_form_one_paragraph() {
    let moved = TextGlyph { block: Some(1), ..line_glyph("Moved block", 54.0, 100.0, 12.0, 0) };
    let layout = TextLayout::new(&[moved, line_glyph("Original block", 54.0, 114.0, 12.0, 1)]);
    let paragraphs: Vec<usize> = layout.lines().iter().map(|line| line.paragraph).collect();
    assert_eq!(paragraphs, [0, 1]);
}
