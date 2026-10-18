use std::collections::HashSet;

use crate::document::Document;
use crate::document::document_form_design::{add_to_form, add_to_page, ensure_form, unique_name};
use crate::error::{Error, Result};
use crate::geometry::{Matrix, Rect};
use crate::image::ImageStreams;
use crate::pages::{PageId, PageTree};
use crate::render::DisplaySpace;
use crate::signing::SignatureMark;
use crate::syntax::{Dict, Name, ObjId, Object, Stream, encode_text};

const PRINT: i64 = 4;
const PICTURE: &str = "Signature";

impl Document {
    pub fn add_signature(&mut self, page: PageId, rect: Rect, name: &str, mark: &SignatureMark) -> Result<ObjId> {
        let page = PageTree::new(&self.store)
            .pages()?
            .into_iter()
            .find(|candidate| candidate.id == page)
            .ok_or_else(|| Error::new(format!("no page {page}")))?;
        let space = DisplaySpace::of(&page)?;
        let extent = mark.extent()?;
        let scale = (rect.width() / extent.width()).min(rect.height() / extent.height());
        if !scale.is_finite() || scale <= 0.0 {
            return Err(Error::unsupported("a signature needs room on the page"));
        }
        let (width, height) = (extent.width() * scale, extent.height() * scale);
        let left = rect.x0 + (rect.width() - width) / 2.0;
        let top = rect.y0 + (rect.height() - height) / 2.0;
        let widget_rect = space.rect_to_user(Rect::new(left, top, left + width, top + height));
        let frame = Matrix::new(scale, 0.0, 0.0, scale, left - extent.x0 * scale, top - extent.y0 * scale)
            .then(&space.display_to_user());
        let taken: HashSet<String> = self.fields()?.into_iter().map(|field| field.name).collect();
        let name = unique_name(if name.trim().is_empty() { "Signature" } else { name }, &taken);
        let picture = Name::from(PICTURE);
        self.store.transact("Add signature", "", |transaction| {
            let form = ensure_form(transaction)?;
            let mut resources = Dict::new();
            if let SignatureMark::Image(image) = mark {
                let id = ImageStreams::rgb(image).allocate(transaction);
                resources.insert("XObject", Object::Dict([(picture.clone(), Object::Ref(id))].into_iter().collect()));
            }
            let appearance: Dict = [
                ("Type", Object::Name(Name::from("XObject"))),
                ("Subtype", Object::Name(Name::from("Form"))),
                ("BBox", widget_rect.to_object()),
                ("Resources", Object::Dict(resources)),
            ]
            .into_iter()
            .map(|(key, value)| (Name::from(key), value))
            .collect();
            let content = mark.content(&frame, &picture);
            let appearance = transaction.allocate(Object::Stream(Stream { dict: appearance, raw: content.into() }));
            let widget: Dict = [
                ("Type", Object::Name(Name::from("Annot"))),
                ("Subtype", Object::Name(Name::from("Widget"))),
                ("FT", Object::Name(Name::from("Sig"))),
                ("T", Object::String(encode_text(&name))),
                ("Rect", widget_rect.to_object()),
                ("P", Object::Ref(page.id.0)),
                ("F", Object::Int(PRINT)),
                ("AP", Object::Dict([(Name::from("N"), Object::Ref(appearance))].into_iter().collect())),
            ]
            .into_iter()
            .map(|(key, value)| (Name::from(key), value))
            .collect();
            let id = transaction.allocate(Object::Dict(widget));
            add_to_form(transaction, form, id)?;
            add_to_page(transaction, page.id.0, id)?;
            Ok(id)
        })
    }
}
