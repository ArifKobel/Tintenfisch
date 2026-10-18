use crate::store::Change;

#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Change>,
    redo: Vec<Change>,
}

impl History {
    pub fn record(&mut self, change: Change) {
        self.undo.push(change);
        self.redo.clear();
    }

    pub fn take_undo(&mut self) -> Option<Change> {
        self.undo.pop()
    }

    pub fn take_redo(&mut self) -> Option<Change> {
        self.redo.pop()
    }

    pub fn push_undone(&mut self, change: Change) {
        self.redo.push(change);
    }

    pub fn push_redone(&mut self, change: Change) {
        self.undo.push(change);
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn forces_rewrite(&self) -> bool {
        self.undo.iter().any(|change| change.forces_rewrite)
    }
}
