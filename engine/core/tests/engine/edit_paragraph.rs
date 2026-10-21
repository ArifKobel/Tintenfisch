use crate::support;

use std::collections::BTreeSet;

use support::fixtures;
use support::rendering::library;
use tintenfisch::content::ContentStream;
use tintenfisch::document::Document;
use tintenfisch::edit::ParagraphEdit;
use tintenfisch::fonts::{FontCache, FontLibrary};
use tintenfisch::pages::PageId;
use tintenfisch::text::{LayoutLine, TextExtractor, TextGlyph, to_display_space};

const SECOND_LINE: &str =
    "state. Text is not stored as paragraphs but as positioned glyph runs, which is precisely what makes in-place";

struct Article {
    document: Document,
    library: FontLibrary,
    page: PageId,
}

impl Article {
    fn open() -> Self {
        let library = library();
        let fixture = fixtures::all().into_iter().find(|fixture| fixture.name == "chrome-article.pdf").unwrap();
        let document = Document::open(fixture.bytes, "").unwrap();
        let page = document.view(&library).pages().unwrap()[0].id;
        Self { document, library, page }
    }

    fn lines(&self) -> Vec<LayoutLine> {
        self.document.view(&self.library).text_layout(self.page).unwrap().lines().to_vec()
    }

    fn glyphs(&self) -> Vec<TextGlyph> {
        self.document.view(&self.library).text_glyphs(self.page).unwrap()
    }

    fn line_index(&self, text: &str) -> usize {
        self.lines().iter().position(|line| line_text(line) == text).unwrap_or_else(|| panic!("no line {text:?}"))
    }

    fn paragraph(&self, line: usize) -> String {
        self.document.paragraph_text(self.page, line, &self.library).unwrap().clusters.concat()
    }

    fn edit(&self, line: usize, text: &str) -> ParagraphEdit {
        ParagraphEdit::new(self.page, line, text)
    }

    fn apply(&mut self, line: usize, text: &str) {
        let edit = self.edit(line, text);
        self.document.edit(&edit, &self.library, "").unwrap();
    }

    fn operations(&self) -> Vec<(usize, Vec<u8>)> {
        let view = self.document.view(&self.library);
        let content = ContentStream::load(self.document.store(), &view.page(self.page).unwrap()).unwrap();
        content
            .operations
            .iter()
            .map(|operation| {
                (operation.ordinal, content.segments[operation.segment].data[operation.span.clone()].to_vec())
            })
            .collect()
    }
}

fn fonts_of(document: &Document, library: &FontLibrary, page: PageId, phrase: &str) -> BTreeSet<String> {
    let view = document.view(library);
    let page = view.page(page).unwrap();
    let fonts = FontCache::new();
    let (glyphs, records) = TextExtractor::new(document.store(), &fonts, library).page_with_records(&page).unwrap();
    let inked: Vec<&TextGlyph> = glyphs.iter().filter(|glyph| !glyph.text.trim().is_empty()).collect();
    let text: String = inked.iter().map(|glyph| glyph.text.as_str()).collect();
    let wanted: String = phrase.chars().filter(|character| !character.is_whitespace()).collect();
    let at = text.find(&wanted).unwrap_or_else(|| panic!("no {wanted:?} in {text:?}"));
    let first = text[..at].chars().count();
    inked[first..first + wanted.chars().count()]
        .iter()
        .map(|glyph| {
            let record = records.iter().find(|record| record.ordinal == glyph.origin.ordinal).unwrap();
            record.font_name.as_ref().map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()).unwrap()
        })
        .collect()
}

fn line_text(line: &LayoutLine) -> String {
    line.clusters.iter().map(|cluster| cluster.text.as_str()).collect()
}

fn paragraph_lines(lines: &[LayoutLine], paragraph: usize) -> Vec<&LayoutLine> {
    lines.iter().filter(|line| line.paragraph == paragraph).collect()
}

fn same_place(a: &TextGlyph, b: &TextGlyph) -> bool {
    (a.start - b.start).abs() < 1e-6 && (a.baseline - b.baseline).abs() < 1e-6 && a.text == b.text
}

fn assert_others_unmoved(before: &Article, after: &Article, paragraph: usize) {
    let lines = before.lines();
    let in_paragraph: BTreeSet<_> = paragraph_lines(&lines, paragraph)
        .iter()
        .flat_map(|line| &line.clusters)
        .flat_map(|cluster| cluster.origins.iter().copied())
        .collect();
    let moved_to = after.glyphs();
    for glyph in before.glyphs().iter().filter(|glyph| !in_paragraph.contains(&glyph.origin)) {
        assert!(moved_to.iter().any(|other| same_place(glyph, other)), "{:?} moved", glyph.text);
    }
}

#[test]
fn a_paragraph_reads_as_one_text_with_its_wraps_as_spaces() {
    let article = Article::open();
    let second = article.line_index(SECOND_LINE);
    let text = article.document.paragraph_text(article.page, second, &article.library).unwrap();
    assert_eq!(text.line, second - 1);
    assert_eq!(text.line_starts.len(), 3);
    let lines = article.lines();
    let expected = [&lines[second - 1], &lines[second], &lines[second + 1]].map(line_text).join(" ");
    assert_eq!(text.clusters.concat(), expected);
    assert_eq!(article.paragraph(0), "On the Structure of Portable Documents");
}

#[test]
fn inserted_words_push_the_rest_of_the_paragraph_down_its_lines() {
    let original = Article::open();
    let mut article = Article::open();
    let second = article.line_index(SECOND_LINE);
    let paragraph = article.lines()[second].paragraph;
    let text = article.paragraph(second).replace("describes pages", "describes many long and winding pages");
    article.apply(second, &text);
    assert_eq!(article.paragraph(second - 1), text, "the paragraph reads as typed");
    let before = original.lines();
    let after = article.lines();
    let old_lines = paragraph_lines(&before, paragraph);
    let new_lines = paragraph_lines(&after, paragraph);
    let right = old_lines.iter().map(|line| line.right()).fold(0.0, f64::max);
    assert_eq!(new_lines.len(), 4, "the paragraph grew by a line");
    for line in &new_lines {
        assert!(line.right() <= right + 0.01, "{:?} runs past the paragraph edge", line_text(line));
        assert!((line.left() - old_lines[0].left()).abs() < 1e-6);
    }
    let step = old_lines[1].baseline - old_lines[0].baseline;
    assert!((new_lines[3].baseline - new_lines[2].baseline - step).abs() < 0.5, "the new line keeps the spacing");
    for (index, cluster) in new_lines[0].clusters.iter().take("The".len()).enumerate() {
        assert!((cluster.left - old_lines[0].clusters[index].left).abs() < 1e-6, "the line keeps its start");
    }
    for line in &new_lines[..new_lines.len() - 1] {
        assert!((line.right() - right).abs() < 0.01, "{:?} is not justified", line_text(line));
    }
    assert_others_unmoved(&original, &article, paragraph);
}

#[test]
fn deleted_words_pull_the_next_line_up() {
    let mut article = Article::open();
    let second = article.line_index(SECOND_LINE);
    let text = article.paragraph(second).replace("Text is not stored as paragraphs but as positioned glyph runs, ", "");
    article.apply(second, &text);
    assert_eq!(article.paragraph(second - 1), text);
    let lines = article.lines();
    assert!(line_text(&lines[second]).contains("editing a hard problem"), "words of the third line moved up");
    assert_eq!(paragraph_lines(&lines, lines[second].paragraph).len(), 3);
    assert!(line_text(&lines[second + 1]).ends_with("drawing white rectangles."));
}

#[test]
fn a_hard_break_ends_the_line_and_reads_back_as_one() {
    let original = Article::open();
    let mut article = Article::open();
    let second = article.line_index(SECOND_LINE);
    let paragraph = article.lines()[second].paragraph;
    let text = article.paragraph(second).replacen("state. ", "state.\n", 1);
    article.apply(second, &text);
    assert_eq!(article.paragraph(second - 1), text);
    assert_eq!(line_text(&article.lines()[second]), "state.");
    assert_others_unmoved(&original, &article, paragraph);
}

#[test]
fn typed_text_takes_the_font_of_the_text_before_it_wherever_it_lands() {
    let mut article = Article::open();
    let second = article.line_index(SECOND_LINE);
    let typed = "and then words that make the paragraph grow by a line.";
    let text = article.paragraph(second).replacen("state. ", &format!("state.\n{typed} "), 1);
    article.apply(second, &text);
    let fonts = |phrase: &str| fonts_of(&article.document, &article.library, article.page, phrase);
    assert_eq!(fonts(typed), fonts("state."));
    assert_ne!(fonts("in-place"), fonts("state."));
}

#[test]
fn lines_before_the_change_and_after_the_breaks_meet_again_stay_byte_identical() {
    let original = Article::open();
    let mut article = Article::open();
    let second = article.line_index(SECOND_LINE);
    let lines = article.lines();
    let records = |line: &LayoutLine| -> BTreeSet<usize> {
        line.clusters.iter().flat_map(|cluster| &cluster.origins).map(|origin| origin.ordinal).collect()
    };
    let edited = records(&lines[second]);
    let text = article.paragraph(second).replacen("precisely", "precisly", 1);
    article.apply(second, &text);
    assert_eq!(line_text(&article.lines()[second - 1]), line_text(&lines[second - 1]));
    let after = article.operations();
    for (ordinal, bytes) in original.operations() {
        let unchanged = after.iter().any(|(_, other_bytes)| *other_bytes == bytes);
        assert!(unchanged || edited.contains(&ordinal), "operation {ordinal} outside the edited line changed");
    }
}

#[test]
fn the_caret_follows_the_planned_lines() {
    let mut article = Article::open();
    let second = article.line_index(SECOND_LINE);
    let paragraph = article.paragraph(second);
    let text = paragraph.replacen("state. ", "state.\n", 1);
    let offset = text.find('\n').unwrap() + 1;
    let edit = article.edit(second, &text);
    let plan = article.document.plan_edit(&edit, &article.library).unwrap();
    let (caret, line) = article
        .document
        .preview(&plan, &article.library, |view| {
            let layout = view.text_layout(edit.page).unwrap();
            (plan.caret(&layout, offset).unwrap(), layout.lines()[second + 1].clone())
        })
        .unwrap();
    assert!((caret.x - line.left()).abs() < 1e-6);
    assert!(caret.top < line.baseline && line.baseline < caret.bottom);
}

#[test]
fn the_preview_shows_exactly_what_the_commit_writes_and_undo_restores_the_file() {
    let original = fixtures::all().into_iter().find(|fixture| fixture.name == "chrome-article.pdf").unwrap().bytes;
    let mut article = Article::open();
    let second = article.line_index(SECOND_LINE);
    for text in [
        article.paragraph(second).replace("describes pages", "describes many long and winding pages"),
        article.paragraph(second).replacen("state. ", "state.\n\n", 1),
        "Short.".to_string(),
    ] {
        let edit = article.edit(second, &text);
        let render = |view: tintenfisch::document::DocumentView| view.render(edit.page, 1.0, None).unwrap().rgba;
        let previewed = article.document.preview_edit(&edit, &article.library, render).unwrap();
        article.document.edit(&edit, &article.library, "").unwrap();
        let committed = article.document.view(&article.library).render(edit.page, 1.0, None).unwrap().rgba;
        assert!(committed == previewed, "{text:?}: preview and commit differ");
        article.document.undo().unwrap();
    }
    assert!(article.document.save(false).unwrap() == original);
}

struct Block {
    document: Document,
    library: FontLibrary,
    page: PageId,
}

impl Block {
    fn open() -> Self {
        let library = library();
        let document = Document::open(fixtures::reflow_features(), "").unwrap();
        let page = document.view(&library).pages().unwrap()[0].id;
        Self { document, library, page }
    }

    fn lines(&self) -> Vec<LayoutLine> {
        self.document.view(&self.library).text_layout(self.page).unwrap().lines().to_vec()
    }

    fn edit(&mut self, text: &str) {
        let edit = ParagraphEdit::new(self.page, 0, text);
        self.document.edit(&edit, &self.library, "").unwrap();
    }

    fn font_of_word(&self, word: &str) -> String {
        let fonts = fonts_of(&self.document, &self.library, self.page, word);
        assert_eq!(fonts.len(), 1, "{word:?} is shown in {fonts:?}");
        fonts.into_iter().next().unwrap()
    }
}

#[test]
fn the_block_reads_as_one_paragraph_with_a_bold_word() {
    let block = Block::open();
    let text = block.document.paragraph_text(block.page, 1, &block.library).unwrap();
    assert_eq!(text.clusters.concat(), "Lines of one block wrap at the widest of them and flow on when text grows.");
    assert_eq!(block.font_of_word("flow"), "F2");
}

#[test]
fn lines_added_at_the_end_leave_what_follows_in_the_same_text_object_in_place() {
    let mut block = Block::open();
    let after = |lines: &[LayoutLine]| {
        lines
            .iter()
            .find(|line| line_text(line) == "Positioned after the block")
            .map(|line| (line.left(), line.baseline))
    };
    let before = after(&block.lines()).unwrap();
    block.edit("Lines of one block wrap at the widest of them and flow on when text grows.\nA second last line.");
    let lines = block.lines();
    assert_eq!(line_text(&lines[3]), "A second last line.");
    assert!((lines[3].baseline - lines[2].baseline - 14.0).abs() < 1e-6, "the new line keeps the leading");
    assert_eq!(after(&lines), Some(before));
}

#[test]
fn a_word_that_moves_to_another_line_keeps_its_font() {
    let mut block = Block::open();
    block.edit("Lines of one block wrap at the widest of them and then rather suddenly flow on when text grows.");
    let lines = block.lines();
    assert!(line_text(&lines[2]).contains("flow"), "{:?}", lines.iter().map(line_text).collect::<Vec<_>>());
    assert_eq!(block.font_of_word("flow"), "F2");
    assert_eq!(block.font_of_word("when"), "F1");
}

fn display_origin(glyph: &TextGlyph) -> (f64, f64) {
    to_display_space(glyph.angle, (glyph.start, glyph.baseline))
}

fn paragraph_glyphs<'g>(glyphs: &'g [TextGlyph], lines: &[LayoutLine], paragraph: usize) -> Vec<&'g TextGlyph> {
    let origins: BTreeSet<_> = paragraph_lines(lines, paragraph)
        .iter()
        .flat_map(|line| &line.clusters)
        .flat_map(|cluster| cluster.origins.iter().copied())
        .collect();
    glyphs.iter().filter(|glyph| origins.contains(&glyph.origin)).collect()
}

#[test]
fn a_moved_box_carries_its_glyphs_and_leaves_the_rest() {
    let original = Article::open();
    let mut article = Article::open();
    let second = article.line_index(SECOND_LINE);
    let paragraph = article.lines()[second].paragraph;
    let text = article.paragraph(second);
    let edit = article.edit(second, &text).moved_by(12.0, -30.0);
    article.document.edit(&edit, &article.library, "").unwrap();
    let before =
        paragraph_glyphs(&original.glyphs(), &original.lines(), paragraph).into_iter().cloned().collect::<Vec<_>>();
    let after = article.glyphs();
    for glyph in &before {
        let (x, y) = display_origin(glyph);
        let moved = after.iter().any(|other| {
            let (ox, oy) = display_origin(other);
            other.text == glyph.text && (ox - x - 12.0).abs() < 1e-6 && (oy - y + 30.0).abs() < 1e-6
        });
        assert!(moved, "{:?} did not move with its box", glyph.text);
    }
    assert_others_unmoved(&original, &article, paragraph);
    assert_eq!(article.paragraph(second - 1), text);
}

#[test]
fn a_moved_block_leaves_what_follows_it_in_the_same_text_object_in_place() {
    let mut block = Block::open();
    let after = |lines: &[LayoutLine]| {
        lines
            .iter()
            .find(|line| line_text(line) == "Positioned after the block")
            .map(|line| (line.left(), line.baseline))
    };
    let before = after(&block.lines()).unwrap();
    let first = block.lines()[0].clone();
    let text = block.document.paragraph_text(block.page, 0, &block.library).unwrap().clusters.concat();
    let edit = ParagraphEdit::new(block.page, 0, text).moved_by(30.0, 15.0);
    block.document.edit(&edit, &block.library, "").unwrap();
    let lines = block.lines();
    assert!(
        (lines[0].left() - first.left() - 30.0).abs() < 1e-6
            && (lines[0].baseline - first.baseline - 15.0).abs() < 1e-6
    );
    assert_eq!(after(&lines), Some(before));
    assert_eq!(block.font_of_word("flow"), "F2");
}

#[test]
fn a_narrower_box_wraps_into_more_lines_and_a_wider_one_into_fewer() {
    for (width, more) in [(110.0, true), (330.0, false)] {
        let mut block = Block::open();
        let lines = block.lines();
        let count = paragraph_lines(&lines, 0).len();
        let left = lines[0].left();
        let text = block.document.paragraph_text(block.page, 0, &block.library).unwrap().clusters.concat();
        let edit = ParagraphEdit::new(block.page, 0, text.clone()).with_width(width);
        block.document.edit(&edit, &block.library, "").unwrap();
        let lines = block.lines();
        let resized = paragraph_lines(&lines, 0);
        assert_eq!(
            resized.len() > count,
            more,
            "{width}: {:?}",
            resized.iter().map(|line| line_text(line)).collect::<Vec<_>>()
        );
        assert!(resized.iter().all(|line| line.right() <= left + width + 0.01), "{width}: a line runs past the box");
        let reread = block.document.paragraph_text(block.page, 0, &block.library).unwrap().clusters.concat();
        assert_eq!(reread, text);
    }
}

#[test]
fn a_resized_justified_paragraph_is_justified_to_its_new_edge() {
    let mut article = Article::open();
    let second = article.line_index(SECOND_LINE);
    let paragraph = article.lines()[second].paragraph;
    let left = article.lines()[second].left();
    let text = article.paragraph(second);
    let edit = article.edit(second, &text).with_width(380.0);
    article.document.edit(&edit, &article.library, "").unwrap();
    let lines = article.lines();
    let resized = paragraph_lines(&lines, paragraph);
    assert!(resized.len() > 3);
    for line in &resized[..resized.len() - 1] {
        assert!((line.right() - (left + 380.0)).abs() < 0.01, "{:?} ends at {}", line_text(line), line.right());
    }
    assert!(resized.last().unwrap().right() < left + 380.0 - 1.0, "the last line stays ragged");
}

#[test]
fn a_ragged_paragraph_stays_ragged() {
    let mut block = Block::open();
    let right = block.lines()[..3].iter().map(|line| line.right()).fold(0.0, f64::max);
    block.edit("Lines of one block wrap at the widest of them and then rather suddenly flow on when text grows.");
    let lines = block.lines();
    assert!(paragraph_lines(&lines, 0)[..2].iter().any(|line| line.right() < right - 1.0), "lines were stretched");
}

#[test]
fn the_frame_wraps_the_planned_lines_and_follows_the_move() {
    let article = Article::open();
    let second = article.line_index(SECOND_LINE);
    let lines = article.lines();
    let paragraph = paragraph_lines(&lines, lines[second].paragraph);
    let text = article.paragraph(second);
    let plan = article.document.plan_edit(&article.edit(second, &text).moved_by(5.0, 7.0), &article.library).unwrap();
    let frame = plan.frame();
    let right = paragraph.iter().map(|line| line.right()).fold(0.0, f64::max);
    assert!((frame.rect.x0 - paragraph[0].left() - 5.0).abs() < 1e-6);
    assert!((frame.rect.x1 - right - 5.0).abs() < 1e-6);
    assert!(frame.rect.y0 < paragraph[0].baseline + 7.0 && frame.rect.y1 > paragraph[2].baseline + 7.0);
}

#[test]
fn moves_and_resizes_preview_exactly_and_undo_completely() {
    let original = fixtures::reflow_features();
    let mut block = Block::open();
    let text = block.document.paragraph_text(block.page, 0, &block.library).unwrap().clusters.concat();
    for edit in [
        ParagraphEdit::new(block.page, 0, text.clone()).moved_by(-8.0, 22.0),
        ParagraphEdit::new(block.page, 0, text.replace("flow", "flow freely")).with_width(140.0).moved_by(10.0, 0.0),
    ] {
        let render = |view: tintenfisch::document::DocumentView| view.render(edit.page, 1.0, None).unwrap().rgba;
        let previewed = block.document.preview_edit(&edit, &block.library, render).unwrap();
        block.document.edit(&edit, &block.library, "").unwrap();
        let committed = block.document.view(&block.library).render(edit.page, 1.0, None).unwrap().rgba;
        assert!(committed == previewed, "{edit:?}: preview and commit differ");
        block.document.undo().unwrap();
    }
    assert!(block.document.save(false).unwrap() == original);
}

#[test]
fn a_paragraph_moved_onto_the_next_stays_its_own_block_also_after_saving() {
    let library = library();
    let mut document = Document::open(fixtures::editing_features(), "").unwrap();
    let page = document.view(&library).pages().unwrap()[0].id;
    let lines_of = |document: &Document| document.view(&library).text_layout(page).unwrap().lines().to_vec();
    let paragraph_of = |lines: &[LayoutLine], text: &str| {
        lines.iter().find(|line| line_text(line) == text).map(|line| line.paragraph).unwrap()
    };
    let lines = lines_of(&document);
    let (moved, below) = (
        lines.iter().position(|line| line_text(line) == "Plain line to edit").unwrap(),
        lines.iter().position(|line| line_text(line) == "Condensed line").unwrap(),
    );
    let mut edit = ParagraphEdit::new(page, moved, "Plain line to edit");
    edit.offset = (0.0, lines[below].baseline - 1.2 * lines[below].size - lines[moved].baseline);
    document.edit(&edit, &library, "").unwrap();
    let lines = lines_of(&document);
    assert_ne!(paragraph_of(&lines, "Plain line to edit"), paragraph_of(&lines, "Condensed line"));
    let reopened = Document::open(document.save(false).unwrap(), "").unwrap();
    let lines = lines_of(&reopened);
    assert_ne!(paragraph_of(&lines, "Plain line to edit"), paragraph_of(&lines, "Condensed line"));
}

#[test]
fn a_paragraph_that_grows_into_the_next_block_stays_apart_from_it() {
    let library = library();
    let mut document = Document::open(fixtures::reflow_features(), "").unwrap();
    let page = document.view(&library).pages().unwrap()[0].id;
    let lines_of = |document: &Document| document.view(&library).text_layout(page).unwrap().lines().to_vec();
    let paragraph = document.paragraph_text(page, 0, &library).unwrap().clusters.concat();
    let grown = format!("{paragraph} and then keeps on growing with many more words until it reaches");
    document.edit(&ParagraphEdit::new(page, 0, grown), &library, "").unwrap();
    let lines = lines_of(&document);
    let after = lines.iter().find(|line| line_text(line) == "Positioned after the block").unwrap();
    let above = lines.iter().rfind(|line| line.baseline < after.baseline).unwrap();
    assert!(
        (0.75 * after.size..2.0 * after.size).contains(&(after.baseline - above.baseline)),
        "the grown paragraph ends just above the next block"
    );
    assert_ne!(above.paragraph, after.paragraph);
}

#[test]
fn a_point_in_the_draft_maps_back_to_the_offset_whose_caret_is_there() {
    let mut article = Article::open();
    let second = article.line_index(SECOND_LINE);
    let text = article.paragraph(second).replacen("precisely", "very precisely", 1);
    let plan = article.document.plan_edit(&article.edit(second, &text), &article.library).unwrap();
    for offset in [0, 5, text.chars().count() / 2, text.chars().count() - 3] {
        let found = article
            .document
            .preview(&plan, &article.library, |view| {
                let layout = view.text_layout(article.page)?;
                let caret = plan.caret(&layout, offset).unwrap();
                Ok::<_, tintenfisch::error::Error>(plan.offset_at(
                    &layout,
                    caret.x + 0.1,
                    (caret.top + caret.bottom) / 2.0,
                ))
            })
            .unwrap()
            .unwrap();
        assert_eq!(found, Some(offset), "offset {offset}");
    }
}

#[test]
fn a_ligature_stays_one_cluster_so_the_caret_reaches_the_end_of_the_line() {
    let library = library();
    let sample = include_bytes!("../../../../src/assets/sample.pdf");
    let mut document = Document::open(sample.to_vec(), "").unwrap();
    let page = document.view(&library).pages().unwrap()[0].id;
    let lines = document.view(&library).text_layout(page).unwrap().lines().to_vec();
    let heading = lines.iter().position(|line| line_text(line) == "Welcome to Tintenfisch").unwrap();
    let paragraph = document.paragraph_text(page, heading, &library).unwrap();
    assert!(paragraph.clusters.contains(&"fi".to_string()));

    for suffix in ["", "!"] {
        let mut clusters = paragraph.clusters.clone();
        clusters.extend((!suffix.is_empty()).then(|| suffix.to_string()));
        let end = clusters.len();
        let edit = ParagraphEdit::clustered(page, paragraph.line, clusters);
        let plan = document.plan_edit(&edit, &library).unwrap();
        let (caret, right) = document
            .preview(&plan, &library, |view| {
                let layout = view.text_layout(page).unwrap();
                let shown = format!("Welcome to Tintenfisch{suffix}");
                let line = layout.lines().iter().find(|line| line_text(line) == shown).unwrap().clone();
                (plan.caret(&layout, end).unwrap(), line.right())
            })
            .unwrap();
        assert!((caret.x - right).abs() < 1e-6, "caret {} short of {right} with {suffix:?}", caret.x);
    }
}
