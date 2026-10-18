use std::collections::HashSet;

use crate::annotations::{annotation_list, write_annotation_list};
use crate::document::Document;
use crate::error::{Error, Result};
use crate::fonts::FontLibrary;
use crate::forms::{FieldFlags, FieldKind, FieldProperties, FieldTree, NewField, NewFieldKind, refresh_appearances};
use crate::pages::{PageId, PageTree};
use crate::render::DisplaySpace;
use crate::store::Transaction;
use crate::syntax::{Dict, Name, ObjId, Object, encode_text};

const FORM_FONT: &str = "Helv";
const FORM_APPEARANCE: &[u8] = b"/Helv 0 Tf 0 g";
const PRINT: i64 = 4;
const BORDER: [f64; 3] = [0.4, 0.4, 0.4];

impl Document {
    pub fn add_field(&mut self, page: PageId, field: &NewField, library: &FontLibrary) -> Result<ObjId> {
        let page = PageTree::new(&self.store)
            .pages()?
            .into_iter()
            .find(|candidate| candidate.id == page)
            .ok_or_else(|| Error::new(format!("no page {page}")))?;
        let rect = DisplaySpace::of(&page)?.rect_to_user(field.rect);
        let existing = self.fields()?;
        let group = match field.kind {
            NewFieldKind::Radio => existing
                .iter()
                .find(|candidate| candidate.name == field.name && candidate.kind == FieldKind::Radio)
                .cloned(),
            _ => None,
        };
        let taken: HashSet<String> = existing.iter().map(|candidate| candidate.name.clone()).collect();
        let name = if group.is_some() { field.name.clone() } else { unique_name(&field.name, &taken) };
        let fonts = &self.caches.fonts;
        self.store.transact("Add field", "", |transaction| {
            let form = ensure_form(transaction)?;
            let mut widget: Dict = [
                ("Type", Object::Name(Name::from("Annot"))),
                ("Subtype", Object::Name(Name::from("Widget"))),
                ("Rect", rect.to_object()),
                ("P", Object::Ref(page.id.0)),
                ("F", Object::Int(PRINT)),
                (
                    "MK",
                    Object::Dict(
                        [(Name::from("BC"), numbers(&BORDER)), (Name::from("BG"), numbers(&[1.0, 1.0, 1.0]))]
                            .into_iter()
                            .collect(),
                    ),
                ),
            ]
            .into_iter()
            .map(|(key, value)| (Name::from(key), value))
            .collect();
            let field_id = match (&field.kind, &group) {
                (NewFieldKind::Radio, Some(group)) => {
                    widget.insert("Parent", Object::Ref(group.id));
                    let widget_id = transaction.allocate(Object::Dict(widget));
                    let mut dict = transaction
                        .object(group.id)?
                        .as_dict()
                        .cloned()
                        .ok_or_else(|| Error::new("a field is not a dictionary"))?;
                    let mut kids =
                        dict.get("Kids").and_then(Object::as_array).map(<[Object]>::to_vec).unwrap_or_default();
                    kids.push(Object::Ref(widget_id));
                    dict.insert("Kids", Object::Array(kids));
                    transaction.put(group.id, Object::Dict(dict))?;
                    add_to_page(transaction, page.id.0, widget_id)?;
                    group.id
                }
                (NewFieldKind::Radio, None) => {
                    let flags = FieldFlags(0).with(FieldFlags::RADIO, true).with(FieldFlags::NO_TOGGLE_TO_OFF, true);
                    let parent: Dict = [
                        ("FT", Object::Name(Name::from("Btn"))),
                        ("T", Object::String(encode_text(&name))),
                        ("Ff", Object::Int(flags.0)),
                        ("V", Object::Name(Name::from("Off"))),
                    ]
                    .into_iter()
                    .map(|(key, value)| (Name::from(key), value))
                    .collect();
                    let parent_id = transaction.allocate(Object::Dict(parent));
                    widget.insert("Parent", Object::Ref(parent_id));
                    let widget_id = transaction.allocate(Object::Dict(widget));
                    let mut dict = transaction.object(parent_id)?.as_dict().cloned().unwrap_or_default();
                    dict.insert("Kids", Object::Array(vec![Object::Ref(widget_id)]));
                    transaction.put(parent_id, Object::Dict(dict))?;
                    add_to_form(transaction, form, parent_id)?;
                    add_to_page(transaction, page.id.0, widget_id)?;
                    parent_id
                }
                (kind, _) => {
                    widget.insert("T", Object::String(encode_text(&name)));
                    for (key, value) in field_entries(kind) {
                        widget.insert(key, value);
                    }
                    let id = transaction.allocate(Object::Dict(widget));
                    add_to_form(transaction, form, id)?;
                    add_to_page(transaction, page.id.0, id)?;
                    id
                }
            };
            refresh_appearances(transaction, fonts, library, |candidate, widget| {
                candidate.id == field_id && widget.dict.get("AP").is_none()
            })?;
            Ok(field_id)
        })
    }

    pub fn set_field_properties(
        &mut self,
        field: ObjId,
        properties: &FieldProperties,
        library: &FontLibrary,
    ) -> Result<Vec<PageId>> {
        let fields = self.fields()?;
        let current = fields
            .iter()
            .find(|candidate| candidate.id == field)
            .cloned()
            .ok_or_else(|| Error::new("no such field"))?;
        let prefix = current.name.rsplit_once('.').map(|(prefix, _)| format!("{prefix}.")).unwrap_or_default();
        let renamed = format!("{prefix}{}", properties.name);
        if properties.name.trim().is_empty() || properties.name.contains('.') {
            return Err(Error::unsupported("a field name is not empty and has no dots"));
        }
        if fields.iter().any(|other| other.id != field && other.name == renamed) {
            return Err(Error::unsupported(format!("another field is called {renamed}")));
        }
        let fonts = &self.caches.fonts;
        self.store.transact("Change field", "", |transaction| {
            let mut dict = transaction
                .object(field)?
                .as_dict()
                .cloned()
                .ok_or_else(|| Error::new("a field is not a dictionary"))?;
            let inherited = FieldTree::new(transaction.store()).flags(field)?;
            let mut flags = inherited
                .with(FieldFlags::REQUIRED, properties.required)
                .with(FieldFlags::READ_ONLY, properties.read_only);
            if matches!(current.kind, FieldKind::Text { .. }) {
                flags = flags.with(FieldFlags::MULTILINE, properties.multiline);
            }
            dict.insert("T", Object::String(encode_text(&properties.name)));
            dict.insert("Ff", Object::Int(flags.0));
            if matches!(current.kind, FieldKind::Choice { .. }) {
                dict.insert(
                    "Opt",
                    Object::Array(
                        properties.options.iter().map(|option| Object::String(encode_text(option))).collect(),
                    ),
                );
            }
            transaction.put(field, Object::Dict(dict))?;
            refresh_appearances(transaction, fonts, library, |candidate, _| candidate.id == field)
        })?;
        Ok(current.widgets.iter().filter_map(|widget| widget.page).collect())
    }

    pub fn move_widget(
        &mut self,
        page: PageId,
        widget: ObjId,
        rect: crate::geometry::Rect,
        library: &FontLibrary,
    ) -> Result<()> {
        let page = PageTree::new(&self.store)
            .pages()?
            .into_iter()
            .find(|candidate| candidate.id == page)
            .ok_or_else(|| Error::new(format!("no page {page}")))?;
        let rect = DisplaySpace::of(&page)?.rect_to_user(rect);
        let signature = self.fields()?.iter().any(|field| {
            field.kind == FieldKind::Signature && field.widgets.iter().any(|candidate| candidate.id == widget)
        });
        let fonts = &self.caches.fonts;
        self.store.transact("Move field", "", |transaction| {
            let mut dict = transaction
                .object(widget)?
                .as_dict()
                .cloned()
                .ok_or_else(|| Error::new("a widget is not a dictionary"))?;
            dict.insert("Rect", rect.to_object());
            if !signature {
                dict.remove("AP");
            }
            transaction.put(widget, Object::Dict(dict))?;
            refresh_appearances(transaction, fonts, library, |_, candidate| candidate.id == widget)
        })
    }

    pub fn delete_field(&mut self, field: ObjId) -> Result<Vec<PageId>> {
        let current = self
            .fields()?
            .into_iter()
            .find(|candidate| candidate.id == field)
            .ok_or_else(|| Error::new("no such field"))?;
        let pages: Vec<PageId> = current.widgets.iter().filter_map(|widget| widget.page).collect();
        self.store.transact("Delete field", "", |transaction| {
            let dict = transaction.object(field)?.as_dict().cloned().unwrap_or_default();
            match dict.get("Parent").and_then(Object::as_reference) {
                Some(parent) => {
                    let mut parent_dict = transaction.object(parent)?.as_dict().cloned().unwrap_or_default();
                    let kids =
                        parent_dict.get("Kids").and_then(Object::as_array).map(<[Object]>::to_vec).unwrap_or_default();
                    parent_dict.insert(
                        "Kids",
                        Object::Array(kids.into_iter().filter(|kid| kid.as_reference() != Some(field)).collect()),
                    );
                    transaction.put(parent, Object::Dict(parent_dict))?;
                }
                None => {
                    let form = ensure_form(transaction)?;
                    let mut form_dict = transaction.object(form)?.as_dict().cloned().unwrap_or_default();
                    let fields =
                        form_dict.get("Fields").and_then(Object::as_array).map(<[Object]>::to_vec).unwrap_or_default();
                    form_dict.insert(
                        "Fields",
                        Object::Array(fields.into_iter().filter(|entry| entry.as_reference() != Some(field)).collect()),
                    );
                    transaction.put(form, Object::Dict(form_dict))?;
                }
            }
            for widget in &current.widgets {
                if let Some(page) = widget.page {
                    let kept = annotation_list(transaction, page.0)?
                        .into_iter()
                        .filter(|entry| entry.as_reference() != Some(widget.id))
                        .collect();
                    write_annotation_list(transaction, page.0, kept)?;
                }
            }
            Ok(())
        })?;
        Ok(pages)
    }
}

fn field_entries(kind: &NewFieldKind) -> Vec<(&'static str, Object)> {
    let options =
        |options: &[String]| Object::Array(options.iter().map(|option| Object::String(encode_text(option))).collect());
    match kind {
        NewFieldKind::Text { multiline } => vec![
            ("FT", Object::Name(Name::from("Tx"))),
            ("Ff", Object::Int(FieldFlags(0).with(FieldFlags::MULTILINE, *multiline).0)),
            ("V", Object::String(Vec::new())),
        ],
        NewFieldKind::Checkbox => vec![("FT", Object::Name(Name::from("Btn"))), ("V", Object::Name(Name::from("Off")))],
        NewFieldKind::Combo { options: entries } => vec![
            ("FT", Object::Name(Name::from("Ch"))),
            ("Ff", Object::Int(FieldFlags::COMBO)),
            ("Opt", options(entries)),
        ],
        NewFieldKind::List { options: entries } => {
            vec![("FT", Object::Name(Name::from("Ch"))), ("Opt", options(entries))]
        }
        NewFieldKind::Radio => Vec::new(),
    }
}

pub(super) fn unique_name(wanted: &str, taken: &HashSet<String>) -> String {
    let base = if wanted.trim().is_empty() { "Field" } else { wanted.trim() };
    if !taken.contains(base) {
        return base.to_string();
    }
    (2..).map(|number| format!("{base}{number}")).find(|candidate| !taken.contains(candidate)).unwrap_or_default()
}

pub(super) fn ensure_form(transaction: &mut Transaction) -> Result<ObjId> {
    let catalog_id = transaction
        .store()
        .trailer()
        .get("Root")
        .and_then(Object::as_reference)
        .ok_or_else(|| Error::new("trailer has no /Root"))?;
    let mut catalog = transaction
        .object(catalog_id)?
        .as_dict()
        .cloned()
        .ok_or_else(|| Error::new("the catalog is not a dictionary"))?;
    let (form_id, mut form) = match catalog.get("AcroForm") {
        Some(Object::Ref(id)) => (Some(*id), transaction.object(*id)?.as_dict().cloned().unwrap_or_default()),
        Some(direct) => (None, transaction.resolve(direct)?.as_dict().cloned().unwrap_or_default()),
        None => (None, Dict::new()),
    };
    let before = form.clone();
    form.insert_missing(Name::from("Fields"), Object::Array(Vec::new()));
    form.insert_missing(Name::from("DA"), Object::String(FORM_APPEARANCE.to_vec()));
    let mut resources = match form.get("DR") {
        Some(resources) => transaction.resolve(resources)?.as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    let mut fonts = match resources.get("Font") {
        Some(fonts) => transaction.resolve(fonts)?.as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    if fonts.get(FORM_FONT).is_none() {
        let helvetica: Dict = [
            ("Type", Object::Name(Name::from("Font"))),
            ("Subtype", Object::Name(Name::from("Type1"))),
            ("BaseFont", Object::Name(Name::from("Helvetica"))),
            ("Encoding", Object::Name(Name::from("WinAnsiEncoding"))),
        ]
        .into_iter()
        .map(|(key, value)| (Name::from(key), value))
        .collect();
        fonts.insert(FORM_FONT, Object::Ref(transaction.allocate(Object::Dict(helvetica))));
        resources.insert("Font", Object::Dict(fonts));
        form.insert("DR", Object::Dict(resources));
    }
    match form_id {
        Some(id) => {
            if form != before {
                transaction.put(id, Object::Dict(form))?;
            }
            Ok(id)
        }
        None => {
            let id = transaction.allocate(Object::Dict(form));
            catalog.insert("AcroForm", Object::Ref(id));
            transaction.put(catalog_id, Object::Dict(catalog))?;
            Ok(id)
        }
    }
}

pub(super) fn add_to_form(transaction: &mut Transaction, form: ObjId, field: ObjId) -> Result<()> {
    let mut dict = transaction.object(form)?.as_dict().cloned().unwrap_or_default();
    let mut fields = match dict.get("Fields") {
        Some(fields) => transaction.resolve(fields)?.as_array().map(<[Object]>::to_vec).unwrap_or_default(),
        None => Vec::new(),
    };
    fields.push(Object::Ref(field));
    dict.insert("Fields", Object::Array(fields));
    transaction.put(form, Object::Dict(dict))
}

pub(super) fn add_to_page(transaction: &mut Transaction, page: ObjId, widget: ObjId) -> Result<()> {
    let mut annotations = annotation_list(transaction, page)?;
    annotations.push(Object::Ref(widget));
    write_annotation_list(transaction, page, annotations)
}

fn numbers(values: &[f64]) -> Object {
    Object::Array(values.iter().map(|value| Object::Real(*value)).collect())
}
