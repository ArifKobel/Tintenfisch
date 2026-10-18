use crate::content::{ContentStream, ContentWriter, Operation, bind_xobject};
use crate::edit::{ContentPatch, EditTarget};
use crate::error::{Error, Result};
use crate::geometry::Matrix;
use crate::image::ImageStreams;
use crate::page_images::{ImageFile, ImageFrame, PlacedImage};
use crate::pages::Page;
use crate::render::DisplaySpace;
use crate::store::{ObjectStore, Transaction};
use crate::syntax::{Dict, Name, ObjId, Object};

struct AddedImage {
    scope: Option<ObjId>,
    name: Name,
    streams: ImageStreams,
}

pub struct ImageEdit {
    patch: ContentPatch,
    added: Option<AddedImage>,
}

impl ImageEdit {
    pub fn moved(store: &ObjectStore, page: &Page, index: usize, frame: &ImageFrame) -> Result<ImageEdit> {
        let placed = PlacedImage::nth(store, page, index)?;
        let space = DisplaySpace::of(page)?;
        let shown = frame.moved(&placed.matrix.then(&space.user_to_display())).ok_or_else(without_area)?;
        let step = shown.then(&space.display_to_user()).then(&placed.matrix.invert().ok_or_else(without_area)?);
        let content = placed.content(store, page)?;
        let position = position_of(&content, placed.ordinal)?;
        let replacement = match transform_around(&content.operations, position) {
            Some((ordinal, own)) => (ordinal, transform(&step.then(&own))),
            None => (placed.ordinal, wrapped(&step, raw(&content, &content.operations[position]))),
        };
        ImageEdit::patching(store, page, &placed, vec![replacement], None)
    }

    pub fn deleted(store: &ObjectStore, page: &Page, index: usize) -> Result<ImageEdit> {
        let placed = PlacedImage::nth(store, page, index)?;
        ImageEdit::patching(store, page, &placed, vec![(placed.ordinal, Vec::new())], None)
    }

    pub fn replaced(store: &ObjectStore, page: &Page, index: usize, file: ImageFile) -> Result<ImageEdit> {
        let placed = PlacedImage::nth(store, page, index)?;
        let name = free_name(store, &placed.resources)?;
        let shown = placed.frame.width / placed.frame.height.max(f64::EPSILON);
        let fitted = if file.aspect() < shown {
            let width = file.aspect() / shown;
            Matrix::new(width, 0.0, 0.0, 1.0, (1.0 - width) / 2.0, 0.0)
        } else {
            let height = shown / file.aspect();
            Matrix::new(1.0, 0.0, 0.0, height, 0.0, (1.0 - height) / 2.0)
        };
        let placement = file
            .orientation
            .then(&fitted)
            .then(&placed.frame.mapping())
            .then(&DisplaySpace::of(page)?.display_to_user())
            .then(&placed.matrix.invert().ok_or_else(without_area)?);
        let drawing = drawn(&placement, &name);
        let added = AddedImage { scope: placed.scope, name, streams: file.streams };
        ImageEdit::patching(store, page, &placed, vec![(placed.ordinal, drawing)], Some(added))
    }

    pub fn inserted(store: &ObjectStore, page: &Page, file: ImageFile, frame: &ImageFrame) -> Result<ImageEdit> {
        let name = free_name(store, &page.resources)?;
        let placement = file.orientation.then(&frame.mapping()).then(&DisplaySpace::of(page)?.display_to_user());
        let patch = ContentPatch {
            page: page.clone(),
            target: EditTarget::PageContent,
            replacements: Vec::new(),
            appended: Some(drawn(&placement, &name)),
        };
        Ok(ImageEdit { patch, added: Some(AddedImage { scope: None, name, streams: file.streams }) })
    }

    fn patching(
        store: &ObjectStore,
        page: &Page,
        placed: &PlacedImage,
        replacements: Vec<(usize, Vec<u8>)>,
        added: Option<AddedImage>,
    ) -> Result<ImageEdit> {
        let target = EditTarget::for_scope(store, page, placed.scope)?;
        Ok(ImageEdit { patch: ContentPatch { page: page.clone(), target, replacements, appended: None }, added })
    }

    pub fn apply(&self, transaction: &mut Transaction) -> Result<()> {
        let page_id = self.patch.page.id.0;
        let mut page =
            transaction.object(page_id)?.as_dict().cloned().ok_or_else(|| Error::new("page is not a dictionary"))?;
        let mut resources = self.patch.page.resources.clone();
        if let Some(added) = &self.added {
            let image = added.streams.allocate(transaction);
            bind_xobject(transaction, &mut resources, added.scope, &added.name, image)?;
        }
        self.patch.apply(transaction, &mut page, &mut resources, &[])?;
        if resources != self.patch.page.resources {
            page.insert("Resources", Object::Dict(resources));
        }
        transaction.put(page_id, Object::Dict(page))
    }
}

fn without_area() -> Error {
    Error::unsupported("the image is drawn without area")
}

fn position_of(content: &ContentStream, ordinal: usize) -> Result<usize> {
    content
        .operations
        .iter()
        .position(|operation| operation.ordinal == ordinal)
        .ok_or_else(|| Error::new("the image's operation is gone"))
}

fn transform_around(operations: &[Operation], position: usize) -> Option<(usize, Matrix)> {
    let before = operations.get(position.checked_sub(2)?..position)?;
    let after = operations.get(position + 1)?;
    let [save, cm] = before else { return None };
    if !save.is("q") || !cm.is("cm") || !after.is("Q") {
        return None;
    }
    let numbers: Vec<f64> = cm.operands.iter().filter_map(Object::as_number).collect();
    Some((cm.ordinal, Matrix::from_numbers(&numbers)?))
}

fn raw<'c>(content: &'c ContentStream, operation: &Operation) -> &'c [u8] {
    &content.segments[operation.segment].data[operation.span.clone()]
}

fn matrix_operands(matrix: &Matrix) -> [Object; 6] {
    [matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f].map(Object::Real)
}

fn transform(matrix: &Matrix) -> Vec<u8> {
    let mut writer = ContentWriter::new();
    writer.operation("cm", &matrix_operands(matrix));
    writer.finish()
}

fn wrapped(matrix: &Matrix, drawing: &[u8]) -> Vec<u8> {
    let mut writer = ContentWriter::new();
    writer.operation("q", &[]).operation("cm", &matrix_operands(matrix));
    let mut bytes = writer.finish();
    bytes.extend_from_slice(drawing);
    bytes.extend_from_slice(b"\nQ\n");
    bytes
}

fn drawn(matrix: &Matrix, name: &Name) -> Vec<u8> {
    let mut writer = ContentWriter::new();
    writer
        .operation("q", &[])
        .operation("cm", &matrix_operands(matrix))
        .operation("Do", &[Object::Name(name.clone())])
        .operation("Q", &[]);
    writer.finish()
}

fn free_name(store: &ObjectStore, resources: &Dict) -> Result<Name> {
    let xobjects = match resources.get("XObject") {
        Some(xobjects) => store.resolve(xobjects)?.as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    Ok((0..)
        .map(|index| Name::new(format!("TfImage{index}")))
        .find(|name| xobjects.get_name(name).is_none())
        .unwrap_or_else(|| Name::from("TfImage")))
}
