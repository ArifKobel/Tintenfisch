use std::collections::HashMap;

use crate::fonts::encoding_tables::standard;
use crate::geometry::Matrix;
use crate::syntax::lexer::{Lexer, Token};

const EEXEC_KEY: u16 = 55665;
pub(super) const CHARSTRING_KEY: u16 = 4330;

pub struct Type1Program {
    pub matrix: Matrix,
    glyphs: HashMap<String, u16>,
    pub charstrings: Vec<Vec<u8>>,
    pub subroutines: Vec<Vec<u8>>,
    encoding: HashMap<u8, String>,
}

impl Type1Program {
    pub fn parse(data: &[u8], length1: usize) -> Option<Type1Program> {
        let data = strip_pfb_segments(data);
        let split = if length1 > 0 && length1 < data.len() { length1 } else { find_eexec(&data)? };
        let (clear, encrypted) = data.split_at(split);
        let encrypted = binary_or_hex(skip_leading_whitespace(encrypted));
        let private = decrypt(&encrypted, EEXEC_KEY, 4);
        let length_iv = number_after(&private, b"/lenIV").unwrap_or(4.0) as i64;
        let subroutines = read_subroutines(&private, length_iv);
        let (glyph_names, charstrings) = read_charstrings(&private, length_iv);
        let glyphs = glyph_names.into_iter().enumerate().map(|(index, name)| (name, index as u16)).collect();
        Some(Type1Program {
            matrix: read_matrix(clear).unwrap_or(Matrix::scale(0.001, 0.001)),
            glyphs,
            charstrings,
            subroutines,
            encoding: read_encoding(clear),
        })
    }

    pub fn glyph(&self, name: &str) -> Option<u16> {
        self.glyphs.get(name).copied()
    }

    pub fn builtin_name(&self, code: u8) -> Option<&str> {
        self.encoding.get(&code).map(String::as_str)
    }
}

pub fn decrypt(cipher: &[u8], key: u16, discard: usize) -> Vec<u8> {
    let mut register = key;
    let mut plain = Vec::with_capacity(cipher.len());
    for &byte in cipher {
        plain.push(byte ^ (register >> 8) as u8);
        register = (byte as u16).wrapping_add(register).wrapping_mul(52845).wrapping_add(22719);
    }
    plain.split_off(discard.min(plain.len()))
}

fn strip_pfb_segments(data: &[u8]) -> Vec<u8> {
    if data.first() != Some(&0x80) {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(data.len());
    let mut position = 0;
    while position + 6 <= data.len() && data[position] == 0x80 && data[position + 1] != 3 {
        let length =
            u32::from_le_bytes([data[position + 2], data[position + 3], data[position + 4], data[position + 5]])
                as usize;
        let start = position + 6;
        out.extend_from_slice(&data[start..(start + length).min(data.len())]);
        position = start + length;
    }
    out
}

fn find_eexec(data: &[u8]) -> Option<usize> {
    let start = data.windows(5).position(|window| window == b"eexec")? + 5;
    Some(start + data[start..].iter().take_while(|byte| byte.is_ascii_whitespace()).count())
}

fn skip_leading_whitespace(data: &[u8]) -> &[u8] {
    let skip = data.iter().take_while(|byte| matches!(byte, b' ' | b'\r' | b'\n' | b'\t')).count();
    &data[skip..]
}

fn binary_or_hex(data: &[u8]) -> Vec<u8> {
    let looks_hex = data.iter().take(4).all(|byte| byte.is_ascii_hexdigit());
    if !looks_hex {
        return data.to_vec();
    }
    let digits: Vec<u8> =
        data.iter().filter_map(|&byte| (byte as char).to_digit(16).map(|digit| digit as u8)).collect();
    digits.chunks(2).filter(|pair| pair.len() == 2).map(|pair| pair[0] << 4 | pair[1]).collect()
}

fn number_after(data: &[u8], key: &[u8]) -> Option<f64> {
    let position = data.windows(key.len()).position(|window| window == key)? + key.len();
    match Lexer::new(data, position).next_token().ok()?? {
        Token::Int(value) => Some(value as f64),
        Token::Real(value) => Some(value),
        _ => None,
    }
}

fn read_matrix(clear: &[u8]) -> Option<Matrix> {
    let position = clear.windows(11).position(|window| window == b"/FontMatrix")? + 11;
    let mut lexer = Lexer::new(clear, position);
    let mut numbers = Vec::new();
    while let Ok(Some(token)) = lexer.next_token() {
        match token {
            Token::Int(value) => numbers.push(value as f64),
            Token::Real(value) => numbers.push(value),
            Token::ArrayEnd | Token::Keyword(b"}") => break,
            _ => {}
        }
    }
    Matrix::from_numbers(&numbers)
}

fn read_encoding(clear: &[u8]) -> HashMap<u8, String> {
    let Some(position) = clear.windows(9).position(|window| window == b"/Encoding") else {
        return HashMap::new();
    };
    let mut lexer = Lexer::new(clear, position + 9);
    if let Ok(Some(Token::Keyword(b"StandardEncoding"))) = lexer.next_token() {
        return standard()
            .iter()
            .enumerate()
            .filter_map(|(code, name)| name.map(|name| (code as u8, name.to_string())))
            .collect();
    }
    let mut encoding = HashMap::new();
    let mut recent: Vec<Token> = Vec::new();
    while let Ok(Some(token)) = lexer.next_token() {
        match token {
            Token::Keyword(b"put") => {
                if let [.., Token::Int(code), Token::Name(name)] = recent.as_slice() {
                    encoding.insert(*code as u8, String::from_utf8_lossy(name.as_bytes()).into_owned());
                }
                recent.clear();
            }
            Token::Keyword(b"readonly" | b"def") => break,
            other => recent.push(other),
        }
    }
    encoding
}

fn binary_block<'a>(private: &'a [u8], lexer: &mut Lexer<'a>, length: usize, length_iv: i64) -> Option<Vec<u8>> {
    match lexer.next_token().ok()?? {
        Token::Keyword(b"RD" | b"-|") => {}
        _ => return None,
    }
    let start = lexer.position() + 1;
    let block = private.get(start..start + length)?;
    lexer.seek(start + length);
    Some(if length_iv < 0 { block.to_vec() } else { decrypt(block, CHARSTRING_KEY, length_iv as usize) })
}

fn read_subroutines(private: &[u8], length_iv: i64) -> Vec<Vec<u8>> {
    let Some(position) = private.windows(6).position(|window| window == b"/Subrs") else {
        return Vec::new();
    };
    let mut lexer = Lexer::new(private, position + 6);
    let count = match lexer.next_token() {
        Ok(Some(Token::Int(count))) => count.clamp(0, 65536) as usize,
        _ => return Vec::new(),
    };
    let mut subroutines = vec![Vec::new(); count];
    while let Ok(Some(token)) = lexer.next_token() {
        match token {
            Token::Keyword(b"dup") => {
                let (Ok(Some(Token::Int(index))), Ok(Some(Token::Int(length)))) =
                    (lexer.next_token(), lexer.next_token())
                else {
                    continue;
                };
                if let Some(block) = binary_block(private, &mut lexer, length.max(0) as usize, length_iv)
                    && let Some(slot) = subroutines.get_mut(index.max(0) as usize)
                {
                    *slot = block;
                }
            }
            Token::Name(name) if name.is("CharStrings") => break,
            _ => {}
        }
    }
    subroutines
}

fn read_charstrings(private: &[u8], length_iv: i64) -> (Vec<String>, Vec<Vec<u8>>) {
    let mut names = Vec::new();
    let mut charstrings = Vec::new();
    let Some(position) = private.windows(12).position(|window| window == b"/CharStrings") else {
        return (names, charstrings);
    };
    let mut lexer = Lexer::new(private, position + 12);
    while let Ok(Some(token)) = lexer.next_token() {
        match token {
            Token::Name(name) => {
                let Ok(Some(Token::Int(length))) = lexer.next_token() else { continue };
                if let Some(block) = binary_block(private, &mut lexer, length.max(0) as usize, length_iv) {
                    names.push(String::from_utf8_lossy(name.as_bytes()).into_owned());
                    charstrings.push(block);
                }
            }
            Token::Keyword(b"end") if !names.is_empty() => break,
            _ => {}
        }
    }
    (names, charstrings)
}
