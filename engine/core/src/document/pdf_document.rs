use crate::document::{DocumentCaches, DocumentView};
use crate::edit::{EditPlan, NewParagraph, ParagraphEdit, ParagraphPlanner, ParagraphText};
use crate::error::Result;
use crate::file::SourceFile;
use crate::fonts::FontLibrary;
use crate::pages::{PageId, PageTree, drawn_forms};
use crate::save::{Encryption, FullRewriter, SaveMode};
use crate::store::{ObjectStore, Reverted};

pub struct Document {
    pub(super) store: ObjectStore,
    pub(super) caches: DocumentCaches,
    pub(super) encryption: Option<Encryption>,
    pub(super) owner_rights: bool,
}

impl Document {
    pub fn open(bytes: Vec<u8>, password: &str) -> Result<Document> {
        let mut store = ObjectStore::new(SourceFile::open_with_password(bytes, password)?);
        PageTree::separate_shared_pages(&mut store)?;
        Ok(Document { store, caches: DocumentCaches::default(), encryption: None, owner_rights: false })
    }

    pub fn store(&self) -> &ObjectStore {
        &self.store
    }

    pub fn view<'a>(&'a self, library: &'a FontLibrary) -> DocumentView<'a> {
        DocumentView { store: &self.store, caches: &self.caches, library }
    }

    pub fn save(&self, optimize: bool) -> Result<Vec<u8>> {
        match &self.encryption {
            Some(encryption) => FullRewriter::new(&self.store).encrypted(encryption).write(),
            None => SaveMode::required_for(&self.store, optimize).write(&self.store),
        }
    }

    pub fn paragraph_text(&self, page: PageId, line: usize, library: &FontLibrary) -> Result<ParagraphText> {
        ParagraphPlanner::new(&self.store, &self.caches.fonts, library).text(page, line)
    }

    pub fn plan_edit(&self, edit: &ParagraphEdit, library: &FontLibrary) -> Result<EditPlan> {
        ParagraphPlanner::new(&self.store, &self.caches.fonts, library).plan(edit)
    }

    pub fn plan_new_text(&self, text: &NewParagraph, library: &FontLibrary) -> Result<EditPlan> {
        text.plan(&self.store, library)
    }

    pub fn apply(&mut self, plan: &EditPlan, ui_state: &str) -> Result<()> {
        if plan.is_empty() {
            return Ok(());
        }
        self.store.transact("Edit text", ui_state, |transaction| plan.apply(transaction))
    }

    pub fn edit(&mut self, edit: &ParagraphEdit, library: &FontLibrary, ui_state: &str) -> Result<()> {
        let plan = self.plan_edit(edit, library)?;
        self.apply(&plan, ui_state)
    }

    pub fn preview<T>(
        &mut self,
        plan: &EditPlan,
        library: &FontLibrary,
        inspect: impl FnOnce(DocumentView) -> T,
    ) -> Result<T> {
        let caches = &self.caches;
        self.store
            .trial(|transaction| plan.apply(transaction), |store| inspect(DocumentView { store, caches, library }))
    }

    pub fn preview_edit<T>(
        &mut self,
        edit: &ParagraphEdit,
        library: &FontLibrary,
        inspect: impl FnOnce(DocumentView) -> T,
    ) -> Result<T> {
        let plan = self.plan_edit(edit, library)?;
        self.preview(&plan, library, inspect)
    }

    pub fn pages_touching(&self, numbers: &[u32], library: &FontLibrary) -> Result<Vec<PageId>> {
        Ok(self
            .view(library)
            .pages()?
            .iter()
            .filter(|page| {
                numbers.contains(&page.id.0.num)
                    || page.contents.iter().any(|stream| numbers.contains(&stream.num))
                    || drawn_forms(&self.store, &page.resources).iter().any(|form| numbers.contains(&form.num))
            })
            .map(|page| page.id)
            .collect())
    }

    pub fn undo(&mut self) -> Option<Reverted> {
        self.store.undo()
    }

    pub fn redo(&mut self) -> Option<Reverted> {
        self.store.redo()
    }
}
