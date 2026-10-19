use crate::fonts::FallbackFace;
use crate::syntax::Name;

#[derive(Clone, Debug, PartialEq)]
pub struct PageFont {
    pub name: Name,
    pub family: Option<String>,
    pub base_font: String,
    pub embedded: bool,
    pub face: FallbackFace,
    pub weight: Option<u16>,
}
