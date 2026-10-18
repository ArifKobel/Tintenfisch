use crate::support;

use support::fixtures;
use support::rendering::library;
use tintenfisch::document::{Document, FieldInput};
use tintenfisch::error::ErrorKind;
use tintenfisch::fonts::FontLibrary;
use tintenfisch::forms::{Field, FieldKind, FieldProperties, FieldValue, NewField, NewFieldKind};
use tintenfisch::geometry::Rect;
use tintenfisch::pages::PageId;
use tintenfisch::syntax::{Name, ObjId, Object};

struct Designer {
    document: Document,
    library: FontLibrary,
    page: PageId,
}

impl Designer {
    fn open() -> Self {
        let library = library();
        let document = Document::open(fixtures::editing_features(), "").unwrap();
        let page = document.view(&library).pages().unwrap()[0].id;
        Self { document, library, page }
    }

    fn add(&mut self, kind: NewFieldKind, name: &str, rect: Rect) -> ObjId {
        let field = NewField { kind, name: name.to_string(), rect };
        self.document.add_field(self.page, &field, &self.library).unwrap()
    }

    fn field(&self, id: ObjId) -> Field {
        self.document.fields().unwrap().into_iter().find(|field| field.id == id).unwrap()
    }

    fn render(&self) -> Vec<u8> {
        self.document.view(&self.library).render(self.page, 1.0, None).unwrap().rgba
    }

    fn annotations(&self) -> Vec<ObjId> {
        let page = self.document.store().object(self.page.0).unwrap();
        page.as_dict()
            .unwrap()
            .get("Annots")
            .and_then(Object::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(Object::as_reference)
            .collect()
    }
}

fn properties(name: &str) -> FieldProperties {
    FieldProperties { name: name.to_string(), required: false, read_only: false, multiline: false, options: Vec::new() }
}

#[test]
fn a_text_field_creates_the_form_shows_its_box_saves_and_undoes() {
    let original = fixtures::editing_features();
    let mut designer = Designer::open();
    let before = designer.render();
    let id = designer.add(NewFieldKind::Text { multiline: false }, "Name", Rect::new(200.0, 30.0, 360.0, 50.0));
    let field = designer.field(id);
    assert_eq!(
        (field.name.as_str(), &field.kind),
        ("Name", &FieldKind::Text { multiline: false, password: false, comb: None })
    );
    assert!(field.widgets[0].dict.get("AP").is_some(), "the new widget draws itself");
    assert_ne!(designer.render(), before);
    assert_eq!(designer.annotations(), [id]);
    let saved = designer.document.save(false).unwrap();
    let reopened = Document::open(saved, "").unwrap();
    assert_eq!(reopened.fields().unwrap().len(), 1);
    designer.document.set_field(id, FieldInput::Text("Grace".to_string()), &designer.library).unwrap();
    assert_eq!(designer.field(id).value, FieldValue::Text("Grace".to_string()));
    while designer.document.undo().is_some() {}
    assert!(designer.document.save(false).unwrap() == original);
}

#[test]
fn names_stay_unique() {
    let mut designer = Designer::open();
    let first = designer.add(NewFieldKind::Text { multiline: false }, "Name", Rect::new(200.0, 30.0, 360.0, 50.0));
    let second = designer.add(NewFieldKind::Text { multiline: false }, "Name", Rect::new(200.0, 60.0, 360.0, 80.0));
    assert_eq!((designer.field(first).name, designer.field(second).name), ("Name".to_string(), "Name2".to_string()));
    let error = designer.document.set_field_properties(second, &properties("Name"), &designer.library).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
}

#[test]
fn radio_buttons_of_one_name_form_a_group_with_their_own_states() {
    let mut designer = Designer::open();
    let group = designer.add(NewFieldKind::Radio, "Size", Rect::new(200.0, 30.0, 214.0, 44.0));
    assert_eq!(designer.add(NewFieldKind::Radio, "Size", Rect::new(230.0, 30.0, 244.0, 44.0)), group);
    designer.add(NewFieldKind::Radio, "Size", Rect::new(260.0, 30.0, 274.0, 44.0));
    let field = designer.field(group);
    assert_eq!(field.kind, FieldKind::Radio);
    let states: Vec<Option<Name>> = field.widgets.iter().map(|widget| widget.on_state.clone()).collect();
    assert_eq!(states, [Some(Name::from("Choice1")), Some(Name::from("Choice2")), Some(Name::from("Choice3"))]);
    designer.document.set_field(group, FieldInput::Radio(field.widgets[1].id), &designer.library).unwrap();
    let lit: Vec<bool> = designer
        .field(group)
        .widgets
        .iter()
        .map(|widget| widget.dict.name("AS").is_some_and(|state| !state.is("Off")))
        .collect();
    assert_eq!(lit, [false, true, false]);
    assert_eq!(designer.annotations().len(), 3);
}

#[test]
fn checkboxes_and_choices_take_input_once_created() {
    let mut designer = Designer::open();
    let agree = designer.add(NewFieldKind::Checkbox, "Agree", Rect::new(200.0, 30.0, 216.0, 46.0));
    designer.document.set_field(agree, FieldInput::Checked(true), &designer.library).unwrap();
    assert_eq!(designer.field(agree).value, FieldValue::State(Some(Name::from("Yes"))));
    let options = vec!["Red".to_string(), "Green".to_string()];
    let combo =
        designer.add(NewFieldKind::Combo { options: options.clone() }, "Color", Rect::new(200.0, 60.0, 300.0, 78.0));
    let list = designer.add(NewFieldKind::List { options }, "Colors", Rect::new(200.0, 90.0, 300.0, 130.0));
    assert!(matches!(designer.field(combo).kind, FieldKind::Choice { combo: true, .. }));
    assert!(matches!(designer.field(list).kind, FieldKind::Choice { combo: false, .. }));
    designer.document.set_field(combo, FieldInput::Choices(vec!["Green".to_string()]), &designer.library).unwrap();
    assert_eq!(designer.field(combo).value, FieldValue::Chosen(vec!["Green".to_string()]));
}

#[test]
fn properties_rename_flag_and_change_entries() {
    let mut designer = Designer::open();
    let notes = designer.add(NewFieldKind::Text { multiline: false }, "Notes", Rect::new(200.0, 30.0, 360.0, 90.0));
    let changed = FieldProperties {
        name: "Remarks".to_string(),
        required: true,
        read_only: false,
        multiline: true,
        options: Vec::new(),
    };
    designer.document.set_field_properties(notes, &changed, &designer.library).unwrap();
    let field = designer.field(notes);
    assert_eq!(field.name, "Remarks");
    assert!(field.required && !field.read_only);
    assert!(matches!(field.kind, FieldKind::Text { multiline: true, .. }));
    let color = designer.add(
        NewFieldKind::Combo { options: vec!["Red".to_string()] },
        "Color",
        Rect::new(200.0, 100.0, 300.0, 118.0),
    );
    let entries = FieldProperties { options: vec!["Blue".to_string(), "Gold".to_string()], ..properties("Color") };
    designer.document.set_field_properties(color, &entries, &designer.library).unwrap();
    match designer.field(color).kind {
        FieldKind::Choice { options, .. } => {
            assert_eq!(options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(), ["Blue", "Gold"])
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_moved_widget_draws_where_it_went_and_a_deleted_field_leaves_nothing() {
    let mut designer = Designer::open();
    let empty = designer.render();
    let id = designer.add(NewFieldKind::Text { multiline: false }, "Name", Rect::new(200.0, 30.0, 360.0, 50.0));
    let widget = designer.field(id).widgets[0].id;
    designer
        .document
        .move_widget(designer.page, widget, Rect::new(220.0, 300.0, 300.0, 330.0), &designer.library)
        .unwrap();
    let moved = designer.field(id).widgets[0].rect;
    assert!((moved.width() - 80.0).abs() < 1e-6 && (moved.height() - 30.0).abs() < 1e-6);
    assert!((moved.y1 - (400.0 - 300.0)).abs() < 1e-6, "display space is upside down to user space");
    let appearance = designer.field(id).widgets[0]
        .dict
        .get("AP")
        .and_then(Object::as_dict)
        .and_then(|ap| ap.get("N"))
        .and_then(Object::as_reference)
        .unwrap();
    let stream = designer.document.store().object(appearance).unwrap();
    let bbox = Rect::from_object(stream.as_dict().unwrap().get("BBox").unwrap()).unwrap();
    assert_eq!((bbox.width(), bbox.height()), (80.0, 30.0), "the appearance is drawn anew for the new size");
    let agree = designer.add(NewFieldKind::Checkbox, "Agree", Rect::new(200.0, 100.0, 216.0, 116.0));
    let box_widget = designer.field(agree).widgets[0].id;
    designer
        .document
        .move_widget(designer.page, box_widget, Rect::new(200.0, 100.0, 230.0, 130.0), &designer.library)
        .unwrap();
    let states = designer.field(agree).widgets[0]
        .dict
        .get("AP")
        .and_then(Object::as_dict)
        .and_then(|ap| ap.get("N"))
        .and_then(Object::as_dict)
        .cloned()
        .unwrap();
    let on = designer.document.store().object(states.get("Yes").and_then(Object::as_reference).unwrap()).unwrap();
    assert_eq!(
        Rect::from_object(on.as_dict().unwrap().get("BBox").unwrap()).map(|bbox| bbox.width()),
        Some(30.0),
        "a resized checkbox is drawn anew"
    );
    designer.document.delete_field(agree).unwrap();
    let group = designer.add(NewFieldKind::Radio, "Size", Rect::new(200.0, 60.0, 214.0, 74.0));
    designer.add(NewFieldKind::Radio, "Size", Rect::new(230.0, 60.0, 244.0, 74.0));
    designer.document.delete_field(group).unwrap();
    designer.document.delete_field(id).unwrap();
    assert!(designer.document.fields().unwrap().is_empty());
    assert!(designer.annotations().is_empty());
    assert!(designer.render() == empty);
}

#[test]
#[ignore = "writes a designed form to PREVIEW_DIR for comparing with other viewers"]
fn write_designed_preview() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let mut designer = Designer::open();
    let name = designer.add(NewFieldKind::Text { multiline: false }, "Name", Rect::new(200.0, 30.0, 380.0, 50.0));
    let notes = designer.add(NewFieldKind::Text { multiline: true }, "Notes", Rect::new(200.0, 60.0, 380.0, 120.0));
    let agree = designer.add(NewFieldKind::Checkbox, "Agree", Rect::new(200.0, 130.0, 216.0, 146.0));
    let size = designer.add(NewFieldKind::Radio, "Size", Rect::new(230.0, 130.0, 244.0, 144.0));
    designer.add(NewFieldKind::Radio, "Size", Rect::new(260.0, 130.0, 274.0, 144.0));
    let color = designer.add(
        NewFieldKind::Combo { options: vec!["Red".to_string(), "Green".to_string()] },
        "Color",
        Rect::new(200.0, 160.0, 300.0, 178.0),
    );
    let fill = |designer: &mut Designer, id, input| designer.document.set_field(id, input, &designer.library).unwrap();
    fill(&mut designer, name, FieldInput::Text("Grace Hopper".to_string()));
    fill(&mut designer, notes, FieldInput::Text("Found the first actual bug, a moth in the relay.".to_string()));
    fill(&mut designer, agree, FieldInput::Checked(true));
    let second = designer.field(size).widgets[1].id;
    fill(&mut designer, size, FieldInput::Radio(second));
    fill(&mut designer, color, FieldInput::Choices(vec!["Green".to_string()]));
    std::fs::write(format!("{directory}/designed.pdf"), designer.document.save(false).unwrap()).unwrap();
    let raster = designer.document.view(&designer.library).render(designer.page, 2.0, None).unwrap();
    std::fs::write(format!("{directory}/designed.png"), support::rendering::to_png(&raster)).unwrap();
}
