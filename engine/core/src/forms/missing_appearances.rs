use std::cell::OnceCell;

use crate::error::Result;
use crate::fonts::{FontCache, FontLibrary};
use crate::forms::{AppearanceWriter, Field, FieldTree, FieldValue, WidgetAppearance};
use crate::store::ObjectStore;
use crate::syntax::{Dict, ObjId};

pub struct MissingAppearances<'a> {
    store: &'a ObjectStore,
    fonts: &'a FontCache,
    library: &'a FontLibrary,
    form: OnceCell<(Vec<Field>, Dict)>,
}

impl<'a> MissingAppearances<'a> {
    pub fn new(store: &'a ObjectStore, fonts: &'a FontCache, library: &'a FontLibrary) -> Self {
        Self { store, fonts, library, form: OnceCell::new() }
    }

    pub fn appearance(&self, widget: ObjId) -> Result<Option<WidgetAppearance>> {
        let (fields, resources) = match self.form.get() {
            Some(form) => form,
            None => {
                let tree = FieldTree::new(self.store);
                let resources = match tree.form()?.and_then(|form| form.get("DR").cloned()) {
                    Some(resources) => self.store.resolve(&resources)?.as_dict().cloned().unwrap_or_default(),
                    None => Dict::new(),
                };
                self.form.get_or_init(|| (tree.fields().unwrap_or_default(), resources))
            }
        };
        let Some((field, widget)) = fields.iter().find_map(|field| {
            field.widgets.iter().find(|candidate| candidate.id == widget).map(|widget| (field, widget))
        }) else {
            return Ok(None);
        };
        let writer = AppearanceWriter::new(self.store, self.fonts, self.library, resources.clone());
        Ok(Some(match &field.value {
            FieldValue::Text(text) => writer.text(field, widget, text, &[])?,
            FieldValue::Chosen(chosen) => writer.text(field, widget, &field.shown_choice(chosen), chosen)?,
            FieldValue::State(state) => {
                let (on, off) = writer.states(field, widget);
                let lit = state.is_some() && (widget.on_state.is_none() || widget.on_state == *state);
                if lit { on } else { off }
            }
        }))
    }
}
