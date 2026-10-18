use crate::content::{ContentParser, ContentWriter};
use crate::syntax::{Name, Object};

const FALLBACK_FONT: &str = "Helv";

#[derive(Clone, Debug, PartialEq)]
pub struct DefaultAppearance {
    pub font: Name,
    pub size: f64,
    color: Option<(String, Vec<Object>)>,
}

impl DefaultAppearance {
    pub fn parse(source: Option<&[u8]>) -> DefaultAppearance {
        let mut appearance = DefaultAppearance { font: Name::from(FALLBACK_FONT), size: 0.0, color: None };
        for operation in ContentParser::new(source.unwrap_or_default(), 0).parse(0) {
            match operation.operator.as_str() {
                "Tf" => {
                    if let Some(font) = operation.operands.first().and_then(Object::as_name) {
                        appearance.font = font.clone();
                    }
                    appearance.size = operation.operands.get(1).and_then(Object::as_number).unwrap_or(0.0);
                }
                "g" | "rg" | "k" => appearance.color = Some((operation.operator.clone(), operation.operands.clone())),
                _ => {}
            }
        }
        appearance
    }

    pub fn write_color(&self, writer: &mut ContentWriter) {
        match &self.color {
            Some((operator, operands)) => writer.operation(operator, operands),
            None => writer.operation("g", &[Object::Int(0)]),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_font_size_and_color_from_the_operators() {
        let appearance = DefaultAppearance::parse(Some(b"/Helv 0 Tf 0.2 0.4 0.6 rg"));
        assert_eq!((&appearance.font, appearance.size), (&Name::from("Helv"), 0.0));
        let mut writer = ContentWriter::new();
        appearance.write_color(&mut writer);
        assert_eq!(writer.finish(), b"0.2 0.4 0.6 rg\n");
    }

    #[test]
    fn falls_back_to_auto_sized_helvetica_in_black() {
        let appearance = DefaultAppearance::parse(None);
        assert_eq!((appearance.font, appearance.size), (Name::from("Helv"), 0.0));
    }
}
