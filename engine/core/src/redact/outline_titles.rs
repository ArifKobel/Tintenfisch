use std::collections::HashSet;

use crate::error::Result;
use crate::organize::catalog_of;
use crate::store::Transaction;
use crate::syntax::{ObjId, Object, decode_text, encode_text};

const MAX_DEPTH: usize = 32;
const NEUTRAL_TITLE: &str = "Redacted";

pub fn neutralize_outline_titles(transaction: &mut Transaction, removed: &[String]) -> Result<()> {
    let removed: Vec<String> = removed.iter().map(|text| text.to_lowercase()).collect();
    if removed.is_empty() {
        return Ok(());
    }
    let Some(root) = catalog_of(transaction)?.get("Outlines").and_then(Object::as_reference) else { return Ok(()) };
    let mut pending = vec![(root, 0)];
    let mut seen = HashSet::new();
    while let Some((node, depth)) = pending.pop() {
        if depth > MAX_DEPTH || !seen.insert(node) {
            continue;
        }
        let Some(mut dict) = transaction.object(node)?.as_dict().cloned() else { continue };
        let title = dict.get("Title").and_then(Object::as_string).map(decode_text).unwrap_or_default().to_lowercase();
        if removed.iter().any(|text| title.contains(text.as_str())) {
            dict.insert("Title", Object::String(encode_text(NEUTRAL_TITLE)));
            transaction.put(node, Object::Dict(dict.clone()))?;
        }
        let children: Vec<ObjId> =
            ["First", "Next"].iter().filter_map(|key| dict.get(key).and_then(Object::as_reference)).collect();
        pending.extend(children.into_iter().map(|child| (child, depth + 1)));
    }
    Ok(())
}
