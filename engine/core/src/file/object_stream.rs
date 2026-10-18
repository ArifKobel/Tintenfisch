use crate::error::{Error, Result};
use crate::syntax::lexer::{Lexer, Token};
use crate::syntax::{Dict, Object, Parser};

#[derive(Clone, Debug)]
pub struct ObjectStream {
    data: Vec<u8>,
    first: usize,
    members: Vec<(u32, usize)>,
}

impl ObjectStream {
    pub fn parse(dict: &Dict, data: Vec<u8>) -> Result<ObjectStream> {
        let count = dict
            .int("N")
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| Error::new("object stream without /N"))?;
        let first = dict
            .int("First")
            .and_then(|first| usize::try_from(first).ok())
            .filter(|&first| first <= data.len())
            .ok_or_else(|| Error::new("object stream without a valid /First"))?;
        let mut lexer = Lexer::new(&data[..first], 0);
        let mut members = Vec::with_capacity(count);
        for _ in 0..count {
            match (lexer.next_token()?, lexer.next_token()?) {
                (Some(Token::Int(num)), Some(Token::Int(offset))) => members.push((
                    u32::try_from(num).map_err(|_| Error::new("object stream member number out of range"))?,
                    usize::try_from(offset).map_err(|_| Error::new("object stream member offset out of range"))?,
                )),
                _ => break,
            }
        }
        Ok(ObjectStream { data, first, members })
    }

    pub fn members(&self) -> impl Iterator<Item = u32> + '_ {
        self.members.iter().map(|&(num, _)| num)
    }

    pub fn object(&self, index: usize) -> Result<(u32, Object)> {
        let &(num, offset) =
            self.members.get(index).ok_or_else(|| Error::new(format!("object stream has no member {index}")))?;
        let object = Parser::new(&self.data, self.first + offset).parse_object()?;
        Ok((num, object))
    }

    pub fn index_of(&self, num: u32) -> Option<usize> {
        self.members.iter().position(|&(member, _)| member == num)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Name;

    fn stream(header: &str, body: &str) -> ObjectStream {
        let data = format!("{header}{body}").into_bytes();
        let dict: Dict = [
            (Name::from("N"), Object::Int(header.split_whitespace().count() as i64 / 2)),
            (Name::from("First"), Object::Int(header.len() as i64)),
        ]
        .into_iter()
        .collect();
        ObjectStream::parse(&dict, data).unwrap()
    }

    #[test]
    fn reads_members_by_index() {
        let objects = stream("10 0 11 8 ", "<</A 1>>[1 2]");
        assert_eq!(objects.members().collect::<Vec<_>>(), vec![10, 11]);
        assert_eq!(objects.object(1).unwrap(), (11, Object::Array(vec![Object::Int(1), Object::Int(2)])));
        assert_eq!(objects.index_of(10), Some(0));
        assert!(objects.object(2).is_err());
    }

    #[test]
    fn rejects_a_first_offset_beyond_the_data() {
        let dict: Dict =
            [(Name::from("N"), Object::Int(1)), (Name::from("First"), Object::Int(99))].into_iter().collect();
        assert!(ObjectStream::parse(&dict, b"1 0 null".to_vec()).is_err());
    }
}
