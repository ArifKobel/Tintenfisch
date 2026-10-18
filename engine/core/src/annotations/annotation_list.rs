use crate::error::{Error, Result};
use crate::store::Transaction;
use crate::syntax::{ObjId, Object};

pub fn annotation_list(transaction: &Transaction, page: ObjId) -> Result<Vec<Object>> {
    let page = transaction.object(page)?;
    match page.as_dict().and_then(|dict| dict.get("Annots")) {
        Some(annotations) => {
            Ok(transaction.resolve(annotations)?.as_array().map(<[Object]>::to_vec).unwrap_or_default())
        }
        None => Ok(Vec::new()),
    }
}

pub fn write_annotation_list(transaction: &mut Transaction, page: ObjId, annotations: Vec<Object>) -> Result<()> {
    let mut dict =
        transaction.object(page)?.as_dict().cloned().ok_or_else(|| Error::new("a page is not a dictionary"))?;
    match dict.get("Annots").and_then(Object::as_reference) {
        Some(array) => transaction.put(array, Object::Array(annotations)),
        None => {
            dict.insert("Annots", Object::Array(annotations));
            transaction.put(page, Object::Dict(dict))
        }
    }
}
