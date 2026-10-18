use crate::geometry::Rect;

#[derive(Clone, Debug, PartialEq)]
pub struct NewField {
    pub kind: NewFieldKind,
    pub name: String,
    pub rect: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NewFieldKind {
    Text { multiline: bool },
    Checkbox,
    Radio,
    Combo { options: Vec<String> },
    List { options: Vec<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldProperties {
    pub name: String,
    pub required: bool,
    pub read_only: bool,
    pub multiline: bool,
    pub options: Vec<String>,
}
