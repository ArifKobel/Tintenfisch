use crate::error::{Error, Result};

const SEARCH_WINDOW: usize = 1024;
const MARKER: &[u8] = b"%PDF-";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub offset: usize,
    pub major: u8,
    pub minor: u8,
}

pub fn read_header(bytes: &[u8]) -> Result<Header> {
    let window = &bytes[..bytes.len().min(SEARCH_WINDOW)];
    let offset = window
        .windows(MARKER.len())
        .position(|candidate| candidate == MARKER)
        .ok_or_else(|| Error::new("not a PDF file: no %PDF- header"))?;
    let version = &bytes[offset + MARKER.len()..];
    match version {
        [major @ b'0'..=b'9', b'.', minor @ b'0'..=b'9', ..] => {
            Ok(Header { offset, major: major - b'0', minor: minor - b'0' })
        }
        _ => Err(Error::at(offset, "malformed PDF version")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_plain_header() {
        let header = read_header(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n").unwrap();
        assert_eq!(header, Header { offset: 0, major: 1, minor: 7 });
    }

    #[test]
    fn tolerates_leading_garbage() {
        let header = read_header(b"\x00\x00junk%PDF-2.0\n").unwrap();
        assert_eq!(header, Header { offset: 6, major: 2, minor: 0 });
    }

    #[test]
    fn rejects_files_without_a_header() {
        assert!(read_header(b"hello world").is_err());
        assert!(read_header(b"%PDF-x.y").is_err());
        assert!(read_header(b"%PDF-1").is_err());
    }
}
