use std::ops::Range;

use crate::error::{Error, Result};

#[derive(Clone, Debug, Default)]
pub struct Splice {
    edits: Vec<(Range<usize>, Vec<u8>)>,
}

impl Splice {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replace(&mut self, span: Range<usize>, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.edits.push((span, bytes.into()));
        self
    }

    pub fn insert(&mut self, at: usize, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.replace(at..at, bytes)
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    pub fn apply(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut edits: Vec<&(Range<usize>, Vec<u8>)> = self.edits.iter().collect();
        edits.sort_by_key(|(span, _)| (span.start, span.end));
        let mut out = Vec::with_capacity(data.len());
        let mut cursor = 0;
        for (span, bytes) in edits {
            if span.start < cursor || span.end > data.len() || span.start > span.end {
                return Err(Error::new(format!("splice span {span:?} overlaps or leaves the data")));
            }
            out.extend_from_slice(&data[cursor..span.start]);
            out.extend_from_slice(bytes);
            cursor = span.end;
        }
        out.extend_from_slice(&data[cursor..]);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_and_inserts_in_any_order() {
        let mut splice = Splice::new();
        splice.replace(6..11, "there").insert(0, ">> ").insert(11, "!");
        assert_eq!(splice.apply(b"hello world").unwrap(), b">> hello there!");
    }

    #[test]
    fn leaves_data_untouched_without_edits() {
        assert_eq!(Splice::new().apply(b"q Q").unwrap(), b"q Q");
    }

    #[test]
    fn refuses_overlapping_or_outside_spans() {
        let mut splice = Splice::new();
        splice.replace(0..4, "a").replace(2..6, "b");
        assert!(splice.apply(b"0123456789").is_err());
        assert!(Splice::new().replace(5..20, "x").apply(b"short").is_err());
    }
}
