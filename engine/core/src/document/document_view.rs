use std::rc::Rc;

use crate::document::{DocumentCaches, PageSummary};
use crate::error::{Error, Result};
use crate::fonts::FontLibrary;
use crate::pages::{Page, PageId, PageTree};
use crate::render::{PageRenderer, Raster, RenderContext, Tile};
use crate::store::ObjectStore;
use crate::syntax::Name;
use crate::text::{TextExtractor, TextGlyph, TextLayout};

#[derive(Clone, Copy)]
pub struct DocumentView<'a> {
    pub store: &'a ObjectStore,
    pub caches: &'a DocumentCaches,
    pub library: &'a FontLibrary,
}

impl<'a> DocumentView<'a> {
    pub fn pages(&self) -> Result<Rc<Vec<Page>>> {
        let revision = self.store.revision();
        if let Some((cached, pages)) = &*self.caches.pages.borrow()
            && *cached == revision
        {
            return Ok(pages.clone());
        }
        let pages = Rc::new(PageTree::new(self.store).pages()?);
        *self.caches.pages.borrow_mut() = Some((revision, pages.clone()));
        Ok(pages)
    }

    pub fn page(&self, id: PageId) -> Result<Page> {
        self.pages()?.iter().find(|page| page.id == id).cloned().ok_or_else(|| Error::new(format!("no page {id}")))
    }

    pub fn summaries(&self) -> Result<Vec<PageSummary>> {
        Ok(self
            .pages()?
            .iter()
            .map(|page| {
                let (width, height) = page.display_size();
                PageSummary { id: page.id, width, height, rotate: page.rotate }
            })
            .collect())
    }

    pub fn render(&self, id: PageId, scale: f64, tile: Option<Tile>) -> Result<Raster> {
        let page = self.page(id)?;
        let context = RenderContext {
            store: self.store,
            fonts: &self.caches.fonts,
            images: &self.caches.images,
            library: self.library,
        };
        PageRenderer::new(context).render(&page, scale, tile)
    }

    pub fn text_glyphs(&self, id: PageId) -> Result<Vec<TextGlyph>> {
        TextExtractor::new(self.store, &self.caches.fonts, self.library).page(&self.page(id)?)
    }

    pub fn fonts(&self, id: PageId) -> Result<Vec<(Name, String)>> {
        let page = self.page(id)?;
        let Some(fonts) = page.resources.get("Font") else { return Ok(Vec::new()) };
        let fonts = self.store.resolve(fonts)?.as_dict().cloned().unwrap_or_default();
        Ok(fonts
            .iter()
            .filter_map(|(name, entry)| {
                let font = self.caches.fonts.font(entry, self.store, self.library).ok()?;
                Some((name.clone(), font.family_name().to_string()))
            })
            .collect())
    }

    pub fn text_layout(&self, id: PageId) -> Result<Rc<TextLayout>> {
        let key = (id, self.store.revision());
        if let Some(layout) = self.caches.layouts.borrow().get(&key) {
            return Ok(layout.clone());
        }
        let layout = Rc::new(TextLayout::new(&self.text_glyphs(id)?));
        self.caches.layouts.borrow_mut().insert(key, layout.clone());
        Ok(layout)
    }
}
