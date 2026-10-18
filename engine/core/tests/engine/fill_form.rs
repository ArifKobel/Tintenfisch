use crate::support;

use support::fixtures;
use support::rendering::library;
use tintenfisch::document::{Document, FieldInput};
use tintenfisch::fonts::FontLibrary;
use tintenfisch::forms::{Field, FieldKind, FieldValue};
use tintenfisch::pages::PageId;
use tintenfisch::syntax::{Name, ObjId, Object};

struct Form {
    document: Document,
    library: FontLibrary,
    page: PageId,
}

impl Form {
    fn open(bytes: Vec<u8>) -> Self {
        let library = library();
        let document = Document::open(bytes, "").unwrap();
        let page = document.view(&library).pages().unwrap()[0].id;
        Self { document, library, page }
    }

    fn field(&self, name: &str) -> Field {
        self.document
            .fields()
            .unwrap()
            .into_iter()
            .find(|field| field.name == name)
            .unwrap_or_else(|| panic!("no field {name}"))
    }

    fn fill(&mut self, name: &str, input: FieldInput) {
        let field = self.field(name);
        self.document.set_field(field.id, input, &self.library).unwrap();
    }

    fn render(&self) -> (u32, Vec<u8>) {
        let raster = self.document.view(&self.library).render(self.page, 2.0, None).unwrap();
        (raster.width, raster.rgba)
    }

    fn appearance_text(&self, widget: ObjId) -> String {
        let store = self.document.store();
        let dict = store.object(widget).unwrap().as_dict().cloned().unwrap();
        let normal = store.resolve(dict.get("AP").unwrap()).unwrap().as_dict().unwrap().get("N").cloned().unwrap();
        let stream = store.object(normal.as_reference().unwrap()).unwrap();
        String::from_utf8_lossy(&store.decode_stream(stream.as_stream().unwrap()).unwrap().data).into_owned()
    }

    fn entry(&self, id: ObjId, key: &str) -> Option<Object> {
        self.document.store().object(id).unwrap().as_dict().unwrap().get(key).cloned()
    }
}

fn changed_only_inside(before: &(u32, Vec<u8>), after: &[u8], rect: tintenfisch::geometry::Rect) -> bool {
    let (width, before) = before;
    let mut changed = false;
    for (index, (a, b)) in before.chunks(4).zip(after.chunks(4)).enumerate() {
        if a != b {
            changed = true;
            let (x, y) = ((index as u32 % width) as f64 / 2.0, 400.0 - (index as u32 / width) as f64 / 2.0);
            if x < rect.x0 - 1.0 || x > rect.x1 + 1.0 || y < rect.y0 - 1.0 || y > rect.y1 + 1.0 {
                return false;
            }
        }
    }
    changed
}

#[test]
fn the_field_tree_reads_with_inherited_attributes_and_full_names() {
    let form = Form::open(fixtures::fillable_form());
    let fields = form.document.fields().unwrap();
    let names: Vec<&str> = fields.iter().map(|field| field.name.as_str()).collect();
    assert_eq!(names, ["person.first", "person.last", "notes", "zip", "agree", "color", "language", "fruits", "id"]);
    assert_eq!(form.field("person.first").value, FieldValue::Text("Ada".to_string()));
    assert!(matches!(form.field("notes").kind, FieldKind::Text { multiline: true, .. }));
    assert!(matches!(form.field("zip").kind, FieldKind::Text { comb: Some(5), .. }));
    assert_eq!(form.field("agree").value, FieldValue::State(None));
    assert_eq!(form.field("color").value, FieldValue::State(Some(Name::from("Red"))));
    assert_eq!(form.field("color").widgets.len(), 2);
    assert_eq!(form.field("language").value, FieldValue::Chosen(vec!["en".to_string()]));
    assert!(matches!(form.field("language").kind, FieldKind::Choice { combo: true, .. }));
    assert_eq!(form.field("fruits").value, FieldValue::Chosen(vec!["Banana".to_string()]));
    assert!(form.field("id").read_only);
    assert_eq!(
        form.field("notes").default_appearance.as_deref(),
        Some(&b"/Helv 0 Tf 0 g"[..]),
        "inherited from the form"
    );
}

#[test]
fn a_field_without_an_appearance_still_shows_its_value() {
    let form = Form::open(fixtures::fillable_form());
    let blank = Form::open(fixtures::fillable_form().replace_value(b"/V (Ada)", b"/V ()"));
    let field = form.field("person.first");
    assert!(changed_only_inside(&blank.render(), &form.render().1, field.widgets[0].rect), "Ada is drawn in her field");
}

#[test]
fn text_fills_in_its_own_widget_saves_and_undoes() {
    let original = fixtures::fillable_form();
    let mut form = Form::open(original.clone());
    let before = form.render();
    let field = form.field("person.last");
    form.fill("person.last", FieldInput::Text("Lovelace".to_string()));
    assert_eq!(form.field("person.last").value, FieldValue::Text("Lovelace".to_string()));
    assert!(form.appearance_text(field.widgets[0].id).contains("(Lovelace) Tj"));
    let after = form.render();
    assert!(changed_only_inside(&before, &after.1, field.widgets[0].rect));
    let first = form.field("person.first").widgets[0].id;
    assert!(form.appearance_text(first).contains("(Ada) Tj"), "a field that had no appearance gets one written");
    let reopened = Form::open(form.document.save(false).unwrap());
    assert_eq!(reopened.field("person.last").value, FieldValue::Text("Lovelace".to_string()));
    assert!(reopened.render() == after, "the value looks the same after saving");
    form.document.undo().unwrap();
    assert!(form.document.save(false).unwrap() == original);
}

#[test]
fn long_text_wraps_in_a_multiline_field_and_a_comb_takes_one_character_per_cell() {
    let mut form = Form::open(fixtures::fillable_form());
    let notes = form.field("notes").widgets[0].id;
    form.fill(
        "notes",
        FieldInput::Text("A long note that cannot possibly fit on one single line of this narrow field".to_string()),
    );
    assert!(form.appearance_text(notes).matches(" Tj").count() >= 3, "{}", form.appearance_text(notes));
    let zip = form.field("zip").widgets[0].id;
    form.fill("zip", FieldInput::Text("1011599".to_string()));
    assert_eq!(form.field("zip").value, FieldValue::Text("10115".to_string()), "the comb keeps its length");
    assert_eq!(form.appearance_text(zip).matches(" Tj").count(), 5);
}

#[test]
fn checkboxes_and_radio_buttons_switch_their_states() {
    let mut form = Form::open(fixtures::fillable_form());
    let agree = form.field("agree");
    form.fill("agree", FieldInput::Checked(true));
    assert_eq!(form.field("agree").value, FieldValue::State(Some(Name::from("Yes"))));
    assert_eq!(form.entry(agree.widgets[0].id, "AS"), Some(Object::Name(Name::from("Yes"))));
    let color = form.field("color");
    form.fill("color", FieldInput::Radio(color.widgets[1].id));
    assert_eq!(form.field("color").value, FieldValue::State(Some(Name::from("Blue"))));
    assert_eq!(form.entry(color.widgets[0].id, "AS"), Some(Object::Name(Name::from("Off"))));
    assert_eq!(form.entry(color.widgets[1].id, "AS"), Some(Object::Name(Name::from("Blue"))));
    form.fill("agree", FieldInput::Checked(false));
    assert_eq!(form.field("agree").value, FieldValue::State(None));
}

#[test]
fn choices_show_their_labels_and_lists_mark_what_is_chosen() {
    let mut form = Form::open(fixtures::fillable_form());
    let language = form.field("language").widgets[0].id;
    form.fill("language", FieldInput::Choices(vec!["de".to_string()]));
    assert_eq!(form.field("language").value, FieldValue::Chosen(vec!["de".to_string()]));
    assert!(form.appearance_text(language).contains("(Deutsch) Tj"));
    let fruits = form.field("fruits").widgets[0].id;
    form.fill("fruits", FieldInput::Choices(vec!["Cherry".to_string()]));
    let content = form.appearance_text(fruits);
    assert!(content.contains("(Apple) Tj") && content.contains("(Cherry) Tj") && content.contains("0.6 0.75 0.94 rg"));
}

#[test]
fn a_read_only_field_refuses_input() {
    let mut form = Form::open(fixtures::fillable_form());
    let id = form.field("id").id;
    let error = form.document.set_field(id, FieldInput::Text("B-1".to_string()), &form.library).unwrap_err();
    assert_eq!(error.kind, tintenfisch::error::ErrorKind::Unsupported);
}

trait ReplaceValue {
    fn replace_value(self, from: &[u8], to: &[u8]) -> Vec<u8>;
}

impl ReplaceValue for Vec<u8> {
    fn replace_value(self, from: &[u8], to: &[u8]) -> Vec<u8> {
        let mut padded = to.to_vec();
        padded.resize(from.len(), b' ');
        let at = self.windows(from.len()).position(|window| window == from).unwrap();
        let mut bytes = self;
        bytes[at..at + from.len()].copy_from_slice(&padded);
        bytes
    }
}

#[test]
#[ignore = "writes a filled form to PREVIEW_DIR for comparing with other viewers"]
fn write_filled_preview() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let mut form = Form::open(fixtures::fillable_form());
    std::fs::write(
        format!("{directory}/form-blank.png"),
        support::rendering::to_png(&form.document.view(&form.library).render(form.page, 2.0, None).unwrap()),
    )
    .unwrap();
    form.fill("person.last", FieldInput::Text("Lovelace".to_string()));
    form.fill(
        "notes",
        FieldInput::Text(
            "Wrote the first program for the Analytical Engine, long before any computer existed.".to_string(),
        ),
    );
    form.fill("zip", FieldInput::Text("10115".to_string()));
    form.fill("agree", FieldInput::Checked(true));
    let color = form.field("color").widgets[1].id;
    form.fill("color", FieldInput::Radio(color));
    form.fill("language", FieldInput::Choices(vec!["de".to_string()]));
    form.fill("fruits", FieldInput::Choices(vec!["Cherry".to_string()]));
    std::fs::write(format!("{directory}/form-filled.pdf"), form.document.save(false).unwrap()).unwrap();
    std::fs::write(
        format!("{directory}/form-filled.png"),
        support::rendering::to_png(&form.document.view(&form.library).render(form.page, 2.0, None).unwrap()),
    )
    .unwrap();
}

#[test]
fn every_fill_is_one_undo_step() {
    let original = fixtures::fillable_form();
    let mut form = Form::open(original.clone());
    form.fill("person.last", FieldInput::Text("Lovelace".to_string()));
    form.fill("agree", FieldInput::Checked(true));
    let blue = form.field("color").widgets[1].id;
    form.fill("color", FieldInput::Radio(blue));
    form.fill("language", FieldInput::Choices(vec!["de".to_string()]));
    let labels: Vec<String> = std::iter::from_fn(|| form.document.undo().map(|undone| undone.label)).collect();
    assert_eq!(labels.len(), 4, "{labels:?}");
    assert_eq!(form.field("agree").value, FieldValue::State(None));
    assert!(form.document.save(false).unwrap() == original);
}
