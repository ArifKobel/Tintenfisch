pub fn encode_text(text: &str) -> Vec<u8> {
    if text.chars().all(|character| character.is_ascii() && !character.is_ascii_control()) {
        return text.as_bytes().to_vec();
    }
    let mut bytes = vec![0xFE, 0xFF];
    bytes.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
    bytes
}

pub fn decode_text(bytes: &[u8]) -> String {
    if let Some(units) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = units.as_chunks::<2>().0.iter().map(|pair| u16::from_be_bytes(*pair)).collect();
        return String::from_utf16_lossy(&units);
    }
    if let Some(utf8) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(utf8).into_owned();
    }
    bytes.iter().map(|&byte| byte as char).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trips_through_either_encoding() {
        for text in ["Plain note", "Grüße, Ω and 漢字"] {
            assert_eq!(decode_text(&encode_text(text)), text);
        }
        assert_eq!(encode_text("Plain note"), b"Plain note");
    }
}
