use md5::{Digest, Md5};

use crate::syntax::{Dict, Object};

pub struct DocumentId;

impl DocumentId {
    pub fn next(trailer: &Dict, written: &[u8]) -> Object {
        let revision = Md5::digest(written).to_vec();
        let permanent = trailer
            .get("ID")
            .and_then(Object::as_array)
            .and_then(|parts| parts.first())
            .and_then(Object::as_string)
            .map(<[u8]>::to_vec)
            .unwrap_or_else(|| revision.clone());
        Object::Array(vec![Object::String(permanent), Object::String(revision)])
    }
}
