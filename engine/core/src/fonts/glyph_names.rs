use crate::fonts::encoding_tables::{LATIN_LOWER_ACCENTED, LATIN_SYMBOLS, LATIN_UPPER_ACCENTED};

pub fn unicode_of(name: &str) -> Option<char> {
    let name = name.split('.').next().unwrap_or(name);
    if let [byte] = name.as_bytes()
        && byte.is_ascii_alphabetic()
    {
        return Some(*byte as char);
    }
    if let Some(value) = named(name) {
        return char::from_u32(value);
    }
    if let Some(position) = LATIN_SYMBOLS.iter().skip(1).position(|candidate| *candidate == name) {
        return char::from_u32(0xA1 + position as u32);
    }
    if let Some(position) =
        LATIN_UPPER_ACCENTED.iter().chain(LATIN_LOWER_ACCENTED.iter()).position(|candidate| *candidate == name)
    {
        return char::from_u32(0xC0 + position as u32);
    }
    let hex = name
        .strip_prefix("uni")
        .filter(|hex| hex.len() == 4)
        .or_else(|| name.strip_prefix('u').filter(|hex| (4..=6).contains(&hex.len())))?;
    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
}

fn named(name: &str) -> Option<u32> {
    Some(match name {
        "space" => 0x20,
        "nbspace" => 0xA0,
        "exclam" => 0x21,
        "quotedbl" => 0x22,
        "numbersign" => 0x23,
        "dollar" => 0x24,
        "percent" => 0x25,
        "ampersand" => 0x26,
        "quotesingle" => 0x27,
        "parenleft" => 0x28,
        "parenright" => 0x29,
        "asterisk" => 0x2A,
        "plus" => 0x2B,
        "comma" => 0x2C,
        "hyphen" => 0x2D,
        "period" => 0x2E,
        "slash" => 0x2F,
        "zero" => 0x30,
        "one" => 0x31,
        "two" => 0x32,
        "three" => 0x33,
        "four" => 0x34,
        "five" => 0x35,
        "six" => 0x36,
        "seven" => 0x37,
        "eight" => 0x38,
        "nine" => 0x39,
        "colon" => 0x3A,
        "semicolon" => 0x3B,
        "less" => 0x3C,
        "equal" => 0x3D,
        "greater" => 0x3E,
        "question" => 0x3F,
        "at" => 0x40,
        "bracketleft" => 0x5B,
        "backslash" => 0x5C,
        "bracketright" => 0x5D,
        "asciicircum" => 0x5E,
        "underscore" => 0x5F,
        "grave" => 0x60,
        "braceleft" => 0x7B,
        "bar" => 0x7C,
        "braceright" => 0x7D,
        "asciitilde" => 0x7E,
        "softhyphen" | "sfthyphen" => 0xAD,
        "middot" => 0xB7,
        "florin" => 0x192,
        "dotlessi" => 0x131,
        "Lslash" => 0x141,
        "lslash" => 0x142,
        "OE" => 0x152,
        "oe" => 0x153,
        "Scaron" => 0x160,
        "scaron" => 0x161,
        "Ydieresis" => 0x178,
        "Zcaron" => 0x17D,
        "zcaron" => 0x17E,
        "circumflex" => 0x2C6,
        "caron" => 0x2C7,
        "breve" => 0x2D8,
        "dotaccent" => 0x2D9,
        "ring" => 0x2DA,
        "ogonek" => 0x2DB,
        "tilde" => 0x2DC,
        "hungarumlaut" => 0x2DD,
        "pi" => 0x3C0,
        "endash" => 0x2013,
        "emdash" => 0x2014,
        "quoteleft" => 0x2018,
        "quoteright" => 0x2019,
        "quotesinglbase" => 0x201A,
        "quotedblleft" => 0x201C,
        "quotedblright" => 0x201D,
        "quotedblbase" => 0x201E,
        "dagger" => 0x2020,
        "daggerdbl" => 0x2021,
        "bullet" => 0x2022,
        "ellipsis" => 0x2026,
        "perthousand" => 0x2030,
        "guilsinglleft" => 0x2039,
        "guilsinglright" => 0x203A,
        "fraction" => 0x2044,
        "Euro" => 0x20AC,
        "trademark" => 0x2122,
        "Omega" => 0x2126,
        "partialdiff" => 0x2202,
        "Delta" => 0x2206,
        "product" => 0x220F,
        "summation" => 0x2211,
        "minus" => 0x2212,
        "radical" => 0x221A,
        "infinity" => 0x221E,
        "integral" => 0x222B,
        "approxequal" => 0x2248,
        "notequal" => 0x2260,
        "lessequal" => 0x2264,
        "greaterequal" => 0x2265,
        "lozenge" => 0x25CA,
        "apple" => 0xF8FF,
        "ff" => 0xFB00,
        "fi" => 0xFB01,
        "fl" => 0xFB02,
        "ffi" => 0xFB03,
        "ffl" => 0xFB04,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_names_from_every_source() {
        assert_eq!(unicode_of("A"), Some('A'));
        assert_eq!(unicode_of("eacute"), Some('é'));
        assert_eq!(unicode_of("germandbls"), Some('ß'));
        assert_eq!(unicode_of("copyright"), Some('©'));
        assert_eq!(unicode_of("endash"), Some('\u{2013}'));
        assert_eq!(unicode_of("uni20AC"), Some('€'));
        assert_eq!(unicode_of("u1F600"), Some('\u{1F600}'));
        assert_eq!(unicode_of("a.sc"), Some('a'));
        assert_eq!(unicode_of("g123"), None);
        assert_eq!(unicode_of("space"), Some(' '));
    }
}
