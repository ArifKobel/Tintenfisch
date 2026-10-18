use std::collections::{BTreeMap, HashSet};

use crate::error::{Error, Result};
use crate::file::filters::FilterChain;
use crate::file::xref::{SectionFormat, Xref, XrefEntry, XrefSection};
use crate::syntax::lexer::{Lexer, Token};
use crate::syntax::{Object, Parser, Stream};

const STARTXREF_WINDOW: usize = 4096;

pub struct XrefReader<'a> {
    bytes: &'a [u8],
}

impl<'a> XrefReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub fn read(&self) -> Result<Xref> {
        let start = self.startxref()?;
        let mut sections = Vec::new();
        let mut visited = HashSet::new();
        let mut next = Some(start);
        while let Some(offset) = next {
            if !visited.insert(offset) {
                break;
            }
            let section = self.section(offset)?;
            next = section.trailer.int("Prev").and_then(|prev| usize::try_from(prev).ok());
            sections.push(section);
        }
        Ok(Xref::from_sections(sections, start))
    }

    fn startxref(&self) -> Result<usize> {
        let window_start = self.bytes.len().saturating_sub(STARTXREF_WINDOW);
        let keyword = self.bytes[window_start..]
            .windows(9)
            .rposition(|window| window == b"startxref")
            .ok_or_else(|| Error::new("no startxref"))?;
        let mut lexer = Lexer::new(self.bytes, window_start + keyword + 9);
        match lexer.next_token()? {
            Some(Token::Int(offset)) => usize::try_from(offset)
                .ok()
                .filter(|&offset| offset < self.bytes.len())
                .ok_or_else(|| Error::new("startxref points outside the file")),
            _ => Err(Error::new("startxref without an offset")),
        }
    }

    fn section(&self, offset: usize) -> Result<XrefSection> {
        let mut lexer = Lexer::new(self.bytes, offset);
        lexer.skip_whitespace();
        if self.bytes[lexer.position()..].starts_with(b"xref") {
            let mut section = self.table(lexer.position() + 4)?;
            if let Some(stream_offset) = section.trailer.int("XRefStm").and_then(|value| usize::try_from(value).ok()) {
                let hidden = self.stream_section(stream_offset)?;
                for (num, entry) in hidden.entries {
                    let table_entry = section.entries.entry(num).or_insert(entry);
                    if table_entry.is_free() {
                        *table_entry = entry;
                    }
                }
            }
            Ok(section)
        } else {
            self.stream_section(lexer.position())
        }
    }

    fn table(&self, position: usize) -> Result<XrefSection> {
        let mut lexer = Lexer::new(self.bytes, position);
        let mut entries = BTreeMap::new();
        loop {
            let at = lexer.position();
            match lexer.next_token()? {
                Some(Token::Keyword(b"trailer")) => break,
                Some(Token::Int(first)) => {
                    let Some(Token::Int(count)) = lexer.next_token()? else {
                        return Err(Error::at(at, "xref subsection without a count"));
                    };
                    for num in first..first + count {
                        let entry = table_entry(&mut lexer)?;
                        let num = u32::try_from(num).map_err(|_| Error::at(at, "xref object number out of range"))?;
                        entries.insert(num, entry);
                    }
                }
                _ => return Err(Error::at(at, "malformed xref table")),
            }
        }
        let mut parser = Parser::new(self.bytes, lexer.position());
        let trailer = match parser.parse_object()? {
            Object::Dict(dict) => dict,
            _ => return Err(Error::at(lexer.position(), "trailer is not a dictionary")),
        };
        Ok(XrefSection { format: SectionFormat::Table, entries, trailer })
    }

    fn stream_section(&self, offset: usize) -> Result<XrefSection> {
        let (_, object) = Parser::new(self.bytes, offset).parse_indirect(|_| None)?;
        let stream = match object {
            Object::Stream(stream) if stream.dict.has_type("XRef") => stream,
            _ => return Err(Error::at(offset, "expected a cross-reference stream")),
        };
        let entries = stream_entries(&stream).map_err(|error| Error::at(offset, error.message))?;
        Ok(XrefSection { format: SectionFormat::Stream, entries, trailer: stream.dict })
    }
}

fn table_entry(lexer: &mut Lexer) -> Result<XrefEntry> {
    let at = lexer.position();
    match (lexer.next_token()?, lexer.next_token()?, lexer.next_token()?) {
        (Some(Token::Int(offset)), Some(Token::Int(generation)), Some(Token::Keyword(kind))) => {
            let generation = u16::try_from(generation).unwrap_or(u16::MAX);
            match kind {
                b"n" => Ok(XrefEntry::InUse { offset: usize::try_from(offset).unwrap_or(0), generation }),
                b"f" => Ok(XrefEntry::Free { next_generation: generation }),
                _ => Err(Error::at(at, "xref entry is neither n nor f")),
            }
        }
        _ => Err(Error::at(at, "malformed xref entry")),
    }
}

fn stream_entries(stream: &Stream) -> Result<BTreeMap<u32, XrefEntry>> {
    let widths: Vec<usize> = stream
        .dict
        .get("W")
        .and_then(Object::as_array)
        .map(|items| items.iter().filter_map(|item| item.as_int().and_then(|w| usize::try_from(w).ok())).collect())
        .filter(|widths: &Vec<usize>| widths.len() == 3 && widths.iter().all(|&w| w <= 8))
        .ok_or_else(|| Error::new("xref stream without a valid /W"))?;
    let size = stream.dict.int("Size").unwrap_or(0);
    let index: Vec<i64> = match stream.dict.get("Index").and_then(Object::as_array) {
        Some(items) => items.iter().filter_map(Object::as_int).collect(),
        None => vec![0, size],
    };
    let data = FilterChain::of(&stream.dict, Object::clone)?.decode(&stream.raw)?.data;
    let row_width: usize = widths.iter().sum();
    if row_width == 0 {
        return Err(Error::new("xref stream rows have no width"));
    }
    let mut rows = data.chunks_exact(row_width);
    let mut entries = BTreeMap::new();
    for &[first, count] in index.as_chunks::<2>().0 {
        for num in first..first + count {
            let Some(row) = rows.next() else {
                return Ok(entries);
            };
            let (kind, rest) = row.split_at(widths[0]);
            let (second, third) = rest.split_at(widths[1]);
            let kind = if widths[0] == 0 { 1 } else { big_endian(kind) };
            let (second, third) = (big_endian(second), big_endian(third));
            let entry = match kind {
                0 => XrefEntry::Free { next_generation: third as u16 },
                1 => XrefEntry::InUse { offset: second as usize, generation: third as u16 },
                2 => XrefEntry::Compressed { stream: second as u32, index: third as u32 },
                _ => continue,
            };
            if let Ok(num) = u32::try_from(num) {
                entries.insert(num, entry);
            }
        }
    }
    Ok(entries)
}

fn big_endian(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0, |value, &byte| value << 8 | byte as u64)
}
