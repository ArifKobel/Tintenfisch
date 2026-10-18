use std::collections::{HashMap, HashSet};

use crate::error::Result;
use crate::forms::field::{ChoiceOption, Field, FieldFlags, FieldKind, FieldValue, Widget};
use crate::geometry::Rect;
use crate::pages::{PageId, PageTree};
use crate::store::ObjectStore;
use crate::syntax::{Dict, Name, ObjId, Object, decode_text};

const MAX_DEPTH: usize = 32;

#[derive(Clone, Default)]
struct Inherited {
    name: String,
    field_type: Option<Name>,
    flags: i64,
    value: Option<Object>,
    default_appearance: Option<Vec<u8>>,
    quadding: i64,
    options: Option<Object>,
    max_length: Option<i64>,
}

pub struct FieldTree<'s> {
    store: &'s ObjectStore,
}

impl<'s> FieldTree<'s> {
    pub fn new(store: &'s ObjectStore) -> Self {
        Self { store }
    }

    pub fn form(&self) -> Result<Option<Dict>> {
        let Some(catalog) = self.store.trailer().get("Root").and_then(Object::as_reference) else { return Ok(None) };
        let catalog = self.store.object(catalog)?;
        let Some(form) = catalog.as_dict().and_then(|catalog| catalog.get("AcroForm")) else { return Ok(None) };
        Ok(self.store.resolve(form)?.as_dict().cloned())
    }

    pub fn fields(&self) -> Result<Vec<Field>> {
        let Some(form) = self.form()? else { return Ok(Vec::new()) };
        let pages = self.widget_pages()?;
        let inherited = Inherited {
            default_appearance: form.get("DA").and_then(Object::as_string).map(<[u8]>::to_vec),
            quadding: form.get("Q").and_then(Object::as_int).unwrap_or(0),
            ..Inherited::default()
        };
        let mut fields = Vec::new();
        let mut visited = HashSet::new();
        for root in form
            .get("Fields")
            .map(|fields| self.store.resolve(fields))
            .transpose()?
            .as_ref()
            .and_then(Object::as_array)
            .unwrap_or_default()
        {
            if let Some(id) = root.as_reference() {
                self.walk(id, &inherited, &pages, 0, &mut visited, &mut fields)?;
            }
        }
        Ok(fields)
    }

    pub fn flags(&self, field: ObjId) -> Result<FieldFlags> {
        let mut node = Some(field);
        let mut depth = 0;
        while let Some(id) = node.filter(|_| depth < MAX_DEPTH) {
            let object = self.store.object(id)?;
            let Some(dict) = object.as_dict() else { break };
            if let Some(flags) = dict.get("Ff").and_then(Object::as_int) {
                return Ok(FieldFlags(flags));
            }
            node = dict.get("Parent").and_then(Object::as_reference);
            depth += 1;
        }
        Ok(FieldFlags::default())
    }

    fn walk(
        &self,
        id: ObjId,
        parent: &Inherited,
        pages: &HashMap<ObjId, PageId>,
        depth: usize,
        visited: &mut HashSet<ObjId>,
        fields: &mut Vec<Field>,
    ) -> Result<()> {
        if depth > MAX_DEPTH || !visited.insert(id) {
            return Ok(());
        }
        let object = self.store.object(id)?;
        let Some(node) = object.as_dict() else { return Ok(()) };
        let inherited = self.below(parent, node);
        let kids: Vec<ObjId> = match node.get("Kids") {
            Some(kids) => self
                .store
                .resolve(kids)?
                .as_array()
                .unwrap_or_default()
                .iter()
                .filter_map(Object::as_reference)
                .collect(),
            None => Vec::new(),
        };
        let (field_kids, widget_kids): (Vec<ObjId>, Vec<ObjId>) = kids.into_iter().partition(|kid| {
            self.store
                .object(*kid)
                .ok()
                .and_then(|kid| kid.as_dict().map(|kid| kid.get("T").is_some()))
                .unwrap_or(false)
        });
        for kid in field_kids {
            self.walk(kid, &inherited, pages, depth + 1, visited, fields)?;
        }
        let mut widgets = Vec::new();
        if node.name("Subtype").is_some_and(|subtype| subtype.is("Widget")) {
            widgets.extend(self.widget(id, node, pages)?);
        }
        for kid in widget_kids {
            let object = self.store.object(kid)?;
            if let Some(dict) = object.as_dict() {
                widgets.extend(self.widget(kid, dict, pages)?);
            }
        }
        if widgets.is_empty() {
            return Ok(());
        }
        let options = self.options(inherited.options.as_ref())?;
        let flags = FieldFlags(inherited.flags);
        let Some(kind) = FieldKind::of(inherited.field_type.as_ref(), flags, options, inherited.max_length) else {
            return Ok(());
        };
        let value = self.value(&kind, inherited.value.as_ref(), &widgets)?;
        fields.push(Field {
            id,
            name: inherited.name.clone(),
            kind,
            value,
            read_only: flags.read_only(),
            required: flags.required(),
            default_appearance: inherited.default_appearance.clone(),
            quadding: inherited.quadding,
            widgets,
        });
        Ok(())
    }

    fn below(&self, parent: &Inherited, node: &Dict) -> Inherited {
        let partial = node.get("T").and_then(Object::as_string).map(decode_text);
        let name = match partial {
            Some(partial) if parent.name.is_empty() => partial,
            Some(partial) => format!("{}.{partial}", parent.name),
            None => parent.name.clone(),
        };
        Inherited {
            name,
            field_type: node.name("FT").cloned().or_else(|| parent.field_type.clone()),
            flags: node.get("Ff").and_then(Object::as_int).unwrap_or(parent.flags),
            value: node.get("V").cloned().or_else(|| parent.value.clone()),
            default_appearance: node
                .get("DA")
                .and_then(Object::as_string)
                .map(<[u8]>::to_vec)
                .or_else(|| parent.default_appearance.clone()),
            quadding: node.get("Q").and_then(Object::as_int).unwrap_or(parent.quadding),
            options: node.get("Opt").cloned().or_else(|| parent.options.clone()),
            max_length: node.get("MaxLen").and_then(Object::as_int).or(parent.max_length),
        }
    }

    fn widget(&self, id: ObjId, dict: &Dict, pages: &HashMap<ObjId, PageId>) -> Result<Option<Widget>> {
        let Some(rect) = dict
            .get("Rect")
            .map(|rect| self.store.resolve(rect))
            .transpose()?
            .and_then(|rect| Rect::from_object(&rect))
        else {
            return Ok(None);
        };
        let page = pages.get(&id).copied().or_else(|| dict.get("P").and_then(Object::as_reference).map(PageId));
        let on_state = match dict.get("AP").map(|appearance| self.store.resolve(appearance)).transpose()? {
            Some(appearance) => match appearance
                .as_dict()
                .and_then(|appearance| appearance.get("N"))
                .map(|normal| self.store.resolve(normal))
                .transpose()?
            {
                Some(Object::Dict(states)) => states.iter().map(|(name, _)| name).find(|name| !name.is("Off")).cloned(),
                _ => None,
            },
            None => None,
        };
        Ok(Some(Widget { id, page, rect, on_state, dict: dict.clone() }))
    }

    fn options(&self, options: Option<&Object>) -> Result<Vec<ChoiceOption>> {
        let Some(options) = options else { return Ok(Vec::new()) };
        let options = self.store.resolve(options)?;
        let mut parsed = Vec::new();
        for entry in options.as_array().unwrap_or_default() {
            let entry = self.store.resolve(entry)?;
            let text = |object: &Object| object.as_string().map(decode_text).unwrap_or_default();
            parsed.push(match entry.as_array() {
                Some([value, label, ..]) => ChoiceOption { value: text(value), label: text(label) },
                _ => ChoiceOption { value: text(&entry), label: text(&entry) },
            });
        }
        Ok(parsed)
    }

    fn value(&self, kind: &FieldKind, value: Option<&Object>, widgets: &[Widget]) -> Result<FieldValue> {
        let value = value.map(|value| self.store.resolve(value)).transpose()?;
        Ok(match kind {
            FieldKind::Checkbox | FieldKind::Radio | FieldKind::PushButton => {
                let state = value
                    .as_ref()
                    .and_then(Object::as_name)
                    .cloned()
                    .or_else(|| widgets.iter().find_map(|widget| widget.dict.name("AS").cloned()));
                FieldValue::State(state.filter(|state| !state.is("Off")))
            }
            FieldKind::Choice { .. } => FieldValue::Chosen(match &value {
                Some(Object::Array(items)) => items.iter().filter_map(Object::as_string).map(decode_text).collect(),
                Some(Object::String(text)) => vec![decode_text(text)],
                _ => Vec::new(),
            }),
            FieldKind::Text { .. } | FieldKind::Signature => {
                FieldValue::Text(value.as_ref().and_then(Object::as_string).map(decode_text).unwrap_or_default())
            }
        })
    }

    fn widget_pages(&self) -> Result<HashMap<ObjId, PageId>> {
        let mut pages = HashMap::new();
        for page in PageTree::new(self.store).pages()? {
            let object = self.store.object(page.id.0)?;
            let Some(annotations) = object.as_dict().and_then(|dict| dict.get("Annots")) else { continue };
            for annotation in self.store.resolve(annotations)?.as_array().unwrap_or_default() {
                if let Some(id) = annotation.as_reference() {
                    pages.insert(id, page.id);
                }
            }
        }
        Ok(pages)
    }
}
