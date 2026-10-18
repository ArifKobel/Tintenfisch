use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::file::filters::FilterChain;
use crate::file::object_stream::ObjectStream;
use crate::file::xref::{Xref, XrefEntry};
use crate::syntax::lexer::is_whitespace;
use crate::syntax::{Dict, Name, ObjId, Object, Parser};

pub struct Repair<'a> {
    bytes: &'a [u8],
}

struct Found {
    offset: usize,
    id: ObjId,
    object: Object,
}

impl<'a> Repair<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub fn scan(&self) -> Result<Xref> {
        let found = self.objects();
        let mut entries = BTreeMap::new();
        for object in &found {
            entries.insert(object.id.num, XrefEntry::InUse { offset: object.offset, generation: object.id.generation });
        }
        for object in &found {
            for (index, member) in self.object_stream_members(&object.object).into_iter().enumerate() {
                entries.entry(member).or_insert(XrefEntry::Compressed { stream: object.id.num, index: index as u32 });
            }
        }
        let mut trailer = self.trailer(&found);
        if !trailer.contains("Root") {
            let catalog = found
                .iter()
                .rev()
                .find(|object| object.object.as_dict().is_some_and(|dict| dict.has_type("Catalog")))
                .ok_or_else(|| Error::new("damaged file: no document catalog found"))?;
            trailer.insert("Root", Object::Ref(catalog.id));
        }
        let size = entries.keys().next_back().map_or(0, |num| num + 1);
        trailer.insert("Size", Object::Int(size as i64));
        Ok(Xref::new(entries, trailer, None))
    }

    fn objects(&self) -> Vec<Found> {
        let mut found = Vec::new();
        let mut search = 0;
        while let Some(relative) = self.bytes[search..].windows(3).position(|window| window == b"obj") {
            let keyword = search + relative;
            search = keyword + 3;
            let Some(start) = self.header_start(keyword) else {
                continue;
            };
            let mut parser = Parser::new(self.bytes, start);
            if let Ok((id, object)) = parser.parse_indirect(|_| None) {
                found.push(Found { offset: start, id, object });
                search = search.max(parser.position());
            }
        }
        found
    }

    fn header_start(&self, keyword: usize) -> Option<usize> {
        let bytes = self.bytes;
        if bytes.get(keyword + 3).is_some_and(|&byte| !is_whitespace(byte) && !b"<[/(%".contains(&byte)) {
            return None;
        }
        let mut position = keyword;
        let skip_back = |position: &mut usize, accept: fn(u8) -> bool| {
            let end = *position;
            while *position > 0 && accept(bytes[*position - 1]) {
                *position -= 1;
            }
            end - *position
        };
        let digit = |byte: u8| byte.is_ascii_digit();
        if skip_back(&mut position, is_whitespace) == 0
            || skip_back(&mut position, digit) == 0
            || skip_back(&mut position, is_whitespace) == 0
            || skip_back(&mut position, digit) == 0
        {
            return None;
        }
        let boundary = position == 0 || is_whitespace(bytes[position - 1]) || b">])".contains(&bytes[position - 1]);
        boundary.then_some(position)
    }

    fn object_stream_members(&self, object: &Object) -> Vec<u32> {
        let Object::Stream(stream) = object else {
            return Vec::new();
        };
        if !stream.dict.has_type("ObjStm") {
            return Vec::new();
        }
        FilterChain::of(&stream.dict, Object::clone)
            .and_then(|chain| chain.decode(&stream.raw))
            .and_then(|decoded| ObjectStream::parse(&stream.dict, decoded.data))
            .map(|objects| objects.members().collect())
            .unwrap_or_default()
    }

    fn trailer(&self, found: &[Found]) -> Dict {
        let mut trailers: Vec<Dict> = Vec::new();
        let mut search = 0;
        while let Some(relative) = self.bytes[search..].windows(7).position(|window| window == b"trailer") {
            let keyword = search + relative;
            search = keyword + 7;
            if let Ok(Object::Dict(dict)) = Parser::new(self.bytes, search).parse_object() {
                trailers.push(dict);
            }
        }
        trailers.extend(
            found
                .iter()
                .filter_map(|object| object.object.as_stream())
                .filter(|stream| stream.dict.has_type("XRef"))
                .map(|stream| stream.dict.clone()),
        );
        let mut merged = Dict::new();
        for trailer in trailers.iter().rev().filter(|trailer| trailer.contains("Root")).chain(trailers.iter().rev()) {
            for key in ["Root", "Info", "ID", "Encrypt"] {
                if let Some(value) = trailer.get(key) {
                    merged.insert_missing(Name::from(key), value.clone());
                }
            }
        }
        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_objects_whose_offsets_are_unknown() {
        let file = b"%PDF-1.4\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj<</Type/Pages/Kids[]/Count 0>>endobj 3 1 obj (x) endobj\n";
        let xref = Repair::new(file).scan().unwrap();
        assert_eq!(xref.entry(1), Some(XrefEntry::InUse { offset: 9, generation: 0 }));
        assert!(matches!(xref.entry(3), Some(XrefEntry::InUse { generation: 1, .. })));
        assert_eq!(xref.trailer().get("Root"), Some(&Object::Ref(ObjId::new(1, 0))));
        assert_eq!(xref.trailer().int("Size"), Some(4));
    }

    #[test]
    fn lets_a_later_copy_of_an_object_win() {
        let file = b"%PDF-1.4\n1 0 obj (old) endobj\n1 0 obj (new) endobj\ntrailer << /Root 1 0 R >>";
        let xref = Repair::new(file).scan().unwrap();
        assert_eq!(xref.entry(1), Some(XrefEntry::InUse { offset: 30, generation: 0 }));
    }

    #[test]
    fn ignores_obj_inside_other_words() {
        let file = b"%PDF-1.4\n1 0 obj << /Type /Catalog /S (12 0 objective) >> endobj";
        let xref = Repair::new(file).scan().unwrap();
        assert_eq!(xref.ids().collect::<Vec<_>>(), vec![ObjId::new(1, 0)]);
    }

    #[test]
    fn fails_without_any_catalog() {
        assert!(Repair::new(b"%PDF-1.4\n1 0 obj (x) endobj").scan().is_err());
    }
}
