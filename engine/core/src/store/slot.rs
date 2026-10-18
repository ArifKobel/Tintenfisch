use std::rc::Rc;

use crate::syntax::Object;

#[derive(Clone, Debug, PartialEq)]
pub enum Slot {
    Object { generation: u16, object: Rc<Object> },
    Freed { next_generation: u16 },
}
