use crate::pages::PageId;

#[derive(Clone, Debug, PartialEq)]
pub struct PageSummary {
    pub id: PageId,
    pub width: f64,
    pub height: f64,
    pub rotate: u16,
}
