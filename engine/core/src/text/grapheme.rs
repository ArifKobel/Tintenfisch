const ZERO_WIDTH_JOINER: char = '\u{200D}';

pub fn extends_cluster(character: char) -> bool {
    matches!(character as u32,
        0x0300..=0x036F | 0x0483..=0x0489 | 0x0591..=0x05BD | 0x0610..=0x061A | 0x064B..=0x065F
        | 0x0E31 | 0x0E34..=0x0E3A | 0x0E47..=0x0E4E | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF
        | 0x200C | 0x200D | 0x20D0..=0x20FF | 0xFE00..=0xFE0F | 0xFE20..=0xFE2F
        | 0x1F3FB..=0x1F3FF | 0xE0020..=0xE007F | 0xE0100..=0xE01EF)
}

pub fn joins(previous: &str, next: &str) -> bool {
    let Some(first) = next.chars().next() else { return false };
    extends_cluster(first)
        || previous.ends_with(ZERO_WIDTH_JOINER)
        || (is_regional_indicator(first) && ends_with_odd_regional_indicators(previous))
}

fn is_regional_indicator(character: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&character)
}

fn ends_with_odd_regional_indicators(text: &str) -> bool {
    text.chars().rev().take_while(|character| is_regional_indicator(*character)).count() % 2 == 1
}

pub fn split_graphemes(text: &str) -> Vec<String> {
    let mut clusters: Vec<String> = Vec::new();
    for character in text.chars() {
        let mut buffer = [0u8; 4];
        let piece = character.encode_utf8(&mut buffer);
        match clusters.last_mut() {
            Some(last) if joins(last, piece) => last.push(character),
            _ => clusters.push(piece.to_string()),
        }
    }
    clusters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_marks_joiners_and_flag_pairs() {
        assert!(joins("e", "\u{0301}"));
        assert!(joins("\u{1F468}\u{200D}", "\u{1F469}"));
        assert!(joins("\u{1F1E9}", "\u{1F1EA}"));
        assert!(!joins("\u{1F1E9}\u{1F1EA}", "\u{1F1E6}"));
        assert!(!joins("a", "b"));
        assert!(joins("\u{1F44D}", "\u{1F3FD}"));
    }

    #[test]
    fn splits_text_into_caret_stops() {
        assert_eq!(split_graphemes("e\u{0301}x"), ["e\u{0301}", "x"]);
        assert_eq!(split_graphemes("\u{1F468}\u{200D}\u{1F469}!"), ["\u{1F468}\u{200D}\u{1F469}", "!"]);
        assert!(split_graphemes("").is_empty());
    }
}
