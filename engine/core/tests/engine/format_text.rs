use crate::support;

use support::fixtures;
use support::rendering::library;
use tintenfisch::document::Document;
use tintenfisch::edit::{FontPick, ParagraphEdit, StyleChange, TextStyle};
use tintenfisch::fonts::{FallbackFamily, FontLibrary};
use tintenfisch::pages::PageId;
use tintenfisch::text::LayoutLine;

struct Desk {
    document: Document,
    library: FontLibrary,
    page: PageId,
}

impl Desk {
    fn open(bytes: Vec<u8>) -> Self {
        let library = library();
        let document = Document::open(bytes, "").unwrap();
        let page = document.view(&library).pages().unwrap()[0].id;
        Self { document, library, page }
    }

    fn lines(&self) -> Vec<LayoutLine> {
        self.document.view(&self.library).text_layout(self.page).unwrap().lines().to_vec()
    }

    fn line_index(&self, text: &str) -> usize {
        self.lines()
            .iter()
            .position(|line| text_of(line) == text)
            .unwrap_or_else(|| panic!("no {text:?} in {:?}", self.lines().iter().map(text_of).collect::<Vec<_>>()))
    }

    fn paragraph(&self, line_text: &str) -> (usize, String) {
        let paragraph = self.document.paragraph_text(self.page, self.line_index(line_text), &self.library).unwrap();
        (paragraph.line, paragraph.clusters.concat())
    }

    fn restyle(&mut self, line_text: &str, (old, new): (&str, &str), styles: Vec<StyleChange>) {
        let (line, text) = self.paragraph(line_text);
        let edit = ParagraphEdit::new(self.page, line, text.replacen(old, new, 1)).with_styles(styles);
        let plan = self.document.plan_edit(&edit, &self.library).unwrap();
        self.document.apply(&plan, "").unwrap();
    }

    fn fonts_of(&self, text: &str) -> Vec<(String, String, f64)> {
        let view = self.document.view(&self.library);
        let line = &self.lines()[self.line_index(text)];
        let page_text =
            tintenfisch::edit::PageText::extract(self.document.store(), &Default::default(), &self.library, self.page)
                .unwrap();
        let _ = view;
        line.clusters
            .iter()
            .filter_map(|cluster| {
                let origin = cluster.origins.first()?;
                let (record, _) = page_text.glyph(origin).ok()?;
                Some((
                    cluster.text.clone(),
                    record
                        .font_name
                        .as_ref()
                        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                        .unwrap_or_default(),
                    record.state.size * record.display_scale(),
                ))
            })
            .collect()
    }

    fn render(&self) -> (u32, Vec<u8>) {
        let raster = self.document.view(&self.library).render(self.page, 1.0, None).unwrap();
        (raster.width, raster.rgba)
    }
}

fn text_of(line: &LayoutLine) -> String {
    line.clusters.iter().map(|cluster| cluster.text.as_str()).collect()
}

fn change(start: usize, end: usize, style: TextStyle) -> StyleChange {
    StyleChange { start, end, style }
}

#[test]
fn a_word_turns_bold_red_and_larger_while_the_rest_keeps_its_font() {
    let original = fixtures::editing_features();
    let mut desk = Desk::open(original.clone());
    let before = desk.fonts_of("Plain line to edit");
    let bold = TextStyle { bold: Some(true), ..TextStyle::default() };
    desk.restyle("Plain line to edit", ("", ""), vec![change(6, 10, bold)]);
    let after = desk.fonts_of("Plain line to edit");
    for (index, ((text, font, size), (_, old_font, old_size))) in after.iter().zip(&before).enumerate() {
        if (6..10).contains(&index) {
            assert!(font.starts_with("TfFallback") && font != old_font, "{text} is bold");
        } else {
            assert_eq!(font, old_font, "{text} keeps its font");
        }
        assert!((size - old_size).abs() < 1e-6);
    }
    let red = TextStyle { color: Some([1.0, 0.0, 0.0]), size: Some(18.0), ..TextStyle::default() };
    desk.restyle("Plain line to edit", ("", ""), vec![change(0, 5, red)]);
    let sized = desk.fonts_of("Plain line to edit");
    assert!(sized[..5].iter().all(|(_, _, size)| (size - 18.0).abs() < 1e-6), "{sized:?}");
    assert!(sized[6..].iter().all(|(_, _, size)| (size - 12.0).abs() < 1e-6), "{sized:?}");
    let (width, rgba) = desk.render();
    let line = &desk.lines()[desk.line_index("Plain line to edit")];
    let plain = &line.clusters[..5];
    let (x0, x1) = (plain[0].left as usize, plain[4].right as usize);
    let reddish = (line.top as usize..line.bottom as usize)
        .flat_map(|y| (x0..x1).map(move |x| (y * width as usize + x) * 4))
        .filter(|index| rgba[*index] > 150 && rgba[index + 1] < 90 && rgba[index + 2] < 90)
        .count();
    assert!(reddish > 20, "Plain is painted red: {reddish}");
    let rest = &line.clusters[6..];
    let dark_rest = (line.top as usize..line.bottom as usize)
        .flat_map(|y| {
            (rest[0].left as usize..rest.last().unwrap().right as usize).map(move |x| (y * width as usize + x) * 4)
        })
        .filter(|index| rgba[*index] > 150 && rgba[index + 1] < 90)
        .count();
    assert_eq!(dark_rest, 0, "the color goes back to the text's own after the red word");
    let reopened = Desk::open(desk.document.save(false).unwrap());
    assert_eq!(reopened.fonts_of("Plain line to edit").len(), sized.len());
    while desk.document.undo().is_some() {}
    assert!(desk.document.save(false).unwrap() == original);
}

#[test]
fn text_takes_a_font_of_the_page_or_a_bundled_family_and_typing_inside_keeps_the_style() {
    let mut desk = Desk::open(fixtures::reflow_features());
    let flow = desk.fonts_of("widest of them and flow on when");
    let (_, bold_name, _) = flow.iter().find(|(text, _, _)| text == "f").cloned().unwrap();
    let document = TextStyle {
        font: Some(FontPick::Document(tintenfisch::syntax::Name::from(bold_name.as_str()))),
        ..TextStyle::default()
    };
    desk.restyle("Lines of one block wrap at the", ("", ""), vec![change(0, 5, document)]);
    let lines = desk.fonts_of("Lines of one block wrap at the");
    assert!(lines[..5].iter().all(|(_, font, _)| *font == bold_name), "{lines:?}");
    let serif = TextStyle { font: Some(FontPick::Bundled(FallbackFamily::Serif)), ..TextStyle::default() };
    let (_, text) = desk.paragraph("text grows.");
    let start = text.find("text grows.").unwrap();
    desk.restyle("text grows.", ("", ""), vec![change(start, start + 4, serif.clone())]);
    let grown = desk.fonts_of("text grows.");
    let serif_font = grown[0].1.clone();
    assert!(
        serif_font.starts_with("TfFallback") && grown[..4].iter().all(|(_, font, _)| *font == serif_font),
        "{grown:?}"
    );
    desk.restyle("text grows.", ("text grows.", "texts grows."), vec![change(start, start + 5, serif)]);
    let typed = desk.fonts_of("texts grows.");
    let joined = &typed[0].1;
    assert!(
        joined.starts_with("TfFallback") && typed[..5].iter().all(|(_, font, _)| font == joined),
        "the typed s joins the serif word: {typed:?}"
    );
    assert!(typed[5..].iter().all(|(_, font, _)| font == "F1"));
}

#[test]
#[ignore = "writes formatted text to PREVIEW_DIR for comparing with other viewers"]
fn write_formatted_preview() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let mut desk = Desk::open(fixtures::reflow_features());
    let (_, text) = desk.paragraph("Lines of one block wrap at the");
    let styles = vec![
        change(0, 5, TextStyle { bold: Some(true), ..TextStyle::default() }),
        change(13, 18, TextStyle { color: Some([0.85, 0.1, 0.1]), size: Some(16.0), ..TextStyle::default() }),
        change(
            24,
            30,
            TextStyle {
                font: Some(FontPick::Bundled(FallbackFamily::Serif)),
                italic: Some(true),
                ..TextStyle::default()
            },
        ),
    ];
    let _ = text;
    desk.restyle("Lines of one block wrap at the", ("", ""), styles);
    std::fs::write(format!("{directory}/formatted.pdf"), desk.document.save(false).unwrap()).unwrap();
    let raster = desk.document.view(&desk.library).render(desk.page, 2.0, None).unwrap();
    std::fs::write(format!("{directory}/formatted.png"), support::rendering::to_png(&raster)).unwrap();
}

#[test]
fn fonts_are_named_after_their_embedded_program_or_the_pdf_but_never_after_a_placeholder() {
    let desk = Desk::open(fixtures::embedded_font_names());
    type Described = (String, Option<String>, String, bool, Option<u16>);
    let mut fonts: Vec<Described> = desk
        .document
        .view(&desk.library)
        .fonts(desk.page)
        .unwrap()
        .into_iter()
        .map(|font| {
            let name = String::from_utf8_lossy(font.name.as_bytes()).into_owned();
            (name, font.family, font.base_font, font.embedded, font.weight)
        })
        .collect();
    fonts.sort();
    assert_eq!(
        fonts,
        [
            ("F1".to_string(), Some("Liberation Sans Bold".to_string()), "AAAAAA+false".to_string(), true, Some(700)),
            ("F2".to_string(), Some("Helvetica".to_string()), "BAAAAA+Helvetica".to_string(), false, None),
            ("F3".to_string(), None, "CAAAAA+false".to_string(), false, None),
        ]
    );
}

#[test]
fn the_host_learns_the_style_of_every_cluster_the_fonts_of_the_page_and_where_a_selection_is() {
    let mut desk = Desk::open(fixtures::reflow_features());
    let mut names: Vec<(String, Option<String>)> = desk
        .document
        .view(&desk.library)
        .fonts(desk.page)
        .unwrap()
        .into_iter()
        .map(|font| (String::from_utf8_lossy(font.name.as_bytes()).into_owned(), font.family))
        .collect();
    names.sort();
    assert_eq!(
        names,
        [("F1".to_string(), Some("Helvetica".to_string())), ("F2".to_string(), Some("Helvetica-Bold".to_string()))]
    );
    let line = desk.line_index("widest of them and flow on when");
    let paragraph = desk.document.paragraph_text(desk.page, line, &desk.library).unwrap();
    let flow = paragraph.clusters.concat().find("flow").unwrap();
    assert!(paragraph.styles[flow].bold && !paragraph.styles[flow - 2].bold);
    assert_eq!(paragraph.styles[flow].font, "Helvetica-Bold");
    assert!((paragraph.styles[0].size - 12.0).abs() < 1e-6 && paragraph.styles[0].color == Some([0.0; 3]));
    desk.restyle(
        "Lines of one block wrap at the",
        ("", ""),
        vec![change(0, 5, TextStyle { color: Some([0.2, 0.4, 0.6]), ..TextStyle::default() })],
    );
    let paragraph = desk
        .document
        .paragraph_text(desk.page, desk.line_index("Lines of one block wrap at the"), &desk.library)
        .unwrap();
    assert_eq!(paragraph.styles[0].color, Some([0.2, 0.4, 0.6]));
    let edit = ParagraphEdit::new(desk.page, paragraph.line, paragraph.clusters.concat());
    let plan = desk.document.plan_edit(&edit, &desk.library).unwrap();
    let layout = desk.document.view(&desk.library).text_layout(desk.page).unwrap();
    let first_line = paragraph.line_starts[1];
    let boxes = plan.selection(&layout, 6, first_line + 6);
    assert_eq!(boxes.len(), 2, "the selection reaches into the second line: {boxes:?}");
    assert!(boxes[0].rect.x0 > boxes[1].rect.x0 && boxes[0].rect.y1 <= boxes[1].rect.y0 + 1.0);
}
