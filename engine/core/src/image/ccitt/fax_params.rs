use crate::syntax::{Dict, Object};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaxParams {
    pub k: i64,
    pub columns: usize,
    pub rows: Option<usize>,
    pub byte_aligned: bool,
    pub end_of_line: bool,
    pub black_is_1: bool,
}

impl FaxParams {
    pub fn read(params: &Dict, width: usize, height: usize) -> FaxParams {
        let flag = |key: &str| matches!(params.get(key), Some(Object::Bool(true)));
        let size = |key: &str| params.int(key).filter(|value| *value > 0).map(|value| value as usize);
        FaxParams {
            k: params.int("K").unwrap_or(0),
            columns: size("Columns").unwrap_or(if width > 0 { width } else { 1728 }),
            rows: size("Rows").or((height > 0).then_some(height)),
            byte_aligned: flag("EncodedByteAlign"),
            end_of_line: flag("EndOfLine"),
            black_is_1: flag("BlackIs1"),
        }
    }
}
