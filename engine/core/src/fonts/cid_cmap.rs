use crate::syntax::Name;
use crate::syntax::lexer::{Lexer, Token};

#[derive(Clone, Debug, PartialEq)]
struct CodespaceRange {
    length: usize,
    low: u32,
    high: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct CidRange {
    length: usize,
    low: u32,
    high: u32,
    cid: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CidCmap {
    codespaces: Vec<CodespaceRange>,
    ranges: Vec<CidRange>,
    identity: bool,
    pub vertical: bool,
}

impl CidCmap {
    pub fn identity(vertical: bool) -> Self {
        Self {
            codespaces: vec![CodespaceRange { length: 2, low: 0, high: 0xFFFF }],
            ranges: Vec::new(),
            identity: true,
            vertical,
        }
    }

    pub fn is_identity(&self) -> bool {
        self.identity
    }

    pub fn predefined(name: &Name) -> Self {
        Self::identity(name.as_bytes().ends_with(b"-V"))
    }

    pub fn parse(data: &[u8]) -> Self {
        let mut cmap = CidCmap { codespaces: Vec::new(), ranges: Vec::new(), identity: false, vertical: false };
        let mut lexer = Lexer::new(data, 0);
        let mut operands: Vec<Token> = Vec::new();
        while let Ok(Some(token)) = lexer.next_token() {
            match token {
                Token::Keyword(b"endcodespacerange") => {
                    for pair in operands.chunks(2) {
                        if let [Token::String(low), Token::String(high)] = pair {
                            cmap.codespaces.push(CodespaceRange {
                                length: low.len(),
                                low: number(low),
                                high: number(high),
                            });
                        }
                    }
                    operands.clear();
                }
                Token::Keyword(b"endcidrange") => {
                    for triple in operands.chunks(3) {
                        if let [Token::String(low), Token::String(high), Token::Int(cid)] = triple {
                            cmap.ranges.push(CidRange {
                                length: low.len(),
                                low: number(low),
                                high: number(high),
                                cid: *cid as u32,
                            });
                        }
                    }
                    operands.clear();
                }
                Token::Keyword(b"endcidchar") => {
                    for pair in operands.chunks(2) {
                        if let [Token::String(code), Token::Int(cid)] = pair {
                            let value = number(code);
                            cmap.ranges.push(CidRange {
                                length: code.len(),
                                low: value,
                                high: value,
                                cid: *cid as u32,
                            });
                        }
                    }
                    operands.clear();
                }
                Token::Name(name) if name.is("WMode") => {
                    if let Ok(Some(Token::Int(mode))) = lexer.next_token() {
                        cmap.vertical = mode == 1;
                    }
                }
                Token::Keyword(word) if word.starts_with(b"begin") => operands.clear(),
                Token::Keyword(_) => operands.clear(),
                other => operands.push(other),
            }
        }
        if cmap.codespaces.is_empty() {
            cmap.codespaces.push(CodespaceRange { length: 2, low: 0, high: 0xFFFF });
        }
        cmap
    }

    pub fn next_code(&self, bytes: &[u8]) -> (u32, usize) {
        for length in 1..=4.min(bytes.len()) {
            let value = number(&bytes[..length]);
            if self.codespaces.iter().any(|range| range.length == length && (range.low..=range.high).contains(&value)) {
                return (value, length);
            }
        }
        let length = self.codespaces.iter().map(|range| range.length).min().unwrap_or(1).min(bytes.len()).max(1);
        (number(&bytes[..length]), length)
    }

    pub fn cid(&self, code: u32, length: usize) -> u32 {
        if self.identity {
            return code;
        }
        self.ranges
            .iter()
            .rev()
            .find(|range| range.length == length && (range.low..=range.high).contains(&code))
            .map_or(0, |range| range.cid + (code - range.low))
    }
}

fn number(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0, |value, &byte| value << 8 | byte as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_mixed_width_codes_and_maps_ranges() {
        let cmap = CidCmap::parse(
            b"/WMode 0 def 2 begincodespacerange <00> <80> <8140> <FFFF> endcodespacerange
              1 begincidrange <20> <7e> 1 endcidrange
              1 begincidrange <8140> <817e> 633 endcidrange
              1 begincidchar <8150> 9000 endcidchar",
        );
        assert_eq!(cmap.next_code(b"A\x81\x41"), (0x41, 1));
        assert_eq!(cmap.next_code(b"\x81\x41"), (0x8141, 2));
        assert_eq!(cmap.cid(0x41, 1), 34);
        assert_eq!(cmap.cid(0x8141, 2), 634);
        assert_eq!(cmap.cid(0x8150, 2), 9000);
        assert!(!cmap.vertical);
    }

    #[test]
    fn treats_identity_as_two_byte_codes() {
        let cmap = CidCmap::identity(false);
        assert_eq!(cmap.next_code(b"\x01\x02\x03"), (0x0102, 2));
        assert_eq!(cmap.cid(0x0102, 2), 0x0102);
        assert!(CidCmap::predefined(&Name::from("Identity-V")).vertical);
    }
}
