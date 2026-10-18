use std::io::Write;

use crate::syntax::lexer::{is_delimiter, is_whitespace};
use crate::syntax::{Dict, Name, ObjId, Object, Stream};

pub struct Serializer<'a> {
    out: &'a mut Vec<u8>,
}

impl<'a> Serializer<'a> {
    pub fn new(out: &'a mut Vec<u8>) -> Self {
        Self { out }
    }

    pub fn indirect(&mut self, id: ObjId, object: &Object) {
        let _ = writeln!(self.out, "{} {} obj", id.num, id.generation);
        self.object(object);
        self.out.extend_from_slice(b"\nendobj\n");
    }

    pub fn object(&mut self, object: &Object) {
        match object {
            Object::Null => self.out.extend_from_slice(b"null"),
            Object::Bool(value) => self.out.extend_from_slice(if *value { b"true" } else { b"false" }),
            Object::Int(value) => {
                let _ = write!(self.out, "{value}");
            }
            Object::Real(value) => self.real(*value),
            Object::String(bytes) => self.string(bytes),
            Object::Name(name) => self.name(name),
            Object::Array(items) => {
                self.out.push(b'[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        self.out.push(b' ');
                    }
                    self.object(item);
                }
                self.out.push(b']');
            }
            Object::Dict(dict) => self.dict(dict),
            Object::Stream(stream) => self.stream(stream),
            Object::Ref(id) => {
                let _ = write!(self.out, "{} {} R", id.num, id.generation);
            }
        }
    }

    fn real(&mut self, value: f64) {
        if !value.is_finite() {
            self.out.push(b'0');
            return;
        }
        let text = format!("{}", if value == 0.0 { 0.0 } else { value });
        self.out.extend_from_slice(text.as_bytes());
        if !text.contains('.') {
            self.out.extend_from_slice(b".0");
        }
    }

    fn string(&mut self, bytes: &[u8]) {
        let unprintable = bytes.iter().filter(|&&byte| !(0x20..0x7f).contains(&byte)).count();
        if unprintable * 4 > bytes.len() {
            self.out.push(b'<');
            for byte in bytes {
                let _ = write!(self.out, "{byte:02x}");
            }
            self.out.push(b'>');
            return;
        }
        self.out.push(b'(');
        for &byte in bytes {
            match byte {
                b'(' | b')' | b'\\' => self.out.extend_from_slice(&[b'\\', byte]),
                b'\r' => self.out.extend_from_slice(b"\\r"),
                _ => self.out.push(byte),
            }
        }
        self.out.push(b')');
    }

    fn name(&mut self, name: &Name) {
        self.out.push(b'/');
        for &byte in name.as_bytes() {
            if byte == b'#' || !(0x21..0x7f).contains(&byte) || is_delimiter(byte) || is_whitespace(byte) {
                let _ = write!(self.out, "#{byte:02X}");
            } else {
                self.out.push(byte);
            }
        }
    }

    fn dict(&mut self, dict: &Dict) {
        self.out.extend_from_slice(b"<<");
        for (key, value) in dict.iter() {
            self.name(key);
            self.out.push(b' ');
            self.object(value);
        }
        self.out.extend_from_slice(b">>");
    }

    fn stream(&mut self, stream: &Stream) {
        let mut dict = stream.dict.clone();
        dict.insert("Length", Object::Int(stream.raw.len() as i64));
        self.dict(&dict);
        self.out.extend_from_slice(b"\nstream\n");
        self.out.extend_from_slice(&stream.raw);
        self.out.extend_from_slice(b"\nendstream");
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::syntax::Parser;

    fn round_trip(object: &Object) -> Object {
        let mut out = Vec::new();
        Serializer::new(&mut out).object(object);
        Parser::new(&out, 0).parse_object().unwrap()
    }

    #[test]
    fn writes_objects_that_parse_back_unchanged() {
        let dict: Dict = [
            (Name::from("Type"), Object::Name(Name::from("Page"))),
            (Name::from("Odd Name#"), Object::Name(Name::from("a/b(c)"))),
            (Name::from("Kids"), Object::Array(vec![Object::Ref(ObjId::new(4, 2)), Object::Null, Object::Bool(true)])),
            (Name::from("Scale"), Object::Real(3.0)),
            (Name::from("Tiny"), Object::Real(-0.000125)),
            (Name::from("Text"), Object::String(b"a (nested) \\ back\r\nslash".to_vec())),
            (Name::from("Binary"), Object::String(vec![0, 1, 2, 0xff, 0xfe])),
        ]
        .into_iter()
        .collect();
        let array = Object::Array(vec![Object::Dict(dict), Object::Int(-7)]);
        assert_eq!(round_trip(&array), array);
    }

    #[test]
    fn writes_a_stream_with_its_actual_length() {
        let stream = Stream { dict: Dict::new(), raw: Rc::from(&b"q 1 0 0 1 0 0 cm Q"[..]) };
        let mut out = Vec::new();
        Serializer::new(&mut out).indirect(ObjId::new(9, 0), &Object::Stream(stream));
        let (id, parsed) = Parser::new(&out, 0).parse_indirect(|_| None).unwrap();
        assert_eq!(id, ObjId::new(9, 0));
        let parsed = parsed.as_stream().unwrap();
        assert_eq!(parsed.dict.int("Length"), Some(18));
        assert_eq!(&*parsed.raw, b"q 1 0 0 1 0 0 cm Q");
    }

    #[test]
    fn never_writes_an_exponent_or_a_non_number() {
        let mut out = Vec::new();
        let mut serializer = Serializer::new(&mut out);
        for value in [1e20, 1e-7, f64::NAN, -0.0] {
            serializer.object(&Object::Real(value));
            serializer.out.push(b' ');
        }
        assert_eq!(String::from_utf8(out).unwrap(), "100000000000000000000.0 0.0000001 0 0.0 ");
    }
}
