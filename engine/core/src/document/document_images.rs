use crate::document::Document;
use crate::error::Result;
use crate::page_images::{ImageEdit, ImageFile, ImageFrame, PlacedImage};
use crate::pages::PageId;

const FIT_SHARE: f64 = 0.8;

impl Document {
    pub fn images(&self, page: PageId) -> Result<Vec<ImageFrame>> {
        let page = self.page(page)?;
        Ok(PlacedImage::on_page(&self.store, &page)?.into_iter().map(|placed| placed.frame).collect())
    }

    pub fn insert_image(&mut self, page: PageId, data: &[u8], center: (f64, f64)) -> Result<usize> {
        let page = self.page(page)?;
        let file = ImageFile::read(data)?;
        let shown = page.display_size();
        let (width, height) = file.natural_size;
        let scale = (shown.0 * FIT_SHARE / width).min(shown.1 * FIT_SHARE / height).min(1.0);
        let (width, height) = (width * scale, height * scale);
        let frame = ImageFrame { x: center.0 - width / 2.0, y: center.1 - height / 2.0, width, height, angle: 0.0 };
        let edit = ImageEdit::inserted(&self.store, &page, file, &frame)?;
        self.store.transact("Add image", "", |transaction| edit.apply(transaction))?;
        Ok(PlacedImage::on_page(&self.store, &self.page(page.id)?)?.len().saturating_sub(1))
    }

    pub fn move_image(&mut self, page: PageId, index: usize, frame: &ImageFrame) -> Result<()> {
        let edit = ImageEdit::moved(&self.store, &self.page(page)?, index, frame)?;
        self.store.transact("Move image", "", |transaction| edit.apply(transaction))
    }

    pub fn replace_image(&mut self, page: PageId, index: usize, data: &[u8]) -> Result<()> {
        let edit = ImageEdit::replaced(&self.store, &self.page(page)?, index, ImageFile::read(data)?)?;
        self.store.transact("Replace image", "", |transaction| edit.apply(transaction))
    }

    pub fn delete_image(&mut self, page: PageId, index: usize) -> Result<()> {
        let edit = ImageEdit::deleted(&self.store, &self.page(page)?, index)?;
        self.store.transact("Delete image", "", |transaction| edit.apply(transaction))
    }
}
