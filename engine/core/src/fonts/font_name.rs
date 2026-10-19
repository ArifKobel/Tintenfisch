const PLACEHOLDERS: [&str; 5] = ["false", "true", "null", "undefined", "none"];
const PLAIN_STYLES: [&str; 4] = ["Regular", "Normal", "Book", "Roman"];

pub fn meaningful(name: &str) -> Option<&str> {
    let name = name.trim();
    (!name.is_empty() && !PLACEHOLDERS.iter().any(|placeholder| name.eq_ignore_ascii_case(placeholder))).then_some(name)
}

pub fn with_style(family: &str, style: Option<&str>) -> String {
    match style.filter(|style| !PLAIN_STYLES.contains(style)) {
        Some(style) => format!("{family} {style}"),
        None => family.to_string(),
    }
}

pub fn from_postscript(name: &str) -> Option<String> {
    let name = meaningful(name)?;
    if !name.chars().all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_') {
        return None;
    }
    let (family, style) = match name.split_once('-') {
        Some((family, style)) => (family, Some(spaced(style))),
        None => (name, None),
    };
    Some(with_style(&spaced(family), style.as_deref()))
}

fn spaced(word: &str) -> String {
    let mut spaced = String::with_capacity(word.len() + 4);
    let mut previous: Option<char> = None;
    for character in word.chars() {
        if character == '_' {
            spaced.push(' ');
        } else {
            if character.is_ascii_uppercase() && previous.is_some_and(|previous| previous.is_ascii_lowercase()) {
                spaced.push(' ');
            }
            spaced.push(character);
        }
        previous = Some(character);
    }
    spaced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_no_names() {
        assert_eq!(meaningful(" false "), None);
        assert_eq!(meaningful("Undefined"), None);
        assert_eq!(meaningful("Inter"), Some("Inter"));
    }

    #[test]
    fn postscript_names_read_as_family_and_style() {
        assert_eq!(from_postscript("CabinetGrotesk-Bold").as_deref(), Some("Cabinet Grotesk Bold"));
        assert_eq!(from_postscript("CabinetGrotesk-Regular").as_deref(), Some("Cabinet Grotesk"));
        assert_eq!(from_postscript("ArialMT").as_deref(), Some("Arial MT"));
        assert_eq!(from_postscript("Source_Sans-SemiBoldItalic").as_deref(), Some("Source Sans Semi Bold Italic"));
        assert_eq!(from_postscript("false"), None);
        assert_eq!(from_postscript("Not a postscript name"), None);
    }
}
