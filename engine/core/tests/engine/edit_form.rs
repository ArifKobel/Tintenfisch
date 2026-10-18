use crate::support;

use support::fixtures;
use support::rendering::library;
use tintenfisch::document::Document;
use tintenfisch::edit::ParagraphEdit;
use tintenfisch::fonts::FontLibrary;
use tintenfisch::pages::PageId;
use tintenfisch::syntax::ObjId;
use tintenfisch::text::LayoutLine;

const SHARED_FORM: ObjId = ObjId { num: 6, generation: 0 };
const OUTER_FORM: ObjId = ObjId { num: 7, generation: 0 };
const INNER_FORM: ObjId = ObjId { num: 8, generation: 0 };

struct Forms {
    document: Document,
    library: FontLibrary,
    pages: Vec<PageId>,
}

impl Forms {
    fn open() -> Self {
        let library = library();
        let document = Document::open(fixtures::form_features(), "").unwrap();
        let pages = document.view(&library).pages().unwrap().iter().map(|page| page.id).collect();
        Self { document, library, pages }
    }

    fn lines(&self, page: usize) -> Vec<LayoutLine> {
        self.document.view(&self.library).text_layout(self.pages[page]).unwrap().lines().to_vec()
    }

    fn texts(&self, page: usize) -> Vec<String> {
        self.lines(page).iter().map(line_text).collect()
    }

    fn line(&self, page: usize, text: &str) -> LayoutLine {
        self.lines(page).into_iter().find(|line| line_text(line) == text).unwrap_or_else(|| panic!("no line {text:?}"))
    }

    fn edit(&mut self, page: usize, line: &str, text: &str) -> Result<(), tintenfisch::Error> {
        let index = self.texts(page).iter().position(|candidate| candidate == line).unwrap();
        let edit = ParagraphEdit::new(self.pages[page], index, text);
        self.document.edit(&edit, &self.library, "")
    }

    fn render(&self, page: usize) -> Vec<u8> {
        self.document.view(&self.library).render(self.pages[page], 1.0, None).unwrap().rgba
    }
}

fn line_text(line: &LayoutLine) -> String {
    line.clusters.iter().map(|cluster| cluster.text.as_str()).collect()
}

#[test]
fn text_in_a_form_drawn_by_this_page_alone_is_edited_in_place() {
    let mut forms = Forms::open();
    let before = forms.line(0, "Outer form line");
    forms.edit(0, "Outer form line", "Outer form line edited").unwrap();
    let after = forms.line(0, "Outer form line edited");
    assert!((after.left() - before.left()).abs() < 1e-6 && (after.baseline - before.baseline).abs() < 1e-6);
    assert!(changed(&forms.document, OUTER_FORM), "the form itself was rewritten");
    assert!(forms.texts(0).contains(&"Inner form line".to_string()), "the nested form is still drawn");
}

#[test]
fn text_in_a_nested_form_is_edited_where_it_is_drawn() {
    let mut forms = Forms::open();
    forms.edit(0, "Inner form line", "Inner form line changed Ω").unwrap();
    assert!(forms.texts(0).contains(&"Inner form line changed Ω".to_string()));
    assert!(changed(&forms.document, INNER_FORM));
    let saved = forms.document.save(false).unwrap();
    let reopened = Document::open(saved, "").unwrap();
    let page = reopened.view(&forms.library).pages().unwrap()[0].id;
    let lines = reopened.view(&forms.library).text_layout(page).unwrap().lines().to_vec();
    assert!(
        lines.iter().any(|line| line_text(line) == "Inner form line changed Ω"),
        "the fallback font reaches the form"
    );
}

#[test]
fn a_form_other_pages_draw_too_is_copied_for_this_page() {
    let mut forms = Forms::open();
    let other = forms.render(1);
    forms.edit(0, "Shared form text", "Shared form text, edited here").unwrap();
    assert!(forms.texts(0).contains(&"Shared form text, edited here".to_string()));
    assert_eq!(forms.texts(1), ["Shared form text"]);
    assert_eq!(forms.render(1), other, "the other page still shows the original");
    assert!(!changed(&forms.document, SHARED_FORM), "the shared form itself stays untouched");
}

#[test]
fn a_form_drawn_twice_on_the_page_is_refused() {
    let mut forms = Forms::open();
    let error = forms.edit(2, "Drawn twice", "Drawn once").unwrap_err();
    assert!(error.to_string().contains("more than once"), "{error}");
}

#[test]
fn moving_text_in_a_form_moves_it_on_the_page_and_undo_restores_the_file() {
    let original = fixtures::form_features();
    let mut forms = Forms::open();
    let before = forms.line(0, "Outer form line");
    let index = forms.texts(0).iter().position(|line| line == "Outer form line").unwrap();
    let edit = ParagraphEdit::new(forms.pages[0], index, "Outer form line").moved_by(15.0, 25.0);
    let previewed = forms
        .document
        .preview_edit(&edit, &forms.library, |view| view.render(edit.page, 1.0, None).unwrap().rgba)
        .unwrap();
    forms.document.edit(&edit, &forms.library, "").unwrap();
    assert!(forms.render(0) == previewed, "preview and commit differ");
    let after = forms.line(0, "Outer form line");
    assert!(
        (after.left() - before.left() - 15.0).abs() < 1e-6 && (after.baseline - before.baseline - 25.0).abs() < 1e-6
    );
    let touched = forms.document.undo().unwrap().touched;
    assert_eq!(forms.document.pages_touching(&touched, &forms.library).unwrap(), [forms.pages[0]]);
    assert!(forms.document.save(false).unwrap() == original);
}

fn changed(document: &Document, id: ObjId) -> bool {
    document.store().changes().iter().any(|(num, _)| *num == id.num)
}

#[test]
fn every_line_of_the_page_edits_within_its_own_content_however_the_layout_groups_it() {
    let mut forms = Forms::open();
    for line in forms.texts(0) {
        let paragraph = forms
            .document
            .paragraph_text(
                forms.pages[0],
                forms.texts(0).iter().position(|text| *text == line).unwrap(),
                &forms.library,
            )
            .unwrap();
        let text = paragraph.clusters.concat();
        let index = paragraph.line;
        let edit = ParagraphEdit::new(forms.pages[0], index, format!("{text}x"));
        forms.document.edit(&edit, &forms.library, "").unwrap_or_else(|error| panic!("{line:?}: {error}"));
        assert!(forms.texts(0).iter().any(|shown| shown.ends_with('x')), "{line:?}");
        forms.document.undo().unwrap();
    }
}
