use std::borrow::Cow;
use std::io::Write;
use std::rc::Rc;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::crypt::SecurityHandler;
use crate::syntax::{Name, ObjId, Object, Stream};

const WORTH_COMPRESSING: usize = 64;

pub fn for_output(object: &Object, written_as: ObjId, source_id: ObjId, security: Option<&SecurityHandler>) -> Object {
    let compressed = compress_unfiltered(object).into_owned();
    match security {
        Some(security) if !security.is_dictionary(source_id) => security.encrypt(written_as, compressed),
        _ => compressed,
    }
}

pub fn compress_unfiltered(object: &Object) -> Cow<'_, Object> {
    match object {
        Object::Stream(stream) if !stream.dict.contains("Filter") && stream.raw.len() >= WORTH_COMPRESSING => {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            let compressed = encoder.write_all(&stream.raw).and_then(|_| encoder.finish());
            match compressed {
                Ok(data) if data.len() < stream.raw.len() => {
                    let mut dict = stream.dict.clone();
                    dict.insert("Filter", Object::Name(Name::from("FlateDecode")));
                    Cow::Owned(Object::Stream(Stream { dict, raw: Rc::from(data) }))
                }
                _ => Cow::Borrowed(object),
            }
        }
        _ => Cow::Borrowed(object),
    }
}
