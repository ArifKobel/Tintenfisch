use std::rc::Rc;

use crate::error::{Error, Result};
use crate::syntax::lexer::{Lexer, Token};
use crate::syntax::{Dict, ObjId, Object, Stream};

const MAX_DEPTH: usize = 256;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(bytes: &'a [u8], position: usize) -> Self {
        Self { lexer: Lexer::new(bytes, position) }
    }

    pub fn position(&self) -> usize {
        self.lexer.position()
    }

    pub fn lexer(&mut self) -> &mut Lexer<'a> {
        &mut self.lexer
    }

    pub fn parse_object(&mut self) -> Result<Object> {
        self.object(0)
    }

    pub fn parse_indirect(&mut self, stream_length: impl Fn(ObjId) -> Option<usize>) -> Result<(ObjId, Object)> {
        let start = self.position();
        let id = self.object_header().ok_or_else(|| Error::at(start, "expected an object header"))?;
        let object = match self.peek()? {
            Some(Token::Keyword(b"endobj")) => Object::Null,
            _ => self.object(0)?,
        };
        let object = match object {
            Object::Dict(dict) if self.peek()? == Some(Token::Keyword(b"stream")) => {
                self.lexer.next_token()?;
                Object::Stream(self.stream_body(dict, &stream_length)?)
            }
            other => other,
        };
        if self.peek()? == Some(Token::Keyword(b"endobj")) {
            self.lexer.next_token()?;
        }
        Ok((id, object))
    }

    pub fn object_header(&mut self) -> Option<ObjId> {
        let start = self.position();
        let header = (|| {
            let num = self.integer()?;
            let generation = self.integer()?;
            match self.lexer.next_token().ok()?? {
                Token::Keyword(b"obj") => Some(ObjId::new(u32::try_from(num).ok()?, u16::try_from(generation).ok()?)),
                _ => None,
            }
        })();
        if header.is_none() {
            self.lexer.seek(start);
        }
        header
    }

    fn integer(&mut self) -> Option<i64> {
        match self.lexer.next_token().ok()?? {
            Token::Int(value) => Some(value),
            _ => None,
        }
    }

    fn peek(&mut self) -> Result<Option<Token<'a>>> {
        let start = self.position();
        let token = self.lexer.next_token();
        self.lexer.seek(start);
        token
    }

    fn object(&mut self, depth: usize) -> Result<Object> {
        if depth > MAX_DEPTH {
            return Err(Error::at(self.position(), "objects nested too deeply"));
        }
        let start = self.position();
        let token = self.lexer.next_token()?.ok_or_else(|| Error::at(start, "unexpected end of file"))?;
        Ok(match token {
            Token::Int(value) => self.reference_after(value).unwrap_or(Object::Int(value)),
            Token::Real(value) => Object::Real(value),
            Token::String(bytes) => Object::String(bytes),
            Token::Name(name) => Object::Name(name),
            Token::ArrayStart => self.array(depth)?,
            Token::DictStart => Object::Dict(self.dict(depth)?),
            Token::Keyword(b"true") => Object::Bool(true),
            Token::Keyword(b"false") => Object::Bool(false),
            Token::Keyword(b"null") => Object::Null,
            Token::Keyword(word) => {
                return Err(Error::at(start, format!("unexpected keyword {}", String::from_utf8_lossy(word))));
            }
            Token::ArrayEnd | Token::DictEnd => return Err(Error::at(start, "unexpected closing delimiter")),
        })
    }

    fn reference_after(&mut self, num: i64) -> Option<Object> {
        let start = self.position();
        let reference = (|| {
            let generation = self.integer()?;
            match self.lexer.next_token().ok()?? {
                Token::Keyword(b"R") => {
                    Some(Object::Ref(ObjId::new(u32::try_from(num).ok()?, u16::try_from(generation).ok()?)))
                }
                _ => None,
            }
        })();
        if reference.is_none() {
            self.lexer.seek(start);
        }
        reference
    }

    fn array(&mut self, depth: usize) -> Result<Object> {
        let mut items = Vec::new();
        loop {
            match self.peek()? {
                Some(Token::ArrayEnd) => {
                    self.lexer.next_token()?;
                    return Ok(Object::Array(items));
                }
                None => return Err(Error::at(self.position(), "unterminated array")),
                _ => items.push(self.object(depth + 1)?),
            }
        }
    }

    fn dict(&mut self, depth: usize) -> Result<Dict> {
        let mut dict = Dict::new();
        loop {
            let start = self.position();
            match self.lexer.next_token()? {
                Some(Token::DictEnd) => return Ok(dict),
                Some(Token::Name(key)) => {
                    if self.peek()? == Some(Token::DictEnd) {
                        continue;
                    }
                    let value = self.object(depth + 1)?;
                    if value != Object::Null {
                        dict.insert(key, value);
                    }
                }
                None => return Err(Error::at(start, "unterminated dictionary")),
                Some(_) => return Err(Error::at(start, "dictionary key is not a name")),
            }
        }
    }

    fn stream_body(&mut self, dict: Dict, stream_length: &impl Fn(ObjId) -> Option<usize>) -> Result<Stream> {
        let bytes = self.lexer.bytes();
        let mut data_start = self.position();
        match bytes.get(data_start..data_start + 2) {
            Some(b"\r\n") => data_start += 2,
            Some([b'\n' | b'\r', ..]) => data_start += 1,
            _ => {}
        }
        let declared = match dict.get("Length") {
            Some(Object::Int(length)) => usize::try_from(*length).ok(),
            Some(Object::Ref(id)) => stream_length(*id),
            _ => None,
        };
        let data_end = declared
            .and_then(|length| data_start.checked_add(length))
            .filter(|&end| ends_stream_at(bytes, end))
            .or_else(|| find_endstream(bytes, data_start))
            .ok_or_else(|| Error::at(data_start, "stream without endstream"))?;
        self.lexer.seek(data_end);
        match self.lexer.next_token()? {
            Some(Token::Keyword(b"endstream")) => {}
            _ => return Err(Error::at(data_end, "expected endstream")),
        }
        Ok(Stream { dict, raw: Rc::from(&bytes[data_start..data_end]) })
    }
}

fn ends_stream_at(bytes: &[u8], end: usize) -> bool {
    if end > bytes.len() {
        return false;
    }
    let mut lexer = Lexer::new(bytes, end);
    lexer.skip_whitespace();
    bytes[lexer.position()..].starts_with(b"endstream")
}

fn find_endstream(bytes: &[u8], from: usize) -> Option<usize> {
    let offset = bytes.get(from..)?.windows(9).position(|window| window == b"endstream")?;
    let mut end = from + offset;
    if end > from && bytes[end - 1] == b'\n' {
        end -= 1;
    }
    if end > from && bytes[end - 1] == b'\r' {
        end -= 1;
    }
    Some(end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Name;

    fn parse(source: &[u8]) -> Object {
        Parser::new(source, 0).parse_object().unwrap()
    }

    #[test]
    fn parses_nested_containers_and_references() {
        let object = parse(b"<< /Kids [1 0 R 2 0 R] /Count 2 /Nested << /A [1 [2] 3.5] >> >>");
        let dict = object.as_dict().unwrap();
        assert_eq!(
            dict.get("Kids"),
            Some(&Object::Array(vec![Object::Ref(ObjId::new(1, 0)), Object::Ref(ObjId::new(2, 0))]))
        );
        assert_eq!(dict.int("Count"), Some(2));
        let nested = dict.get("Nested").unwrap().as_dict().unwrap();
        assert_eq!(
            nested.get("A"),
            Some(&Object::Array(vec![Object::Int(1), Object::Array(vec![Object::Int(2)]), Object::Real(3.5)]))
        );
    }

    #[test]
    fn keeps_plain_integers_that_only_look_like_a_reference_start() {
        assert_eq!(parse(b"[1 2 3]"), Object::Array(vec![Object::Int(1), Object::Int(2), Object::Int(3)]));
        assert_eq!(parse(b"[1 0]"), Object::Array(vec![Object::Int(1), Object::Int(0)]));
    }

    #[test]
    fn drops_null_values_from_dictionaries() {
        let object = parse(b"<< /A null /B 1 /C >>");
        let dict = object.as_dict().unwrap();
        assert!(!dict.contains("A"));
        assert_eq!(dict.int("B"), Some(1));
    }

    #[test]
    fn reads_an_indirect_stream_with_a_direct_length() {
        let source = b"7 1 obj\n<< /Length 5 >>\nstream\r\nhello\nendstream\nendobj";
        let (id, object) = Parser::new(source, 0).parse_indirect(|_| None).unwrap();
        assert_eq!(id, ObjId::new(7, 1));
        assert_eq!(&*object.as_stream().unwrap().raw, b"hello");
    }

    #[test]
    fn resolves_an_indirect_length_and_recovers_from_a_wrong_one() {
        let source = b"1 0 obj << /Length 9 0 R >> stream\nabc\nendstream endobj";
        let (_, object) = Parser::new(source, 0).parse_indirect(|_| Some(3)).unwrap();
        assert_eq!(&*object.as_stream().unwrap().raw, b"abc");
        let wrong = b"1 0 obj << /Length 99 >> stream\nabcdef\r\nendstream endobj";
        let (_, object) = Parser::new(wrong, 0).parse_indirect(|_| None).unwrap();
        assert_eq!(&*object.as_stream().unwrap().raw, b"abcdef");
    }

    #[test]
    fn rejects_garbage_with_an_offset() {
        let error = Parser::new(b"<< /A >> ]", 9).parse_object().unwrap_err();
        assert_eq!(error.offset, Some(9));
        assert!(Parser::new(b"<< 1 2 >>", 0).parse_object().is_err());
    }

    #[test]
    fn refuses_unbounded_nesting() {
        let deep = vec![b'['; 10_000];
        assert!(Parser::new(&deep, 0).parse_object().is_err());
    }

    #[test]
    fn treats_an_empty_object_as_null() {
        let (_, object) = Parser::new(b"3 0 obj endobj", 0).parse_indirect(|_| None).unwrap();
        assert_eq!(object, Object::Null);
        assert_eq!(parse(b"/Name"), Object::Name(Name::from("Name")));
    }
}
