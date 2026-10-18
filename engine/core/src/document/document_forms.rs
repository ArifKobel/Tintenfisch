use crate::document::Document;
use crate::error::{Error, Result};
use crate::fonts::FontLibrary;
use crate::forms::{Field, FieldKind, FieldTree, FieldValue, on_state, refresh_appearances};
use crate::pages::PageId;
use crate::syntax::{Name, ObjId, Object, encode_text};

#[derive(Clone, Debug, PartialEq)]
pub enum FieldInput {
    Text(String),
    Checked(bool),
    Radio(ObjId),
    Choices(Vec<String>),
}

impl Document {
    pub fn fields(&self) -> Result<Vec<Field>> {
        FieldTree::new(&self.store).fields()
    }

    pub fn set_field(&mut self, field: ObjId, input: FieldInput, library: &FontLibrary) -> Result<Vec<PageId>> {
        let field = self
            .fields()?
            .into_iter()
            .find(|candidate| candidate.id == field)
            .ok_or_else(|| Error::new("no such field"))?;
        if field.read_only {
            return Err(Error::unsupported(format!("{} is read only", field.name)));
        }
        let value = value_for(&field, &input)?;
        let fonts = &self.caches.fonts;
        self.store.transact("Fill in field", "", |transaction| {
            let mut dict = transaction
                .object(field.id)?
                .as_dict()
                .cloned()
                .ok_or_else(|| Error::new("a field is not a dictionary"))?;
            dict.insert("V", value_object(&value));
            transaction.put(field.id, Object::Dict(dict))?;
            refresh_appearances(transaction, fonts, library, |candidate, widget| {
                candidate.id == field.id || widget.dict.get("AP").is_none()
            })
        })?;
        Ok(field.widgets.iter().filter_map(|widget| widget.page).collect())
    }
}

fn value_for(field: &Field, input: &FieldInput) -> Result<FieldValue> {
    Ok(match (&field.kind, input) {
        (FieldKind::Text { comb, .. }, FieldInput::Text(text)) => {
            FieldValue::Text(comb.map_or(text.clone(), |cells| text.chars().take(cells).collect()))
        }
        (FieldKind::Checkbox, FieldInput::Checked(checked)) => {
            FieldValue::State(field.widgets.first().map(|widget| on_state(field, widget)).filter(|_| *checked))
        }
        (FieldKind::Radio, FieldInput::Radio(chosen)) => {
            let widget = field
                .widgets
                .iter()
                .find(|widget| widget.id == *chosen)
                .ok_or_else(|| Error::new("the button is not part of this field"))?;
            FieldValue::State(Some(on_state(field, widget)))
        }
        (FieldKind::Choice { multiple, .. }, FieldInput::Choices(chosen)) => {
            FieldValue::Chosen(if *multiple { chosen.clone() } else { chosen.iter().take(1).cloned().collect() })
        }
        (FieldKind::Choice { editable: true, .. }, FieldInput::Text(text)) => FieldValue::Chosen(vec![text.clone()]),
        _ => return Err(Error::unsupported(format!("{} does not take that kind of input", field.name))),
    })
}

fn value_object(value: &FieldValue) -> Object {
    match value {
        FieldValue::Text(text) => Object::String(encode_text(text)),
        FieldValue::State(state) => Object::Name(state.clone().unwrap_or_else(|| Name::from("Off"))),
        FieldValue::Chosen(chosen) if chosen.len() == 1 => Object::String(encode_text(&chosen[0])),
        FieldValue::Chosen(chosen) => {
            Object::Array(chosen.iter().map(|value| Object::String(encode_text(value))).collect())
        }
    }
}
