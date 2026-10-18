pub(super) const LATIN_UPPER_ACCENTED: [&str; 32] = [
    "Agrave",
    "Aacute",
    "Acircumflex",
    "Atilde",
    "Adieresis",
    "Aring",
    "AE",
    "Ccedilla",
    "Egrave",
    "Eacute",
    "Ecircumflex",
    "Edieresis",
    "Igrave",
    "Iacute",
    "Icircumflex",
    "Idieresis",
    "Eth",
    "Ntilde",
    "Ograve",
    "Oacute",
    "Ocircumflex",
    "Otilde",
    "Odieresis",
    "multiply",
    "Oslash",
    "Ugrave",
    "Uacute",
    "Ucircumflex",
    "Udieresis",
    "Yacute",
    "Thorn",
    "germandbls",
];

pub(super) const LATIN_LOWER_ACCENTED: [&str; 32] = [
    "agrave",
    "aacute",
    "acircumflex",
    "atilde",
    "adieresis",
    "aring",
    "ae",
    "ccedilla",
    "egrave",
    "eacute",
    "ecircumflex",
    "edieresis",
    "igrave",
    "iacute",
    "icircumflex",
    "idieresis",
    "eth",
    "ntilde",
    "ograve",
    "oacute",
    "ocircumflex",
    "otilde",
    "odieresis",
    "divide",
    "oslash",
    "ugrave",
    "uacute",
    "ucircumflex",
    "udieresis",
    "yacute",
    "thorn",
    "ydieresis",
];

pub(super) const LATIN_SYMBOLS: [&str; 32] = [
    "space",
    "exclamdown",
    "cent",
    "sterling",
    "currency",
    "yen",
    "brokenbar",
    "section",
    "dieresis",
    "copyright",
    "ordfeminine",
    "guillemotleft",
    "logicalnot",
    "hyphen",
    "registered",
    "macron",
    "degree",
    "plusminus",
    "twosuperior",
    "threesuperior",
    "acute",
    "mu",
    "paragraph",
    "periodcentered",
    "cedilla",
    "onesuperior",
    "ordmasculine",
    "guillemotright",
    "onequarter",
    "onehalf",
    "threequarters",
    "questiondown",
];

const ASCII_PUNCTUATION: [&str; 16] = [
    "space",
    "exclam",
    "quotedbl",
    "numbersign",
    "dollar",
    "percent",
    "ampersand",
    "quotesingle",
    "parenleft",
    "parenright",
    "asterisk",
    "plus",
    "comma",
    "hyphen",
    "period",
    "slash",
];

const DIGITS: [&str; 10] = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"];

const ASCII_BETWEEN: [(u8, &str); 13] = [
    (58, "colon"),
    (59, "semicolon"),
    (60, "less"),
    (61, "equal"),
    (62, "greater"),
    (63, "question"),
    (64, "at"),
    (91, "bracketleft"),
    (92, "backslash"),
    (93, "bracketright"),
    (94, "asciicircum"),
    (95, "underscore"),
    (96, "grave"),
];

const ASCII_TAIL: [(u8, &str); 4] = [(123, "braceleft"), (124, "bar"), (125, "braceright"), (126, "asciitilde")];

const WIN_ANSI_HIGH: [(u8, &str); 27] = [
    (127, "bullet"),
    (128, "Euro"),
    (130, "quotesinglbase"),
    (131, "florin"),
    (132, "quotedblbase"),
    (133, "ellipsis"),
    (134, "dagger"),
    (135, "daggerdbl"),
    (136, "circumflex"),
    (137, "perthousand"),
    (138, "Scaron"),
    (139, "guilsinglleft"),
    (140, "OE"),
    (142, "Zcaron"),
    (145, "quoteleft"),
    (146, "quoteright"),
    (147, "quotedblleft"),
    (148, "quotedblright"),
    (149, "bullet"),
    (150, "endash"),
    (151, "emdash"),
    (152, "tilde"),
    (153, "trademark"),
    (154, "scaron"),
    (155, "guilsinglright"),
    (156, "oe"),
    (158, "zcaron"),
];

const STANDARD_HIGH: [(u8, &str); 56] = [
    (161, "exclamdown"),
    (162, "cent"),
    (163, "sterling"),
    (164, "fraction"),
    (165, "yen"),
    (166, "florin"),
    (167, "section"),
    (168, "currency"),
    (169, "quotesingle"),
    (170, "quotedblleft"),
    (171, "guillemotleft"),
    (172, "guilsinglleft"),
    (173, "guilsinglright"),
    (174, "fi"),
    (175, "fl"),
    (177, "endash"),
    (178, "dagger"),
    (179, "daggerdbl"),
    (180, "periodcentered"),
    (182, "paragraph"),
    (183, "bullet"),
    (184, "quotesinglbase"),
    (185, "quotedblbase"),
    (186, "quotedblright"),
    (187, "guillemotright"),
    (188, "ellipsis"),
    (189, "perthousand"),
    (191, "questiondown"),
    (193, "grave"),
    (194, "acute"),
    (195, "circumflex"),
    (196, "tilde"),
    (197, "macron"),
    (198, "breve"),
    (199, "dotaccent"),
    (200, "dieresis"),
    (202, "ring"),
    (203, "cedilla"),
    (205, "hungarumlaut"),
    (206, "ogonek"),
    (207, "caron"),
    (208, "emdash"),
    (225, "AE"),
    (227, "ordfeminine"),
    (232, "Lslash"),
    (233, "Oslash"),
    (234, "OE"),
    (235, "ordmasculine"),
    (241, "ae"),
    (245, "dotlessi"),
    (248, "lslash"),
    (249, "oslash"),
    (250, "oe"),
    (251, "germandbls"),
    (39, "quoteright"),
    (96, "quoteleft"),
];

const MAC_ROMAN_HIGH: [&str; 128] = [
    "Adieresis",
    "Aring",
    "Ccedilla",
    "Eacute",
    "Ntilde",
    "Odieresis",
    "Udieresis",
    "aacute",
    "agrave",
    "acircumflex",
    "adieresis",
    "atilde",
    "aring",
    "ccedilla",
    "eacute",
    "egrave",
    "ecircumflex",
    "edieresis",
    "iacute",
    "igrave",
    "icircumflex",
    "idieresis",
    "ntilde",
    "oacute",
    "ograve",
    "ocircumflex",
    "odieresis",
    "otilde",
    "uacute",
    "ugrave",
    "ucircumflex",
    "udieresis",
    "dagger",
    "degree",
    "cent",
    "sterling",
    "section",
    "bullet",
    "paragraph",
    "germandbls",
    "registered",
    "copyright",
    "trademark",
    "acute",
    "dieresis",
    "notequal",
    "AE",
    "Oslash",
    "infinity",
    "plusminus",
    "lessequal",
    "greaterequal",
    "yen",
    "mu",
    "partialdiff",
    "summation",
    "product",
    "pi",
    "integral",
    "ordfeminine",
    "ordmasculine",
    "Omega",
    "ae",
    "oslash",
    "questiondown",
    "exclamdown",
    "logicalnot",
    "radical",
    "florin",
    "approxequal",
    "Delta",
    "guillemotleft",
    "guillemotright",
    "ellipsis",
    "space",
    "Agrave",
    "Atilde",
    "Otilde",
    "OE",
    "oe",
    "endash",
    "emdash",
    "quotedblleft",
    "quotedblright",
    "quoteleft",
    "quoteright",
    "divide",
    "lozenge",
    "ydieresis",
    "Ydieresis",
    "fraction",
    "currency",
    "guilsinglleft",
    "guilsinglright",
    "fi",
    "fl",
    "daggerdbl",
    "periodcentered",
    "quotesinglbase",
    "quotedblbase",
    "perthousand",
    "Acircumflex",
    "Ecircumflex",
    "Aacute",
    "Edieresis",
    "Egrave",
    "Iacute",
    "Icircumflex",
    "Idieresis",
    "Igrave",
    "Oacute",
    "Ocircumflex",
    "apple",
    "Ograve",
    "Uacute",
    "Ucircumflex",
    "Ugrave",
    "dotlessi",
    "circumflex",
    "tilde",
    "macron",
    "breve",
    "dotaccent",
    "ring",
    "cedilla",
    "hungarumlaut",
    "ogonek",
    "caron",
];

pub type EncodingTable = [Option<&'static str>; 256];

fn ascii() -> EncodingTable {
    let mut table: EncodingTable = [None; 256];
    for (offset, name) in ASCII_PUNCTUATION.iter().enumerate() {
        table[32 + offset] = Some(name);
    }
    for (offset, name) in DIGITS.iter().enumerate() {
        table[48 + offset] = Some(name);
    }
    for &(code, name) in ASCII_BETWEEN.iter().chain(ASCII_TAIL.iter()) {
        table[code as usize] = Some(name);
    }
    for letter in 0..26u8 {
        table[(b'A' + letter) as usize] = Some(LETTERS_UPPER[letter as usize]);
        table[(b'a' + letter) as usize] = Some(LETTERS_LOWER[letter as usize]);
    }
    table
}

const LETTERS_UPPER: [&str; 26] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W",
    "X", "Y", "Z",
];

const LETTERS_LOWER: [&str; 26] = [
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w",
    "x", "y", "z",
];

pub fn win_ansi() -> EncodingTable {
    let mut table = ascii();
    for &(code, name) in &WIN_ANSI_HIGH {
        table[code as usize] = Some(name);
    }
    table[159] = Some("Ydieresis");
    for (offset, name) in LATIN_SYMBOLS.iter().enumerate() {
        table[160 + offset] = Some(name);
    }
    for (offset, name) in LATIN_UPPER_ACCENTED.iter().chain(LATIN_LOWER_ACCENTED.iter()).enumerate() {
        table[192 + offset] = Some(name);
    }
    table
}

pub fn standard() -> EncodingTable {
    let mut table = ascii();
    for &(code, name) in &STANDARD_HIGH {
        table[code as usize] = Some(name);
    }
    table
}

pub fn mac_roman() -> EncodingTable {
    let mut table = ascii();
    for (offset, name) in MAC_ROMAN_HIGH.iter().enumerate() {
        table[128 + offset] = Some(name);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_the_known_landmarks() {
        let win = win_ansi();
        assert_eq!(
            (win[65], win[128], win[150], win[233], win[255]),
            (Some("A"), Some("Euro"), Some("endash"), Some("eacute"), Some("ydieresis"))
        );
        let standard = standard();
        assert_eq!(
            (standard[39], standard[96], standard[174], standard[251]),
            (Some("quoteright"), Some("quoteleft"), Some("fi"), Some("germandbls"))
        );
        assert_eq!(standard[128], None);
        let mac = mac_roman();
        assert_eq!((mac[128], mac[202], mac[255]), (Some("Adieresis"), Some("space"), Some("caron")));
        assert_eq!(win[39], Some("quotesingle"));
    }
}
