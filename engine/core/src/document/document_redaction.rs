use crate::document::Document;
use crate::error::{Error, Result};
use crate::fonts::FontLibrary;
use crate::pages::{PageId, PageTree};
use crate::redact::{Redactor, neutralize_outline_titles};

impl Document {
    pub fn apply_redactions(&mut self, library: &FontLibrary) -> Result<Vec<PageId>> {
        let redactor = Redactor::new(&self.store, &self.caches.fonts, library);
        let mut planned = Vec::new();
        for page in PageTree::new(&self.store).pages()? {
            if let Some(redaction) = redactor.plan(&page)? {
                planned.push((page.id, redaction));
            }
        }
        if planned.is_empty() {
            return Err(Error::unsupported("nothing is marked for redaction"));
        }
        let removed: Vec<String> =
            planned.iter().flat_map(|(_, redaction)| redaction.removed_text().iter().cloned()).collect();
        self.store.transact("Apply redactions", "", |transaction| {
            transaction.force_rewrite();
            for (_, redaction) in &planned {
                redaction.apply(transaction)?;
            }
            neutralize_outline_titles(transaction, &removed)
        })?;
        Ok(planned.into_iter().map(|(page, _)| page).collect())
    }
}
