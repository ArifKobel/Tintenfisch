use crate::error::Result;
use crate::pages::PageId;
use crate::store::ObjectStore;
use crate::syntax::{Dict, Name, Object};

const MAX_DEPTH: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Page { page: PageId, top: Option<f64> },
    Uri(String),
}

impl Target {
    pub fn of(store: &ObjectStore, dict: &Dict) -> Result<Option<Target>> {
        for key in ["Dest", "A"] {
            if let Some(target) = dict.get(key) {
                return Target::resolve(store, target, 0);
            }
        }
        Ok(None)
    }

    pub fn write(&self, dict: &mut Dict) {
        dict.remove("Dest");
        dict.remove("A");
        match self {
            Target::Page { page, top } => {
                let view = match top {
                    Some(top) => vec![Object::Name(Name::from("XYZ")), Object::Null, Object::Real(*top), Object::Null],
                    None => vec![Object::Name(Name::from("Fit"))],
                };
                dict.insert("Dest", Object::Array([vec![Object::Ref(page.0)], view].concat()));
            }
            Target::Uri(uri) => {
                let action: Dict = [
                    ("Type", Object::Name(Name::from("Action"))),
                    ("S", Object::Name(Name::from("URI"))),
                    ("URI", Object::String(uri.as_bytes().to_vec())),
                ]
                .into_iter()
                .map(|(key, value)| (Name::from(key), value))
                .collect();
                dict.insert("A", Object::Dict(action));
            }
        }
    }

    fn resolve(store: &ObjectStore, target: &Object, depth: usize) -> Result<Option<Target>> {
        if depth > MAX_DEPTH {
            return Ok(None);
        }
        Ok(match store.resolve(target)? {
            Object::Array(items) => Target::explicit(&items),
            Object::Dict(action) => match action.name("S") {
                Some(kind) if kind.is("URI") => action
                    .get("URI")
                    .and_then(Object::as_string)
                    .map(|uri| Target::Uri(String::from_utf8_lossy(uri).into_owned())),
                Some(kind) if !kind.is("GoTo") => None,
                _ => match action.get("D") {
                    Some(destination) => Target::resolve(store, destination, depth + 1)?,
                    None => None,
                },
            },
            Object::Name(name) => match named_destinations(store)?.and_then(|named| named.get_name(&name).cloned()) {
                Some(destination) => Target::resolve(store, &destination, depth + 1)?,
                None => None,
            },
            Object::String(name) => match tree_lookup(store, &name)? {
                Some(destination) => Target::resolve(store, &destination, depth + 1)?,
                None => None,
            },
            _ => None,
        })
    }

    fn explicit(items: &[Object]) -> Option<Target> {
        let page = PageId(items.first()?.as_reference()?);
        let number = |index: usize| items.get(index).and_then(Object::as_number);
        let top = match items.get(1).and_then(Object::as_name) {
            Some(view) if view.is("XYZ") => number(3),
            Some(view) if view.is("FitH") || view.is("FitBH") => number(2),
            Some(view) if view.is("FitR") => number(5),
            _ => None,
        };
        Some(Target::Page { page, top })
    }
}

pub(super) fn catalog(store: &ObjectStore) -> Result<Option<Dict>> {
    let Some(root) = store.trailer().get("Root") else { return Ok(None) };
    Ok(store.resolve(root)?.as_dict().cloned())
}

fn named_destinations(store: &ObjectStore) -> Result<Option<Dict>> {
    let Some(dests) = catalog(store)?.and_then(|catalog| catalog.get("Dests").cloned()) else { return Ok(None) };
    Ok(store.resolve(&dests)?.as_dict().cloned())
}

fn tree_lookup(store: &ObjectStore, name: &[u8]) -> Result<Option<Object>> {
    let names = catalog(store)?.and_then(|catalog| catalog.get("Names").cloned());
    let Some(names) = names else { return Ok(None) };
    let Some(tree) = store.resolve(&names)?.as_dict().and_then(|names| names.get("Dests").cloned()) else {
        return Ok(None);
    };
    let mut pending = vec![(store.resolve(&tree)?, 0)];
    while let Some((node, depth)) = pending.pop() {
        let Some(node) = node.as_dict() else { continue };
        for pair in node.get("Names").and_then(Object::as_array).unwrap_or_default().chunks(2) {
            if let [key, value] = pair
                && key.as_string() == Some(name)
            {
                return Ok(Some(value.clone()));
            }
        }
        if depth < MAX_DEPTH {
            for kid in node.get("Kids").and_then(Object::as_array).unwrap_or_default() {
                pending.push((store.resolve(kid)?, depth + 1));
            }
        }
    }
    Ok(None)
}
