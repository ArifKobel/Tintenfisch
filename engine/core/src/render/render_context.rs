use crate::fonts::{FontCache, FontLibrary};
use crate::render::ImageCache;
use crate::store::ObjectStore;

#[derive(Clone, Copy)]
pub struct RenderContext<'a> {
    pub store: &'a ObjectStore,
    pub fonts: &'a FontCache,
    pub images: &'a ImageCache,
    pub library: &'a FontLibrary,
}
