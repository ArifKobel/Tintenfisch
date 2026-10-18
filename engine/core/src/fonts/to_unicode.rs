use std::collections::HashMap;

use crate::syntax::lexer::{Lexer, Token};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToUnicode {
    map: HashMap<u32, (String, usize)>,
}

impl ToUnicode {
    pub fn parse(data: &[u8]) -> ToUnicode {
        let mut map = HashMap::new();
        let mut lexer = Lexer::new(data, 0);
        let mut operands: Vec<Token> = Vec::new();
        let mut in_array: Option<Vec<String>> = None;
        while let Ok(Some(token)) = lexer.next_token() {
            match token {
                Token::ArrayStart => in_array = Some(Vec::new()),
                Token::ArrayEnd => {
                    if let Some(items) = in_array.take() {
                        operands.push(Token::Keyword(b"array"));
                        let start = operands.len().saturating_sub(3);
                        if let [Token::String(low_bytes), Token::String(high), _] = &operands[start..] {
                            let (low, high) = (number(low_bytes), number(high));
                            for (offset, text) in items.into_iter().enumerate() {
                                if low + offset as u32 <= high {
                                    map.insert(low + offset as u32, (text, low_bytes.len()));
                                }
                            }
                        }
                        operands.truncate(start);
                    }
                }
                Token::String(bytes) if in_array.is_some() => in_array.as_mut().unwrap().push(utf16(&bytes)),
                Token::Keyword(b"endbfchar") => {
                    for pair in operands.chunks(2) {
                        if let [Token::String(code), Token::String(text)] = pair {
                            map.insert(number(code), (utf16(text), code.len()));
                        }
                    }
                    operands.clear();
                }
                Token::Keyword(b"endbfrange") => {
                    for triple in operands.chunks(3) {
                        if let [Token::String(low_bytes), Token::String(high), Token::String(start)] = triple {
                            let low = number(low_bytes);
                            let high = number(high).min(low + 0xFFFF);
                            for code in low..=high {
                                map.insert(code, (incremented(start, code - low), low_bytes.len()));
                            }
                        }
                    }
                    operands.clear();
                }
                Token::Keyword(word) if word.starts_with(b"begin") => operands.clear(),
                other => operands.push(other),
            }
        }
        ToUnicode { map }
    }

    pub fn text(&self, code: u32) -> Option<&str> {
        self.map.get(&code).map(|(text, _)| text.as_str())
    }

    pub fn code_for(&self, text: &str) -> Option<(u32, usize)> {
        self.map.iter().filter(|(_, (mapped, _))| mapped == text).map(|(&code, &(_, length))| (code, length)).min()
    }
}

fn number(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0, |value, &byte| value << 8 | byte as u32)
}

fn utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> =
        bytes.chunks(2).map(|pair| u16::from_be_bytes([pair[0], *pair.get(1).unwrap_or(&0)])).collect();
    String::from_utf16_lossy(&units)
}

fn incremented(start: &[u8], offset: u32) -> String {
    let mut bytes = start.to_vec();
    if let Some(last) = bytes.last_mut() {
        *last = last.wrapping_add(offset as u8);
    }
    utf16(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_chars_ranges_and_range_arrays() {
        let cmap = ToUnicode::parse(
            b"begincmap 2 beginbfchar <0003> <0020> <0024> <00660069> endbfchar
              1 beginbfrange <0010> <0012> <0041> endbfrange
              1 beginbfrange <0020> <0021> [<00E9> <D83DDE00>] endbfrange endcmap",
        );
        assert_eq!(cmap.text(3), Some(" "));
        assert_eq!(cmap.text(0x24), Some("fi"));
        assert_eq!((cmap.text(0x10), cmap.text(0x12)), (Some("A"), Some("C")));
        assert_eq!((cmap.text(0x20), cmap.text(0x21)), (Some("é"), Some("\u{1F600}")));
        assert_eq!(cmap.text(0x13), None);
        assert_eq!(cmap.code_for("B"), Some((0x11, 2)));
        assert_eq!(cmap.code_for("Z"), None);
    }
}
