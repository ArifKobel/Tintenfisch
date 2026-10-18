use std::io::Read;

use flate2::read::{DeflateDecoder, ZlibDecoder};

use crate::error::{Error, Result};

pub fn inflate(data: &[u8]) -> Result<Vec<u8>> {
    let zlib = read_lenient(ZlibDecoder::new(data));
    if !zlib.is_empty() || data.is_empty() {
        return Ok(zlib);
    }
    let raw = read_lenient(DeflateDecoder::new(data));
    if raw.is_empty() {
        return Err(Error::new("FlateDecode: data is neither zlib nor deflate"));
    }
    Ok(raw)
}

fn read_lenient(mut decoder: impl Read) -> Vec<u8> {
    let mut out = Vec::new();
    let mut chunk = [0u8; 16 * 1024];
    while let Ok(read) = decoder.read(&mut chunk) {
        if read == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..read]);
    }
    out
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::{DeflateEncoder, ZlibEncoder};

    use super::*;

    fn zlib(data: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn inflates_zlib_data() {
        let text = b"BT /F1 12 Tf (Hello) Tj ET ".repeat(50);
        assert_eq!(inflate(&zlib(&text)).unwrap(), text);
    }

    #[test]
    fn falls_back_to_raw_deflate() {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"raw deflate body").unwrap();
        assert_eq!(inflate(&encoder.finish().unwrap()).unwrap(), b"raw deflate body");
    }

    #[test]
    fn keeps_the_readable_prefix_of_a_truncated_stream() {
        let text = (0..4000u32).flat_map(|n| n.to_le_bytes()).collect::<Vec<_>>();
        let compressed = zlib(&text);
        let partial = inflate(&compressed[..compressed.len() / 2]).unwrap();
        assert!(!partial.is_empty());
        assert_eq!(&text[..partial.len()], &partial[..]);
    }

    #[test]
    fn rejects_noise() {
        assert!(inflate(&[0xff; 32]).is_err());
    }
}
