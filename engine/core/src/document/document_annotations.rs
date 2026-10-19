use std::rc::Rc;

use crate::annotations::{
    Annotation, AnnotationSummary, HighlightPlan, Markup, MarkupBuilder, MarkupStyle, TextBoxAppearance,
    annotation_list, write_annotation_list,
};
use crate::document::Document;
use crate::error::{Error, Result};
use crate::fonts::FontLibrary;
use crate::forms::DefaultAppearance;
use crate::geometry::{Matrix, Rect};
use crate::pages::{Page, PageId, PageTree};
use crate::render::DisplaySpace;
use crate::syntax::{Dict, Name, ObjId, Object, Stream, encode_text};

const PRINT: i64 = 4;
const POPUP_SIZE: (f64, f64) = (180.0, 100.0);

impl Document {
    pub fn annotations(&self, page: PageId) -> Result<Vec<AnnotationSummary>> {
        let page = self.page(page)?;
        AnnotationSummary::of(&page, &Annotation::on_page(&self.store, &page)?)
    }

    pub fn add_markup(
        &mut self,
        page: PageId,
        markup: &Markup,
        style: &MarkupStyle,
        library: &FontLibrary,
    ) -> Result<ObjId> {
        let page = self.page(page)?;
        let layout = self.view(library).text_layout(page.id)?;
        let (planned, replaced, notes) = match markup {
            Markup::Highlight { anchor, focus } => {
                let to_display = DisplaySpace::of(&page)?.user_to_display();
                let existing = Annotation::on_page(&self.store, &page)?;
                match HighlightPlan::new(&layout, &to_display, &existing, (*anchor, *focus), style.color) {
                    HighlightPlan::Unchanged(id) => return Ok(id),
                    HighlightPlan::Draw { anchor, focus, replaced, notes } => {
                        (Markup::Highlight { anchor, focus }, replaced, notes)
                    }
                }
            }
            other => (other.clone(), Vec::new(), Vec::new()),
        };
        let mut built = MarkupBuilder::new(&page, &layout, library)?.build(&planned, style)?;
        if !notes.is_empty() {
            built.entries.insert("Contents", Object::String(encode_text(&notes.join("\n\n"))));
        }
        let popup_rect = match markup {
            Markup::Note { at, .. } => Some(popup_rect(&page, *at)?),
            _ => None,
        };
        self.store.transact("Add comment", "", |transaction| {
            let form: Dict = [
                ("Type", Object::Name(Name::from("XObject"))),
                ("Subtype", Object::Name(Name::from("Form"))),
                ("BBox", built.rect.to_object()),
                ("Resources", Object::Dict(built.resources.clone())),
            ]
            .into_iter()
            .map(|(key, value)| (Name::from(key), value))
            .collect();
            let appearance =
                transaction.allocate(Object::Stream(Stream { dict: form, raw: Rc::from(built.appearance.clone()) }));
            let mut annotation = built.entries.clone();
            for (key, value) in [
                ("Type", Object::Name(Name::from("Annot"))),
                ("Subtype", Object::Name(Name::from(built.subtype))),
                ("Rect", built.rect.to_object()),
                ("C", Object::Array(style.color.iter().map(|value| Object::Real(*value)).collect())),
                ("F", Object::Int(PRINT)),
                ("P", Object::Ref(page.id.0)),
                ("M", Object::String(style.date.as_bytes().to_vec())),
                ("CreationDate", Object::String(style.date.as_bytes().to_vec())),
                ("AP", Object::Dict([(Name::from("N"), Object::Ref(appearance))].into_iter().collect())),
            ] {
                annotation.insert(key, value);
            }
            if !style.author.is_empty() {
                annotation.insert("T", Object::String(encode_text(&style.author)));
            }
            let id = transaction.allocate(Object::Dict(annotation.clone()));
            let mut added = vec![id];
            if let (true, Some(rect)) = (built.has_popup, popup_rect) {
                let popup: Dict = [
                    ("Type", Object::Name(Name::from("Annot"))),
                    ("Subtype", Object::Name(Name::from("Popup"))),
                    ("Rect", rect.to_object()),
                    ("Parent", Object::Ref(id)),
                    ("Open", Object::Bool(false)),
                    ("F", Object::Int(PRINT)),
                ]
                .into_iter()
                .map(|(key, value)| (Name::from(key), value))
                .collect();
                let popup = transaction.allocate(Object::Dict(popup));
                annotation.insert("Popup", Object::Ref(popup));
                transaction.put(id, Object::Dict(annotation))?;
                added.push(popup);
            }
            let mut gone = replaced.clone();
            for id in &replaced {
                gone.extend(
                    transaction
                        .object(*id)?
                        .as_dict()
                        .and_then(|dict| dict.get("Popup"))
                        .and_then(Object::as_reference),
                );
            }
            let mut annotations: Vec<Object> = annotation_list(transaction, page.id.0)?
                .into_iter()
                .filter(|entry| entry.as_reference().is_none_or(|id| !gone.contains(&id)))
                .collect();
            annotations.extend(added.into_iter().map(Object::Ref));
            write_annotation_list(transaction, page.id.0, annotations)?;
            Ok(id)
        })
    }

    pub fn delete_annotation(&mut self, page: PageId, annotation: ObjId) -> Result<()> {
        self.store.transact("Delete comment", "", |transaction| {
            let popup = transaction
                .object(annotation)?
                .as_dict()
                .and_then(|dict| dict.get("Popup"))
                .and_then(Object::as_reference);
            let annotations = annotation_list(transaction, page.0)?;
            let kept: Vec<Object> = annotations
                .iter()
                .filter(|entry| entry.as_reference().is_none_or(|id| id != annotation && Some(id) != popup))
                .cloned()
                .collect();
            if kept.len() == annotations.len() {
                return Err(Error::new("the comment is not on this page"));
            }
            write_annotation_list(transaction, page.0, kept)
        })
    }

    pub fn set_annotation_contents(
        &mut self,
        annotation: ObjId,
        contents: &str,
        date: &str,
        library: &FontLibrary,
    ) -> Result<Vec<PageId>> {
        let pages = PageTree::new(&self.store).pages()?;
        let page = pages.into_iter().find(|page| {
            Annotation::on_page(&self.store, page)
                .is_ok_and(|annotations| annotations.iter().any(|candidate| candidate.id == Some(annotation)))
        });
        let mut redrawn = Vec::new();
        self.store.transact("Edit comment", "", |transaction| {
            let mut dict = transaction
                .object(annotation)?
                .as_dict()
                .cloned()
                .ok_or_else(|| Error::new("the comment is not a dictionary"))?;
            dict.insert("Contents", Object::String(encode_text(contents)));
            dict.insert("M", Object::String(date.as_bytes().to_vec()));
            if let Some(page) =
                page.as_ref().filter(|_| dict.name("Subtype").is_some_and(|subtype| subtype.is("FreeText")))
            {
                let rect = dict
                    .get("Rect")
                    .and_then(Rect::from_object)
                    .ok_or_else(|| Error::new("the text box has no rectangle"))?;
                let space = DisplaySpace::of(page)?;
                let shown = space.rect_to_display(rect);
                let frame = Matrix::new(1.0, 0.0, 0.0, -1.0, shown.x0, shown.y1).then(&space.display_to_user());
                let size = Some(DefaultAppearance::parse(dict.get("DA").and_then(Object::as_string)).size)
                    .filter(|size| *size > 0.0)
                    .unwrap_or(12.0);
                let color = dict
                    .get("C")
                    .and_then(Object::as_array)
                    .and_then(|color| {
                        <[f64; 3]>::try_from(color.iter().filter_map(Object::as_number).collect::<Vec<_>>()).ok()
                    })
                    .unwrap_or([0.0; 3]);
                let content = TextBoxAppearance::new(library).content(
                    &frame,
                    shown.width(),
                    shown.height(),
                    contents,
                    size,
                    color,
                );
                let form: Dict = [
                    ("Type", Object::Name(Name::from("XObject"))),
                    ("Subtype", Object::Name(Name::from("Form"))),
                    ("BBox", rect.to_object()),
                    ("Resources", Object::Dict(TextBoxAppearance::resources())),
                ]
                .into_iter()
                .map(|(key, value)| (Name::from(key), value))
                .collect();
                let appearance = transaction.allocate(Object::Stream(Stream { dict: form, raw: Rc::from(content) }));
                dict.insert("AP", Object::Dict([(Name::from("N"), Object::Ref(appearance))].into_iter().collect()));
                redrawn.push(page.id);
            }
            transaction.put(annotation, Object::Dict(dict))
        })?;
        Ok(redrawn)
    }

    pub(super) fn page(&self, page: PageId) -> Result<Page> {
        PageTree::new(&self.store)
            .pages()?
            .into_iter()
            .find(|candidate| candidate.id == page)
            .ok_or_else(|| Error::new(format!("no page {page}")))
    }
}

fn popup_rect(page: &Page, (x, y): (f64, f64)) -> Result<Rect> {
    Ok(DisplaySpace::of(page)?.rect_to_user(Rect::new(x + 24.0, y, x + 24.0 + POPUP_SIZE.0, y + POPUP_SIZE.1)))
}
