use std::rc::Rc;

use crate::error::{Error, Result};
use crate::fonts::{FontCache, FontLibrary};
use crate::forms::{AppearanceWriter, Field, FieldKind, FieldTree, FieldValue, Widget, WidgetAppearance};
use crate::geometry::Rect;
use crate::store::Transaction;
use crate::syntax::{Dict, Name, ObjId, Object, Stream};

const DEFAULT_ON_STATE: &str = "Yes";

enum Planned {
    Normal(WidgetAppearance),
    State { on: Name, lit: bool, generated: Option<(WidgetAppearance, WidgetAppearance)> },
}

pub fn refresh_appearances(
    transaction: &mut Transaction,
    fonts: &FontCache,
    library: &FontLibrary,
    chosen: impl Fn(&Field, &Widget) -> bool,
) -> Result<()> {
    let planned = {
        let tree = FieldTree::new(transaction.store());
        let resources = match tree.form()?.and_then(|form| form.get("DR").cloned()) {
            Some(resources) => transaction.resolve(&resources)?.as_dict().cloned().unwrap_or_default(),
            None => Dict::new(),
        };
        let writer = AppearanceWriter::new(transaction.store(), fonts, library, resources);
        let mut planned = Vec::new();
        for field in
            tree.fields()?.iter().filter(|field| !matches!(field.kind, FieldKind::PushButton | FieldKind::Signature))
        {
            for widget in field.widgets.iter().filter(|widget| chosen(field, widget)) {
                planned.push((widget.id, plan(&writer, field, widget)?));
            }
        }
        planned
    };
    for (widget, planned) in planned {
        let mut dict =
            transaction.object(widget)?.as_dict().cloned().ok_or_else(|| Error::new("a widget is not a dictionary"))?;
        match planned {
            Planned::Normal(appearance) => {
                let stream = appearance_stream(transaction, appearance);
                dict.insert("AP", Object::Dict([(Name::from("N"), Object::Ref(stream))].into_iter().collect()));
            }
            Planned::State { on, lit, generated } => {
                if let Some((on_appearance, off_appearance)) = generated {
                    let on_stream = appearance_stream(transaction, on_appearance);
                    let off_stream = appearance_stream(transaction, off_appearance);
                    let states: Dict =
                        [(on.clone(), Object::Ref(on_stream)), (Name::from("Off"), Object::Ref(off_stream))]
                            .into_iter()
                            .collect();
                    dict.insert("AP", Object::Dict([(Name::from("N"), Object::Dict(states))].into_iter().collect()));
                }
                dict.insert("AS", Object::Name(if lit { on } else { Name::from("Off") }));
            }
        }
        transaction.put(widget, Object::Dict(dict))?;
    }
    Ok(())
}

pub fn on_state(field: &Field, widget: &Widget) -> Name {
    if let Some(state) = &widget.on_state {
        return state.clone();
    }
    if field.kind != FieldKind::Radio {
        return Name::from(DEFAULT_ON_STATE);
    }
    let taken: Vec<&Name> = field.widgets.iter().filter_map(|other| other.on_state.as_ref()).collect();
    let position = field.widgets.iter().position(|other| other.id == widget.id).unwrap_or(0);
    (position + 1..)
        .map(|number| Name::new(format!("Choice{number}")))
        .find(|name| !taken.contains(&name))
        .unwrap_or_else(|| Name::from(DEFAULT_ON_STATE))
}

fn plan(writer: &AppearanceWriter, field: &Field, widget: &Widget) -> Result<Planned> {
    Ok(match &field.value {
        FieldValue::Text(text) => Planned::Normal(writer.text(field, widget, text, &[])?),
        FieldValue::Chosen(chosen) => {
            Planned::Normal(writer.text(field, widget, &field.shown_choice(chosen), chosen)?)
        }
        FieldValue::State(state) => {
            let on = on_state(field, widget);
            let lit = state.as_ref() == Some(&on);
            let generated = widget.on_state.is_none().then(|| writer.states(field, widget));
            Planned::State { on, lit, generated }
        }
    })
}

fn appearance_stream(transaction: &mut Transaction, appearance: WidgetAppearance) -> ObjId {
    let form: Dict = [
        ("Type", Object::Name(Name::from("XObject"))),
        ("Subtype", Object::Name(Name::from("Form"))),
        ("BBox", Rect::new(0.0, 0.0, appearance.width, appearance.height).to_object()),
        ("Resources", Object::Dict(appearance.resources)),
    ]
    .into_iter()
    .map(|(key, value)| (Name::from(key), value))
    .collect();
    transaction.allocate(Object::Stream(Stream { dict: form, raw: Rc::from(appearance.content) }))
}
