use crate::syntax::ObjId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlyphOrigin {
    pub form: Option<ObjId>,
    pub ordinal: usize,
    pub index: usize,
}
