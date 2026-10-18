use crate::fonts::encoding_tables::{EncodingTable, mac_roman, standard, win_ansi};
use crate::syntax::{Name, Object};

#[derive(Clone, Debug, PartialEq)]
pub struct Encoding {
    names: Vec<Option<String>>,
}

impl Encoding {
    fn from_table(table: EncodingTable) -> Self {
        Self { names: table.iter().map(|name| name.map(str::to_string)).collect() }
    }

    pub fn standard() -> Self {
        Self::from_table(standard())
    }

    pub fn win_ansi() -> Self {
        Self::from_table(win_ansi())
    }

    pub fn empty() -> Self {
        Self { names: vec![None; 256] }
    }

    fn named(name: &Name) -> Option<Self> {
        match name.as_bytes() {
            b"StandardEncoding" => Some(Self::standard()),
            b"WinAnsiEncoding" => Some(Self::win_ansi()),
            b"MacRomanEncoding" => Some(Self::from_table(mac_roman())),
            _ => None,
        }
    }

    pub fn read(object: Option<&Object>, default: Encoding) -> Self {
        match object {
            Some(Object::Name(name)) => Self::named(name).unwrap_or(default),
            Some(Object::Dict(dict)) => {
                let mut encoding = dict.name("BaseEncoding").and_then(Self::named).unwrap_or(default);
                if let Some(differences) = dict.get("Differences").and_then(Object::as_array) {
                    encoding.apply_differences(differences);
                }
                encoding
            }
            _ => default,
        }
    }

    fn apply_differences(&mut self, differences: &[Object]) {
        let mut code = 0usize;
        for item in differences {
            match item {
                Object::Int(start) => code = (*start).clamp(0, 255) as usize,
                Object::Name(name) => {
                    if let Some(slot) = self.names.get_mut(code) {
                        *slot = Some(String::from_utf8_lossy(name.as_bytes()).into_owned());
                    }
                    code += 1;
                }
                _ => {}
            }
        }
    }

    pub fn name(&self, code: u8) -> Option<&str> {
        self.names[code as usize].as_deref()
    }

    pub fn has_names(&self) -> bool {
        self.names.iter().any(Option::is_some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Parser;

    #[test]
    fn applies_differences_over_the_base_encoding() {
        let object = Parser::new(b"<< /BaseEncoding /WinAnsiEncoding /Differences [65 /alpha /beta 200 /gamma] >>", 0)
            .parse_object()
            .unwrap();
        let encoding = Encoding::read(Some(&object), Encoding::standard());
        assert_eq!((encoding.name(65), encoding.name(66), encoding.name(67)), (Some("alpha"), Some("beta"), Some("C")));
        assert_eq!((encoding.name(200), encoding.name(128)), (Some("gamma"), Some("Euro")));
    }

    #[test]
    fn falls_back_to_the_default_for_unknown_names() {
        let encoding = Encoding::read(Some(&Object::Name(Name::from("Bogus"))), Encoding::standard());
        assert_eq!(encoding.name(39), Some("quoteright"));
    }
}
