use crate::error::Result;
use crate::save::full_rewriter::FullRewriter;
use crate::save::incremental_writer::IncrementalWriter;
use crate::store::ObjectStore;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveMode {
    Incremental,
    FullRewrite,
}

impl SaveMode {
    pub fn required_for(store: &ObjectStore, optimize: bool) -> SaveMode {
        if optimize || store.forces_rewrite() || store.source().is_repaired() {
            SaveMode::FullRewrite
        } else {
            SaveMode::Incremental
        }
    }

    pub fn write(self, store: &ObjectStore) -> Result<Vec<u8>> {
        match self {
            SaveMode::Incremental => IncrementalWriter::new(store).write(),
            SaveMode::FullRewrite => FullRewriter::new(store).write(),
        }
    }
}
