use crate::error::{Error, Result};
use crate::syntax::lexer::is_whitespace;

pub fn decode_hex(data: &[u8]) -> Result<Vec<u8>> {
    let mut nibbles = Vec::with_capacity(data.len());
    for &byte in data {
        let nibble = match byte {
            b'>' => break,
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ if is_whitespace(byte) => continue,
            _ => return Err(Error::new(format!("ASCIIHexDecode: invalid byte {byte:#04x}"))),
        };
        nibbles.push(nibble);
    }
    if nibbles.len() % 2 == 1 {
        nibbles.push(0);
    }
    Ok(nibbles.chunks(2).map(|pair| pair[0] << 4 | pair[1]).collect())
}

pub fn decode_85(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() * 4 / 5);
    let mut group = [0u8; 5];
    let mut filled = 0;
    let body = data.strip_prefix(b"<~").unwrap_or(data);
    for &byte in body {
        match byte {
            b'~' => break,
            b'z' if filled == 0 => out.extend_from_slice(&[0; 4]),
            b'!'..=b'u' => {
                group[filled] = byte - b'!';
                filled += 1;
                if filled == 5 {
                    out.extend_from_slice(&group_value(&group)?.to_be_bytes());
                    filled = 0;
                }
            }
            _ if is_whitespace(byte) => {}
            _ => return Err(Error::new(format!("ASCII85Decode: invalid byte {byte:#04x}"))),
        }
    }
    if filled > 1 {
        group[filled..].fill(b'u' - b'!');
        out.extend_from_slice(&group_value(&group)?.to_be_bytes()[..filled - 1]);
    }
    Ok(out)
}

fn group_value(group: &[u8; 5]) -> Result<u32> {
    let value = group.iter().fold(0u64, |value, &digit| value * 85 + digit as u64);
    u32::try_from(value).map_err(|_| Error::new("ASCII85Decode: group out of range"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_hex_ignoring_whitespace_and_padding_an_odd_digit() {
        assert_eq!(decode_hex(b"48 65\n6c6C 6f>trailing").unwrap(), b"Hello");
        assert_eq!(decode_hex(b"7").unwrap(), vec![0x70]);
        assert!(decode_hex(b"zz").is_err());
    }

    #[test]
    fn decodes_ascii_85_groups_partial_groups_and_zero_shortcuts() {
        assert_eq!(decode_85(b"87cURD_*#TDfTZ)+T~>").unwrap(), b"Hello, world!");
        assert_eq!(decode_85(b"<~z@:E^~>").unwrap(), b"\0\0\0\0abc");
        assert_eq!(decode_85(b"87cU\nRD_*#TDfTZ)+T").unwrap(), b"Hello, world!");
        assert!(decode_85(b"{").is_err());
    }
}
