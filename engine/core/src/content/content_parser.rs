use crate::content::Operation;
use crate::syntax::lexer::{Token, is_delimiter, is_whitespace};
use crate::syntax::{Dict, Object, Parser};

pub struct ContentParser<'a> {
    data: &'a [u8],
    segment: usize,
}

impl<'a> ContentParser<'a> {
    pub fn new(data: &'a [u8], segment: usize) -> Self {
        Self { data, segment }
    }

    pub fn parse(&self, first_ordinal: usize) -> Vec<Operation> {
        let mut operations = Vec::new();
        let mut parser = Parser::new(self.data, 0);
        let mut operands = Vec::new();
        let mut start = None;
        loop {
            parser.lexer().skip_whitespace();
            let at = parser.position();
            let token = match parser.lexer().next_token() {
                Ok(Some(token)) => token,
                Ok(None) | Err(_) => break,
            };
            match token {
                Token::Keyword(word) if !matches!(word, b"true" | b"false" | b"null") => {
                    let operator = String::from_utf8_lossy(word).into_owned();
                    let begin = start.take().unwrap_or(at);
                    let operation = if operator == "BI" {
                        match self.inline_image(&mut parser) {
                            Some(operands) => Operation {
                                operator,
                                operands,
                                segment: self.segment,
                                span: begin..parser.position(),
                                ordinal: first_ordinal + operations.len(),
                            },
                            None => break,
                        }
                    } else {
                        Operation {
                            operator,
                            operands: std::mem::take(&mut operands),
                            segment: self.segment,
                            span: begin..parser.position(),
                            ordinal: first_ordinal + operations.len(),
                        }
                    };
                    operands.clear();
                    operations.push(operation);
                }
                _ => {
                    parser.lexer().seek(at);
                    match parser.parse_object() {
                        Ok(operand) => {
                            start.get_or_insert(at);
                            operands.push(operand);
                        }
                        Err(_) => {
                            parser.lexer().seek(at);
                            let _ = parser.lexer().next_token();
                            if parser.position() == at {
                                break;
                            }
                        }
                    }
                }
            }
        }
        operations
    }

    fn inline_image(&self, parser: &mut Parser) -> Option<Vec<Object>> {
        let mut dict = Dict::new();
        loop {
            parser.lexer().skip_whitespace();
            let at = parser.position();
            match parser.lexer().next_token().ok()?? {
                Token::Keyword(b"ID") => break,
                Token::Name(key) => {
                    let value = parser.parse_object().ok()?;
                    dict.insert(key, value);
                }
                _ => {
                    parser.lexer().seek(at);
                    return None;
                }
            }
        }
        let data_start = parser.position() + 1;
        let declared = dict.int("L").or_else(|| dict.int("Length")).and_then(|length| usize::try_from(length).ok());
        let data_end = declared
            .map(|length| data_start + length)
            .filter(|&end| self.ends_image_at(end))
            .or_else(|| self.find_end_marker(data_start))?;
        let mut after = data_end;
        while self.data.get(after).is_some_and(|&byte| is_whitespace(byte)) {
            after += 1;
        }
        parser.lexer().seek(after + 2);
        let data = self.data.get(data_start..data_end)?.to_vec();
        Some(vec![Object::Dict(dict), Object::String(data)])
    }

    fn ends_image_at(&self, end: usize) -> bool {
        let mut position = end;
        while self.data.get(position).is_some_and(|&byte| is_whitespace(byte)) {
            position += 1;
        }
        self.data.get(position..position + 2) == Some(b"EI") && self.is_boundary(position + 2)
    }

    fn find_end_marker(&self, from: usize) -> Option<usize> {
        (from..self.data.len().saturating_sub(1)).find_map(|position| {
            let marker = self.data.get(position..position + 2)? == b"EI";
            let preceded = position > from && is_whitespace(self.data[position - 1]);
            (marker && preceded && self.is_boundary(position + 2)).then_some(position - 1)
        })
    }

    fn is_boundary(&self, position: usize) -> bool {
        self.data.get(position).is_none_or(|&byte| is_whitespace(byte) || is_delimiter(byte))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Name;

    fn parse(data: &[u8]) -> Vec<Operation> {
        ContentParser::new(data, 0).parse(0)
    }

    #[test]
    fn splits_operations_with_operands_and_spans() {
        let data = b"q 1 0 0 1 72 720 cm\nBT /F1 12 Tf [(Hel) -20 (lo)] TJ ET Q";
        let operations = parse(data);
        let operators: Vec<&str> = operations.iter().map(|operation| operation.operator.as_str()).collect();
        assert_eq!(operators, ["q", "cm", "BT", "Tf", "TJ", "ET", "Q"]);
        assert_eq!(&data[operations[1].span.clone()], b"1 0 0 1 72 720 cm");
        assert_eq!(operations[3].operands, vec![Object::Name(Name::from("F1")), Object::Int(12)]);
        assert_eq!(&data[operations[4].span.clone()], b"[(Hel) -20 (lo)] TJ");
        assert_eq!(
            operations.iter().map(|operation| operation.ordinal).collect::<Vec<_>>(),
            (0..7).collect::<Vec<_>>()
        );
    }

    #[test]
    fn reads_an_inline_image_with_binary_data() {
        let data = b"q BI /W 2 /H 1 /CS /G /BPC 8 ID \x00EI\xff EI Q";
        let operations = parse(data);
        assert_eq!(operations.len(), 3);
        let image = &operations[1];
        assert!(image.is("BI"));
        assert_eq!(image.operands[0].as_dict().unwrap().int("W"), Some(2));
        assert_eq!(image.operands[1], Object::String(b"\x00EI\xff".to_vec()));
        assert!(operations[2].is("Q"));
    }

    #[test]
    fn honours_an_inline_image_length() {
        let data = b"BI /W 1 /H 1 /L 4 ID  EI\nEI Q";
        let operations = parse(data);
        assert_eq!(operations[0].operands[1], Object::String(b" EI\n".to_vec()));
        assert!(operations[1].is("Q"));
    }

    #[test]
    fn survives_garbage_between_operations() {
        let operations = parse(b"q ] 1 0 0 rg >> Q (unterminated");
        let operators: Vec<&str> = operations.iter().map(|operation| operation.operator.as_str()).collect();
        assert_eq!(operators, ["q", "rg", "Q"]);
    }
}
