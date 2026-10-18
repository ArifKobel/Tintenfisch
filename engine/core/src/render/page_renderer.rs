use crate::annotations::{Annotation, appearance_placement};
use crate::content::ContentStream;
use crate::error::{Error, Result};
use crate::forms::MissingAppearances;
use crate::geometry::{Matrix, Rect};
use crate::pages::Page;
use crate::render::canvas::Canvas;
use crate::render::interpreter::Interpreter;
use crate::render::{Raster, RenderContext, Tile, Viewport};

const TILE_MARGIN: u32 = 8;

pub struct PageRenderer<'a> {
    context: RenderContext<'a>,
}

impl<'a> PageRenderer<'a> {
    pub fn new(context: RenderContext<'a>) -> Self {
        Self { context }
    }

    pub fn render(&self, page: &Page, scale: f64, tile: Option<Tile>) -> Result<Raster> {
        let viewport = Viewport::new(page, scale);
        let tile = viewport
            .clip(tile.unwrap_or_else(|| viewport.whole()))
            .ok_or_else(|| Error::new("tile lies outside the page"))?;
        let painted = viewport.expand(tile, TILE_MARGIN);
        let mut canvas =
            Canvas::new(painted.width, painted.height).ok_or_else(|| Error::new("tile is too large to allocate"))?;
        let content = ContentStream::load(self.context.store, page)?;
        let device = viewport.device_matrix(painted);
        Interpreter::new(self.context, &mut canvas, device).run(&content.operations, &page.resources);
        self.draw_annotations(page, &mut canvas, device)?;
        Ok(Raster::from_pixmap(&canvas.pixmap).crop(tile.x - painted.x, tile.y - painted.y, tile.width, tile.height))
    }

    fn draw_annotations(&self, page: &Page, canvas: &mut Canvas, device: Matrix) -> Result<()> {
        let store = self.context.store;
        let missing = MissingAppearances::new(store, self.context.fonts, self.context.library);
        for annotation in Annotation::on_page(store, page)?.iter().filter(|annotation| annotation.is_shown()) {
            let Some(id) = annotation.normal_appearance(store)? else {
                let generated = match annotation.id.filter(|_| annotation.subtype.is("Widget")) {
                    Some(widget) => missing.appearance(widget)?,
                    None => None,
                };
                if let Some(generated) = generated {
                    let bounds = Rect::new(0.0, 0.0, generated.width, generated.height);
                    let placement = appearance_placement(bounds, Matrix::IDENTITY, annotation.rect);
                    Interpreter::new(self.context, canvas, device).draw_generated(
                        &generated.content,
                        &generated.resources,
                        bounds,
                        placement,
                    );
                }
                continue;
            };
            let appearance = store.object(id)?;
            let Some(form) = appearance.as_dict() else { continue };
            let bbox = form.get("BBox").and_then(Rect::from_object).unwrap_or(annotation.rect);
            let matrix = form.get("Matrix").and_then(Matrix::from_object).unwrap_or(Matrix::IDENTITY);
            let placement = appearance_placement(bbox, matrix, annotation.rect);
            Interpreter::new(self.context, canvas, device).draw_appearance(id, placement, &page.resources);
        }
        Ok(())
    }
}
