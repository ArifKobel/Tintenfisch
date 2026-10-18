use std::collections::HashSet;

use crate::error::Result;
use crate::organize::page_list::catalog_of;
use crate::store::Transaction;
use crate::syntax::{Dict, ObjId, Object};

const MAX_DEPTH: usize = 32;

pub struct DeadLinks<'g> {
    gone: &'g HashSet<ObjId>,
    gone_widgets: HashSet<ObjId>,
    named: Dict,
    name_tree: Option<Object>,
}

impl<'g> DeadLinks<'g> {
    pub fn prune(transaction: &mut Transaction, gone: &'g HashSet<ObjId>, remaining: &[ObjId]) -> Result<()> {
        let mut gone_widgets = HashSet::new();
        for page in gone {
            gone_widgets.extend(annotation_refs(transaction, *page)?);
        }
        let catalog = catalog_of(transaction)?;
        let named = match catalog.get("Dests") {
            Some(dests) => transaction.resolve(dests)?.as_dict().cloned().unwrap_or_default(),
            None => Dict::new(),
        };
        let name_tree = match catalog.get("Names") {
            Some(names) => transaction.resolve(names)?.as_dict().and_then(|names| names.get("Dests")).cloned(),
            None => None,
        };
        let pruner = Self { gone, gone_widgets, named, name_tree };
        if let Some(outlines) = catalog.get("Outlines").and_then(Object::as_reference) {
            pruner.outline_children(transaction, outlines, 0)?;
        }
        for page in remaining {
            pruner.links(transaction, *page)?;
        }
        pruner.named_destinations(transaction)?;
        if let Some(form) = catalog.get("AcroForm") {
            pruner.fields(transaction, form)?;
        }
        Ok(())
    }

    pub fn prune_widgets(transaction: &mut Transaction, widgets: HashSet<ObjId>) -> Result<()> {
        let gone = HashSet::new();
        let pruner = DeadLinks { gone: &gone, gone_widgets: widgets, named: Dict::new(), name_tree: None };
        if let Some(form) = catalog_of(transaction)?.get("AcroForm").cloned() {
            pruner.fields(transaction, &form)?;
        }
        Ok(())
    }

    fn leads_to_gone(&self, transaction: &Transaction, target: &Object, depth: usize) -> Result<bool> {
        if depth > MAX_DEPTH {
            return Ok(false);
        }
        let target = transaction.resolve(target)?;
        Ok(match &target {
            Object::Array(items) => {
                items.first().and_then(Object::as_reference).is_some_and(|page| self.gone.contains(&page))
            }
            Object::Dict(dict) if dict.name("S").is_some_and(|kind| kind.is("GoTo")) => match dict.get("D") {
                Some(destination) => self.leads_to_gone(transaction, destination, depth + 1)?,
                None => false,
            },
            Object::Dict(dict) => match dict.get("D") {
                Some(destination) => self.leads_to_gone(transaction, destination, depth + 1)?,
                None => false,
            },
            Object::Name(name) => match self.named.get_name(name) {
                Some(destination) => self.leads_to_gone(transaction, &destination.clone(), depth + 1)?,
                None => false,
            },
            Object::String(name) => match self.tree_lookup(transaction, name)? {
                Some(destination) => self.leads_to_gone(transaction, &destination, depth + 1)?,
                None => false,
            },
            _ => false,
        })
    }

    fn targets_gone(&self, transaction: &Transaction, dict: &Dict) -> Result<bool> {
        for key in ["Dest", "A"] {
            if let Some(target) = dict.get(key)
                && self.leads_to_gone(transaction, target, 0)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn outline_children(&self, transaction: &mut Transaction, parent: ObjId, depth: usize) -> Result<i64> {
        let node = transaction.object(parent)?.as_dict().cloned().unwrap_or_default();
        let mut children = Vec::new();
        let mut next = node.get("First").and_then(Object::as_reference);
        let mut seen = HashSet::new();
        while let Some(id) = next.filter(|id| seen.insert(*id) && depth < MAX_DEPTH) {
            next = transaction.object(id)?.as_dict().and_then(|item| item.get("Next")).and_then(Object::as_reference);
            children.push(id);
        }
        let mut kept = Vec::new();
        for id in children {
            let shown = self.outline_children(transaction, id, depth + 1)?;
            let mut item = transaction.object(id)?.as_dict().cloned().unwrap_or_default();
            if self.targets_gone(transaction, &item)? {
                if item.get("First").is_none() {
                    continue;
                }
                item.remove("Dest");
                item.remove("A");
            }
            kept.push((id, item, shown));
        }
        let mut visible = 0;
        for index in 0..kept.len() {
            let previous = index.checked_sub(1).map(|previous| kept[previous].0);
            let following = kept.get(index + 1).map(|following| following.0);
            let (id, item, shown) = &mut kept[index];
            let before = transaction.object(*id)?.as_dict().cloned().unwrap_or_default();
            set_link(item, "Prev", previous);
            set_link(item, "Next", following);
            if item.get("First").is_some() {
                let open = item.get("Count").and_then(Object::as_int).is_none_or(|count| count > 0);
                item.insert("Count", Object::Int(if open { *shown } else { -*shown }));
                if open {
                    visible += *shown;
                }
            }
            visible += 1;
            if *item != before {
                transaction.put(*id, Object::Dict(item.clone()))?;
            }
        }
        let mut node = transaction.object(parent)?.as_dict().cloned().unwrap_or_default();
        let before = node.clone();
        set_link(&mut node, "First", kept.first().map(|item| item.0));
        set_link(&mut node, "Last", kept.last().map(|item| item.0));
        if depth == 0 {
            node.insert("Count", Object::Int(visible));
        }
        if node != before {
            transaction.put(parent, Object::Dict(node))?;
        }
        Ok(visible)
    }

    fn links(&self, transaction: &mut Transaction, page: ObjId) -> Result<()> {
        let mut dict = transaction.object(page)?.as_dict().cloned().unwrap_or_default();
        let Some(annotations) = dict.get("Annots").cloned() else { return Ok(()) };
        let listed = transaction.resolve(&annotations)?.as_array().map(<[Object]>::to_vec).unwrap_or_default();
        let mut kept = Vec::with_capacity(listed.len());
        for entry in &listed {
            let annotation = transaction.resolve(entry)?;
            let is_link = annotation
                .as_dict()
                .is_some_and(|annotation| annotation.name("Subtype").is_some_and(|kind| kind.is("Link")));
            let dead = is_link && self.targets_gone(transaction, annotation.as_dict().unwrap_or(&Dict::new()))?;
            if !dead {
                kept.push(entry.clone());
            }
        }
        if kept.len() != listed.len() {
            dict.insert("Annots", Object::Array(kept));
            transaction.put(page, Object::Dict(dict))?;
        }
        Ok(())
    }

    fn named_destinations(&self, transaction: &mut Transaction) -> Result<()> {
        let catalog_id = transaction.store().trailer().get("Root").and_then(Object::as_reference);
        let Some(catalog_id) = catalog_id else { return Ok(()) };
        let mut catalog = catalog_of(transaction)?;
        let mut kept = Dict::new();
        for (name, destination) in self.named.iter() {
            if !self.leads_to_gone(transaction, destination, 0)? {
                kept.insert(name.clone(), destination.clone());
            }
        }
        if kept.len() != self.named.len() {
            match catalog.get("Dests").and_then(Object::as_reference) {
                Some(id) => transaction.put(id, Object::Dict(kept))?,
                None => {
                    catalog.insert("Dests", Object::Dict(kept));
                    transaction.put(catalog_id, Object::Dict(catalog))?;
                }
            }
        }
        if let Some(tree) = &self.name_tree {
            let (pruned, _) = self.tree_node(transaction, tree, 0)?;
            if pruned != *tree {
                let mut catalog = catalog_of(transaction)?;
                let mut names = match catalog.get("Names") {
                    Some(names) => transaction.resolve(names)?.as_dict().cloned().unwrap_or_default(),
                    None => Dict::new(),
                };
                names.insert("Dests", pruned);
                match catalog.get("Names").and_then(Object::as_reference) {
                    Some(id) => transaction.put(id, Object::Dict(names))?,
                    None => {
                        catalog.insert("Names", Object::Dict(names));
                        transaction.put(catalog_id, Object::Dict(catalog))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn tree_node(&self, transaction: &mut Transaction, node: &Object, depth: usize) -> Result<(Object, bool)> {
        let id = node.as_reference();
        let original = transaction.resolve(node)?;
        let Some(before) = original.as_dict().filter(|_| depth <= MAX_DEPTH) else { return Ok((node.clone(), true)) };
        let mut dict = before.clone();
        let mut keys: Vec<Vec<u8>> = Vec::new();
        if let Some(names) = dict.get("Names").and_then(Object::as_array).map(<[Object]>::to_vec) {
            let mut kept = Vec::new();
            for pair in names.chunks(2) {
                let [key, value] = pair else { continue };
                if !self.leads_to_gone(transaction, value, 0)? {
                    keys.extend(key.as_string().map(<[u8]>::to_vec));
                    kept.extend([key.clone(), value.clone()]);
                }
            }
            dict.insert("Names", Object::Array(kept));
        }
        if let Some(kids) = dict.get("Kids").and_then(Object::as_array).map(<[Object]>::to_vec) {
            let mut kept = Vec::new();
            for kid in &kids {
                let (pruned, filled) = self.tree_node(transaction, kid, depth + 1)?;
                if filled {
                    let child = transaction.resolve(&pruned)?;
                    if let Some(limits) =
                        child.as_dict().and_then(|child| child.get("Limits")).and_then(Object::as_array)
                    {
                        keys.extend(limits.iter().filter_map(Object::as_string).map(<[u8]>::to_vec));
                    }
                    kept.push(pruned);
                }
            }
            dict.insert("Kids", Object::Array(kept));
        }
        keys.sort();
        if dict.get("Limits").is_some()
            && let (Some(first), Some(last)) = (keys.first(), keys.last())
        {
            dict.insert("Limits", Object::Array(vec![Object::String(first.clone()), Object::String(last.clone())]));
        }
        let filled = !keys.is_empty();
        if dict == *before {
            return Ok((node.clone(), filled));
        }
        match id {
            Some(id) => {
                transaction.put(id, Object::Dict(dict))?;
                Ok((node.clone(), filled))
            }
            None => Ok((Object::Dict(dict), filled)),
        }
    }

    fn tree_lookup(&self, transaction: &Transaction, name: &[u8]) -> Result<Option<Object>> {
        let Some(tree) = &self.name_tree else { return Ok(None) };
        let mut pending = vec![(transaction.resolve(tree)?, 0)];
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
                    pending.push((transaction.resolve(kid)?, depth + 1));
                }
            }
        }
        Ok(None)
    }

    fn fields(&self, transaction: &mut Transaction, form: &Object) -> Result<()> {
        let form_id = form.as_reference();
        let mut form = transaction.resolve(form)?.as_dict().cloned().unwrap_or_default();
        let fields = form.get("Fields").and_then(Object::as_array).map(<[Object]>::to_vec).unwrap_or_default();
        let mut kept = Vec::new();
        for field in &fields {
            if self.field_survives(transaction, field, 0)? {
                kept.push(field.clone());
            }
        }
        if kept.len() != fields.len() {
            form.insert("Fields", Object::Array(kept));
            match form_id {
                Some(id) => transaction.put(id, Object::Dict(form))?,
                None => {
                    let mut catalog = catalog_of(transaction)?;
                    catalog.insert("AcroForm", Object::Dict(form));
                    let catalog_id = transaction.store().trailer().get("Root").and_then(Object::as_reference);
                    if let Some(catalog_id) = catalog_id {
                        transaction.put(catalog_id, Object::Dict(catalog))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn field_survives(&self, transaction: &mut Transaction, field: &Object, depth: usize) -> Result<bool> {
        let Some(id) = field.as_reference() else { return Ok(true) };
        if self.gone_widgets.contains(&id) {
            return Ok(false);
        }
        let mut dict = transaction.object(id)?.as_dict().cloned().unwrap_or_default();
        let Some(kids) = dict.get("Kids").and_then(Object::as_array).map(<[Object]>::to_vec) else { return Ok(true) };
        if depth > MAX_DEPTH || kids.is_empty() {
            return Ok(true);
        }
        let mut kept = Vec::new();
        for kid in &kids {
            if self.field_survives(transaction, kid, depth + 1)? {
                kept.push(kid.clone());
            }
        }
        if kept.is_empty() {
            return Ok(false);
        }
        if kept.len() != kids.len() {
            dict.insert("Kids", Object::Array(kept));
            transaction.put(id, Object::Dict(dict))?;
        }
        Ok(true)
    }
}

fn annotation_refs(transaction: &Transaction, page: ObjId) -> Result<Vec<ObjId>> {
    let object = transaction.object(page)?;
    let Some(annotations) = object.as_dict().and_then(|page| page.get("Annots")) else { return Ok(Vec::new()) };
    Ok(transaction
        .resolve(annotations)?
        .as_array()
        .unwrap_or_default()
        .iter()
        .filter_map(Object::as_reference)
        .collect())
}

fn set_link(dict: &mut Dict, key: &str, target: Option<ObjId>) {
    match target {
        Some(id) => dict.insert(key, Object::Ref(id)),
        None => {
            dict.remove(key);
        }
    }
}
