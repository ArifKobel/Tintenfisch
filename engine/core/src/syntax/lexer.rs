use crate::error::{Error, Result};
use crate::syntax::Name;

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'a> {
    Int(i64),
    Real(f64),
    String(Vec<u8>),
    Name(Name),
    Keyword(&'a [u8]),
    ArrayStart,
    ArrayEnd,
    DictStart,
    DictEnd,
}

pub fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b'\0' | b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

pub fn is_delimiter(byte: u8) -> bool {
    matches!(byte, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

fn is_regular(byte: u8) -> bool {
    !is_whitespace(byte) && !is_delimiter(byte)
}

pub struct Lexer<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(bytes: &'a [u8], position: usize) -> Self {
        Self { bytes, position }
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn seek(&mut self, position: usize) {
        self.position = position.min(self.bytes.len());
    }

    pub fn skip_whitespace(&mut self) {
        while let Some(&byte) = self.bytes.get(self.position) {
            if is_whitespace(byte) {
                self.position += 1;
            } else if byte == b'%' {
                while self.bytes.get(self.position).is_some_and(|&b| b != b'\n' && b != b'\r') {
                    self.position += 1;
                }
            } else {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> Result<Option<Token<'a>>> {
        self.skip_whitespace();
        let start = self.position;
        let Some(&byte) = self.bytes.get(start) else {
            return Ok(None);
        };
        let token = match byte {
            b'[' => {
                self.position += 1;
                Token::ArrayStart
            }
            b']' => {
                self.position += 1;
                Token::ArrayEnd
            }
            b'<' if self.bytes.get(start + 1) == Some(&b'<') => {
                self.position += 2;
                Token::DictStart
            }
            b'>' if self.bytes.get(start + 1) == Some(&b'>') => {
                self.position += 2;
                Token::DictEnd
            }
            b'<' => Token::String(self.hex_string()?),
            b'(' => Token::String(self.literal_string()?),
            b'/' => Token::Name(self.name()),
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.number(),
            b')' | b'>' | b'{' | b'}' => {
                self.position += 1;
                Token::Keyword(&self.bytes[start..start + 1])
            }
            _ => Token::Keyword(self.regular_run()),
        };
        Ok(Some(token))
    }

    fn regular_run(&mut self) -> &'a [u8] {
        let start = self.position;
        while self.bytes.get(self.position).is_some_and(|&b| is_regular(b)) {
            self.position += 1;
        }
        &self.bytes[start..self.position]
    }

    fn number(&mut self) -> Token<'a> {
        let run = self.regular_run();
        let signs = run.iter().take_while(|&&b| b == b'+' || b == b'-').count();
        let sign = if run[..signs].contains(&b'-') { -1 } else { 1 };
        let body = &run[signs..];
        if !body.contains(&b'.')
            && let Some(value) = std::str::from_utf8(body).ok().and_then(|text| text.parse::<i64>().ok())
        {
            return Token::Int(sign * value);
        }
        Token::Real(sign as f64 * lenient_real(body))
    }

    fn name(&mut self) -> Name {
        self.position += 1;
        let run = self.regular_run();
        let mut name = Vec::with_capacity(run.len());
        let mut index = 0;
        while index < run.len() {
            if run[index] == b'#'
                && let Some(value) = run.get(index + 1..index + 3).and_then(decode_hex_pair)
            {
                name.push(value);
                index += 3;
                continue;
            }
            name.push(run[index]);
            index += 1;
        }
        Name::new(name)
    }

    fn hex_string(&mut self) -> Result<Vec<u8>> {
        let start = self.position;
        self.position += 1;
        let mut nibbles = Vec::new();
        loop {
            let Some(&byte) = self.bytes.get(self.position) else {
                return Err(Error::at(start, "unterminated hex string"));
            };
            self.position += 1;
            match byte {
                b'>' => break,
                _ if is_whitespace(byte) => {}
                _ => nibbles.push(hex_value(byte).ok_or_else(|| Error::at(self.position - 1, "invalid hex digit"))?),
            }
        }
        if nibbles.len() % 2 == 1 {
            nibbles.push(0);
        }
        Ok(nibbles.chunks(2).map(|pair| pair[0] << 4 | pair[1]).collect())
    }

    fn literal_string(&mut self) -> Result<Vec<u8>> {
        let start = self.position;
        self.position += 1;
        let mut depth = 1;
        let mut out = Vec::new();
        loop {
            let Some(&byte) = self.bytes.get(self.position) else {
                return Err(Error::at(start, "unterminated literal string"));
            };
            self.position += 1;
            match byte {
                b'(' => {
                    depth += 1;
                    out.push(byte);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(out);
                    }
                    out.push(byte);
                }
                b'\\' => self.escape(&mut out),
                b'\r' => {
                    if self.bytes.get(self.position) == Some(&b'\n') {
                        self.position += 1;
                    }
                    out.push(b'\n');
                }
                _ => out.push(byte),
            }
        }
    }

    fn escape(&mut self, out: &mut Vec<u8>) {
        let Some(&byte) = self.bytes.get(self.position) else {
            return;
        };
        self.position += 1;
        match byte {
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'b' => out.push(0x08),
            b'f' => out.push(0x0c),
            b'\r' => {
                if self.bytes.get(self.position) == Some(&b'\n') {
                    self.position += 1;
                }
            }
            b'\n' => {}
            b'0'..=b'7' => {
                let mut value = (byte - b'0') as u32;
                for _ in 0..2 {
                    match self.bytes.get(self.position) {
                        Some(&digit @ b'0'..=b'7') => {
                            value = value * 8 + (digit - b'0') as u32;
                            self.position += 1;
                        }
                        _ => break,
                    }
                }
                out.push(value as u8);
            }
            _ => out.push(byte),
        }
    }
}

fn lenient_real(body: &[u8]) -> f64 {
    let mut mantissa = 0.0;
    let mut fraction_digits = None;
    for &byte in body {
        match byte {
            b'0'..=b'9' => {
                mantissa = mantissa * 10.0 + (byte - b'0') as f64;
                if let Some(count) = fraction_digits.as_mut() {
                    *count += 1;
                }
            }
            b'.' if fraction_digits.is_none() => fraction_digits = Some(0),
            b'.' => {}
            _ => break,
        }
    }
    mantissa / 10f64.powi(fraction_digits.unwrap_or(0))
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn decode_hex_pair(pair: &[u8]) -> Option<u8> {
    Some(hex_value(pair[0])? << 4 | hex_value(pair[1])?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(source: &[u8]) -> Vec<Token<'_>> {
        let mut lexer = Lexer::new(source, 0);
        std::iter::from_fn(|| lexer.next_token().unwrap()).collect()
    }

    #[test]
    fn reads_numbers_including_sloppy_forms() {
        assert_eq!(
            tokens(b"0 -12 +7 3.5 -.25 4. --3 1.2.3"),
            vec![
                Token::Int(0),
                Token::Int(-12),
                Token::Int(7),
                Token::Real(3.5),
                Token::Real(-0.25),
                Token::Real(4.0),
                Token::Int(-3),
                Token::Real(1.23),
            ]
        );
    }

    #[test]
    fn reads_literal_strings_with_nesting_and_escapes() {
        assert_eq!(
            tokens(b"(a (b) c) (\\n\\(\\)\\\\) (\\101\\7x) (split\\\nline) (cr\r\nlf)"),
            vec![
                Token::String(b"a (b) c".to_vec()),
                Token::String(b"\n()\\".to_vec()),
                Token::String(b"A\x07x".to_vec()),
                Token::String(b"splitline".to_vec()),
                Token::String(b"cr\nlf".to_vec()),
            ]
        );
    }

    #[test]
    fn reads_hex_strings_padding_an_odd_digit() {
        assert_eq!(tokens(b"<48 65 6c6C6f> <7>"), vec![Token::String(b"Hello".to_vec()), Token::String(vec![0x70])]);
    }

    #[test]
    fn decodes_name_escapes() {
        assert_eq!(
            tokens(b"/A#20B /#2Fslash"),
            vec![Token::Name(Name::from("A B")), Token::Name(Name::from("/slash"))]
        );
    }

    #[test]
    fn skips_comments_and_splits_on_delimiters() {
        assert_eq!(
            tokens(b"%comment\n<</Type/Page>>[1 R]true"),
            vec![
                Token::DictStart,
                Token::Name(Name::from("Type")),
                Token::Name(Name::from("Page")),
                Token::DictEnd,
                Token::ArrayStart,
                Token::Int(1),
                Token::Keyword(b"R"),
                Token::ArrayEnd,
                Token::Keyword(b"true"),
            ]
        );
    }

    #[test]
    fn reports_unterminated_strings() {
        assert!(Lexer::new(b"(open", 0).next_token().is_err());
        assert!(Lexer::new(b"<4142", 0).next_token().is_err());
    }
}
