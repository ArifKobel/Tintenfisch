use crate::syntax::{Object, Serializer};

#[derive(Default)]
pub struct ContentWriter {
    out: Vec<u8>,
}

impl ContentWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn operation(&mut self, operator: &str, operands: &[Object]) -> &mut Self {
        for operand in operands {
            Serializer::new(&mut self.out).object(operand);
            self.out.push(b' ');
        }
        self.out.extend_from_slice(operator.as_bytes());
        self.out.push(b'\n');
        self
    }

    pub fn finish(self) -> Vec<u8> {
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ContentParser;
    use crate::syntax::Name;

    #[test]
    fn writes_operations_the_parser_reads_back() {
        let mut writer = ContentWriter::new();
        writer
            .operation("BT", &[])
            .operation("Tf", &[Object::Name(Name::from("F1")), Object::Real(11.5)])
            .operation("TJ", &[Object::Array(vec![Object::String(b"a(b)".to_vec()), Object::Int(-250)])])
            .operation("ET", &[]);
        let data = writer.finish();
        let operations = ContentParser::new(&data, 0).parse(0);
        assert_eq!(operations.len(), 4);
        assert_eq!(operations[1].operands, vec![Object::Name(Name::from("F1")), Object::Real(11.5)]);
        assert_eq!(operations[2].operands[0], Object::Array(vec![Object::String(b"a(b)".to_vec()), Object::Int(-250)]));
    }
}
