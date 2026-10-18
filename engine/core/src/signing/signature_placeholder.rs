use std::io::Write;
use std::ops::Range;

use crate::error::{Error, Result};
use crate::syntax::Object;

const UNKNOWN: i64 = 9_999_999_999;
const BYTE_RANGE_TEXT: &[u8] = b"[0 9999999999 9999999999 9999999999]";

pub struct SignaturePlaceholder {
    byte_range: Range<usize>,
    contents: Range<usize>,
}

impl SignaturePlaceholder {
    pub fn byte_range() -> Object {
        Object::Array([0, UNKNOWN, UNKNOWN, UNKNOWN].map(Object::Int).to_vec())
    }

    pub fn contents(capacity: usize) -> Object {
        Object::String(vec![0; capacity])
    }

    pub fn find(bytes: &[u8], from: usize, capacity: usize) -> Result<SignaturePlaceholder> {
        let missing = || Error::new("the signature dictionary was not written as expected");
        let byte_range = find(bytes, from, BYTE_RANGE_TEXT).ok_or_else(missing)?;
        let mut hex = vec![b'0'; capacity * 2 + 2];
        hex[0] = b'<';
        hex[capacity * 2 + 1] = b'>';
        let contents = find(bytes, from, &hex).ok_or_else(missing)?;
        Ok(SignaturePlaceholder { byte_range, contents })
    }

    pub fn write_byte_range<'b>(&self, bytes: &'b mut [u8]) -> Result<[&'b [u8]; 2]> {
        let (before, after) = (self.contents.start, self.contents.end);
        let mut text = Vec::new();
        let _ = write!(text, "[0 {before} {after} {}", bytes.len() - after);
        let room = self.byte_range.len() - 1;
        if text.len() > room {
            return Err(Error::new("the file is too large for its signature placeholder"));
        }
        text.resize(room, b' ');
        text.push(b']');
        bytes[self.byte_range.clone()].copy_from_slice(&text);
        let (head, tail) = bytes.split_at(after);
        Ok([&head[..before], tail])
    }

    pub fn write_contents(&self, bytes: &mut [u8], signature: &[u8]) -> Result<()> {
        let room = self.contents.len() - 2;
        if signature.len() * 2 > room {
            return Err(Error::new("the signature is larger than the room kept for it"));
        }
        let hex: Vec<u8> = signature.iter().flat_map(|byte| format!("{byte:02x}").into_bytes()).collect();
        bytes[self.contents.start + 1..self.contents.start + 1 + hex.len()].copy_from_slice(&hex);
        Ok(())
    }
}

fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<Range<usize>> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|start| from + start..from + start + needle.len())
}
