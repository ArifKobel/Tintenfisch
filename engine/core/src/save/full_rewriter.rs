use std::collections::{HashMap, VecDeque};
use std::io::Write;

use crate::error::Result;
use crate::save::Encryption;
use crate::save::document_id::DocumentId;
use crate::save::object_output::for_output;
use crate::store::ObjectStore;
use crate::syntax::{Dict, ObjId, Object, Serializer, Stream};

const SOURCE: Encryption = Encryption::Source;

pub struct FullRewriter<'a> {
    store: &'a ObjectStore,
    encryption: &'a Encryption,
}

impl<'a> FullRewriter<'a> {
    pub fn new(store: &'a ObjectStore) -> Self {
        Self { store, encryption: &SOURCE }
    }

    pub fn encrypted(self, encryption: &'a Encryption) -> Self {
        Self { encryption, ..self }
    }

    pub fn write(&self) -> Result<Vec<u8>> {
        let source_trailer = self.store.trailer();
        let kept_keys: &[&str] = match self.encryption {
            Encryption::Source => &["Root", "Info", "Encrypt"],
            _ => &["Root", "Info"],
        };
        let roots: Vec<ObjId> =
            kept_keys.iter().filter_map(|key| source_trailer.get(key).and_then(Object::as_reference)).collect();
        let order = self.reachable(&roots)?;
        let numbers: HashMap<ObjId, u32> =
            order.iter().enumerate().map(|(index, id)| (*id, index as u32 + 1)).collect();
        let header = self.store.source().header();
        let mut out = format!("%PDF-{}.{}\n", header.major, header.minor).into_bytes();
        out.extend_from_slice(b"%\xe2\xe3\xcf\xd3\n");
        let security = match self.encryption {
            Encryption::Source => self.store.source().security(),
            Encryption::Sealed { handler, .. } => Some(handler),
            Encryption::Plain => None,
        };
        let mut offsets = Vec::with_capacity(order.len() + 1);
        for id in &order {
            let object = self.store.object(*id)?;
            offsets.push(out.len());
            let written_as = ObjId::new(numbers[id], 0);
            let output = for_output(&renumber(&object, &numbers), written_as, *id, security);
            Serializer::new(&mut out).indirect(written_as, &output);
        }
        let mut trailer = Dict::new();
        if let Encryption::Sealed { dictionary, .. } = self.encryption {
            let written_as = ObjId::new(order.len() as u32 + 1, 0);
            offsets.push(out.len());
            Serializer::new(&mut out).indirect(written_as, &Object::Dict(dictionary.clone()));
            trailer.insert("Encrypt", Object::Ref(written_as));
        }
        let start = out.len();
        let _ = write!(out, "xref\n0 {}\n0000000000 65535 f\r\n", offsets.len() + 1);
        for offset in &offsets {
            let _ = write!(out, "{offset:010} 00000 n\r\n");
        }
        trailer.insert("Size", Object::Int(offsets.len() as i64 + 1));
        for key in kept_keys {
            if let Some(value) = source_trailer.get(key) {
                trailer.insert(*key, renumber(value, &numbers));
            }
        }
        trailer.insert("ID", DocumentId::next(source_trailer, &out));
        out.extend_from_slice(b"trailer\n");
        Serializer::new(&mut out).object(&Object::Dict(trailer));
        let _ = write!(out, "\nstartxref\n{start}\n%%EOF\n");
        Ok(out)
    }

    fn reachable(&self, roots: &[ObjId]) -> Result<Vec<ObjId>> {
        let mut order = Vec::new();
        let mut seen: HashMap<ObjId, ()> = HashMap::new();
        let mut queue: VecDeque<ObjId> = VecDeque::new();
        for root in roots {
            if seen.insert(*root, ()).is_none() {
                queue.push_back(*root);
            }
        }
        while let Some(id) = queue.pop_front() {
            let object = self.store.object(id)?;
            if *object == Object::Null {
                continue;
            }
            order.push(id);
            let mut references = Vec::new();
            collect_references(&object, &mut references);
            for reference in references {
                if seen.insert(reference, ()).is_none() {
                    queue.push_back(reference);
                }
            }
        }
        Ok(order)
    }
}

fn collect_references(object: &Object, into: &mut Vec<ObjId>) {
    match object {
        Object::Ref(id) => into.push(*id),
        Object::Array(items) => items.iter().for_each(|item| collect_references(item, into)),
        Object::Dict(dict) => dict.iter().for_each(|(_, value)| collect_references(value, into)),
        Object::Stream(stream) => stream.dict.iter().for_each(|(_, value)| collect_references(value, into)),
        _ => {}
    }
}

fn renumber(object: &Object, numbers: &HashMap<ObjId, u32>) -> Object {
    match object {
        Object::Ref(id) => match numbers.get(id) {
            Some(&num) => Object::Ref(ObjId::new(num, 0)),
            None => Object::Null,
        },
        Object::Array(items) => Object::Array(items.iter().map(|item| renumber(item, numbers)).collect()),
        Object::Dict(dict) => Object::Dict(renumber_dict(dict, numbers)),
        Object::Stream(stream) => {
            Object::Stream(Stream { dict: renumber_dict(&stream.dict, numbers), raw: stream.raw.clone() })
        }
        other => other.clone(),
    }
}

fn renumber_dict(dict: &Dict, numbers: &HashMap<ObjId, u32>) -> Dict {
    dict.iter()
        .map(|(key, value)| (key.clone(), renumber(value, numbers)))
        .filter(|(_, value)| *value != Object::Null)
        .collect()
}
