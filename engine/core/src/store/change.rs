use crate::store::Slot;

#[derive(Clone, Debug, PartialEq)]
pub struct SlotChange {
    pub num: u32,
    pub before: Option<Slot>,
    pub after: Option<Slot>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub label: String,
    pub slots: Vec<SlotChange>,
    pub next_num_before: u32,
    pub next_num_after: u32,
    pub forces_rewrite: bool,
    pub ui_before: String,
    pub ui_after: String,
}

impl Change {
    pub fn touched(&self) -> Vec<u32> {
        self.slots.iter().map(|slot| slot.num).collect()
    }
}
