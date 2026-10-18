use std::ops::Range;

use crate::syntax::Object;

#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    pub operator: String,
    pub operands: Vec<Object>,
    pub segment: usize,
    pub span: Range<usize>,
    pub ordinal: usize,
}

impl Operation {
    pub fn is(&self, operator: &str) -> bool {
        self.operator == operator
    }

    pub fn number(&self, index: usize) -> Option<f64> {
        self.operands.get(index).and_then(Object::as_number)
    }
}
