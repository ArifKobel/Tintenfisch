use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::fonts::FontCache;
use crate::pages::{Page, PageId};
use crate::render::ImageCache;
use crate::text::TextLayout;

#[derive(Default)]
pub struct DocumentCaches {
    pub fonts: FontCache,
    pub images: ImageCache,
    pub pages: RefCell<Option<(u64, Rc<Vec<Page>>)>>,
    pub layouts: RefCell<HashMap<(PageId, u64), Rc<TextLayout>>>,
}
