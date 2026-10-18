use crate::error::Result;
use crate::store::{ObjectStore, Transaction};
use crate::syntax::{Dict, Name, ObjId, Object};

pub fn resource(store: &ObjectStore, resources: &Dict, category: &str, name: &Object) -> Option<Object> {
    let name = name.as_name()?;
    let category = store.resolve(resources.get(category)?).ok()?;
    category.as_dict()?.get_name(name).cloned()
}

pub fn bind_xobject(
    transaction: &mut Transaction,
    page_resources: &mut Dict,
    scope: Option<ObjId>,
    name: &Name,
    image: ObjId,
) -> Result<()> {
    let insert = |transaction: &Transaction, resources: &mut Dict| -> Result<()> {
        let mut xobjects = match resources.get("XObject") {
            Some(xobjects) => transaction.resolve(xobjects)?.as_dict().cloned().unwrap_or_default(),
            None => Dict::new(),
        };
        xobjects.insert(name.clone(), Object::Ref(image));
        resources.insert("XObject", Object::Dict(xobjects));
        Ok(())
    };
    let Some(form) = scope else { return insert(transaction, page_resources) };
    let object = transaction.object(form)?;
    let Some(mut stream) = object.as_stream().cloned() else { return Ok(()) };
    match stream.dict.get("Resources") {
        Some(own) => {
            let mut own = transaction.resolve(own)?.as_dict().cloned().unwrap_or_default();
            insert(transaction, &mut own)?;
            stream.dict.insert("Resources", Object::Dict(own));
            transaction.put(form, Object::Stream(stream))
        }
        None => insert(transaction, page_resources),
    }
}
