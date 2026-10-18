use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use crate::annotations::{Annotation, annotation_list, write_annotation_list};
use crate::content::{ContentWriter, bind_xobject};
use crate::content::{Drawings, ImageDraw, ImageSource};
use crate::edit::{ContentPatch, EditTarget, PageText, RecordEdits, ShowWriter};
use crate::error::{Error, Result};
use crate::fonts::{FontCache, FontLibrary};
use crate::geometry::Quad;
use crate::image::ImageStreams;
use crate::organize::DeadLinks;
use crate::pages::Page;
use crate::redact::image_eraser::ImageEraser;
use crate::redact::redaction_mark::RedactionMark;
use crate::render::DisplaySpace;
use crate::store::{ObjectStore, Transaction};
use crate::syntax::{Dict, Name, ObjId, Object, Stream};
use crate::text::{GlyphOrigin, to_display_space};

const SHORTEST_QUOTED: usize = 3;
const MARKED_TEXT_KEYS: [&str; 3] = ["ActualText", "Alt", "E"];

struct ImageReplacement {
    scope: Option<ObjId>,
    name: Name,
    erased: ImageStreams,
}

pub struct PageRedaction {
    page: Page,
    marks: Vec<RedactionMark>,
    patches: Vec<ContentPatch>,
    images: Vec<ImageReplacement>,
    removed_annotations: HashSet<ObjId>,
    removed_text: Vec<String>,
}

pub struct Redactor<'a> {
    store: &'a ObjectStore,
    fonts: &'a FontCache,
    library: &'a FontLibrary,
}

impl<'a> Redactor<'a> {
    pub fn new(store: &'a ObjectStore, fonts: &'a FontCache, library: &'a FontLibrary) -> Self {
        Self { store, fonts, library }
    }

    pub fn plan(&self, page: &Page) -> Result<Option<PageRedaction>> {
        let marks = RedactionMark::on_page(self.store, page)?;
        if marks.is_empty() {
            return Ok(None);
        }
        let areas: Vec<Quad> = marks.iter().flat_map(|mark| mark.areas.iter().copied()).collect();
        let text = PageText::extract(self.store, self.fonts, self.library, page.id)?;
        let space = DisplaySpace::of(page)?;
        let (removed, removed_text) = covered_glyphs(&text, &space, &areas);
        let mut replacements: HashMap<Option<ObjId>, BTreeMap<usize, Vec<u8>>> = HashMap::new();
        let mut edits = RecordEdits::default();
        for origin in &removed {
            edits.delete(*origin);
        }
        let touched: HashSet<(Option<ObjId>, usize)> =
            removed.iter().map(|origin| (origin.form, origin.ordinal)).collect();
        for &(scope, ordinal) in &touched {
            let record = text.record(scope, ordinal)?;
            let font = record.font_name.as_ref().ok_or_else(|| Error::new("covered text has no font resource"))?;
            replacements
                .entry(scope)
                .or_default()
                .insert(ordinal, ShowWriter::new(record, font).rewrite(record, &edits.glyphs(record), None));
        }
        for (scope, ordinal, bytes) in marked_text_rewrites(&text, &touched) {
            replacements.entry(scope).or_default().insert(ordinal, bytes);
        }
        let drawings = Drawings::of(self.store, page)?;
        for path in drawings.paths.iter().filter(|path| areas.iter().any(|area| path.lies_within(area))) {
            let removed = replacements.entry(path.scope).or_default();
            removed.extend(path.ordinals.iter().map(|ordinal| (*ordinal, Vec::new())));
        }
        let images = self.erase_images(drawings.images, &areas, &mut replacements)?;
        let mut patches = Vec::new();
        for (scope, replacements) in replacements {
            let target = EditTarget::for_scope(self.store, page, scope)?;
            patches.push(ContentPatch {
                page: page.clone(),
                target,
                replacements: replacements.into_iter().collect(),
                appended: None,
            });
        }
        let removed_annotations = self.covered_annotations(page, &areas)?;
        Ok(Some(PageRedaction { page: page.clone(), marks, patches, images, removed_annotations, removed_text }))
    }

    fn erase_images(
        &self,
        draws: Vec<ImageDraw>,
        areas: &[Quad],
        replacements: &mut HashMap<Option<ObjId>, BTreeMap<usize, Vec<u8>>>,
    ) -> Result<Vec<ImageReplacement>> {
        let eraser = ImageEraser::new(self.store);
        let mut images = Vec::new();
        for (index, draw) in draws.into_iter().enumerate() {
            if !areas.iter().any(|area| area.overlaps(&Quad::unit_square(&draw.matrix))) {
                continue;
            }
            match draw.source {
                ImageSource::XObject { name, id } => {
                    let object = self.store.object(id)?;
                    let Some(stream) = object.as_stream() else { continue };
                    if let Some(erased) =
                        eraser.erase(&stream.dict, &stream.raw, &draw.resources, &draw.matrix, areas)?
                    {
                        images.push(ImageReplacement { scope: draw.scope, name, erased });
                    }
                }
                ImageSource::Inline { dict, data } => {
                    if let Some(erased) = eraser.erase(&dict, &data, &draw.resources, &draw.matrix, areas)? {
                        let name = Name::new(format!("TfRedactedImage{index}"));
                        let mut writer = ContentWriter::new();
                        writer.operation("Do", &[Object::Name(name.clone())]);
                        replacements.entry(draw.scope).or_default().insert(draw.ordinal, writer.finish());
                        images.push(ImageReplacement { scope: draw.scope, name, erased });
                    }
                }
            }
        }
        Ok(images)
    }

    fn covered_annotations(&self, page: &Page, areas: &[Quad]) -> Result<HashSet<ObjId>> {
        let mut covered = HashSet::new();
        for annotation in Annotation::on_page(self.store, page)? {
            let Some(id) = annotation.id else { continue };
            let reached = areas.iter().any(|area| area.overlaps(&Quad::from_rect(annotation.rect)));
            if reached || annotation.subtype.is("Redact") {
                covered.insert(id);
                covered.extend(annotation.dict.get("Popup").and_then(Object::as_reference));
            }
        }
        Ok(covered)
    }
}

impl PageRedaction {
    pub fn removed_text(&self) -> &[String] {
        &self.removed_text
    }

    pub fn apply(&self, transaction: &mut Transaction) -> Result<()> {
        let page_id = self.page.id.0;
        let mut page =
            transaction.object(page_id)?.as_dict().cloned().ok_or_else(|| Error::new("page is not a dictionary"))?;
        let mut resources = self.page.resources.clone();
        for replacement in &self.images {
            let image = replacement.erased.allocate(transaction);
            bind_xobject(transaction, &mut resources, replacement.scope, &replacement.name, image)?;
        }
        for patch in &self.patches {
            patch.apply(transaction, &mut page, &mut resources, &[])?;
        }
        let mut opening = ContentWriter::new();
        opening.operation("q", &[]);
        let mut cover = ContentWriter::new();
        cover.operation("Q", &[]).operation("q", &[]);
        for mark in &self.marks {
            cover.operation("rg", &mark.fill.map(Object::Real));
            for area in &mark.areas {
                area.write_path(&mut cover);
            }
            cover.operation("f", &[]);
        }
        cover.operation("Q", &[]);
        let mut contents = vec![Object::Ref(transaction.allocate(stream(opening.finish())))];
        contents.extend(page_contents(transaction, &page)?.into_iter().map(Object::Ref));
        contents.push(Object::Ref(transaction.allocate(stream(cover.finish()))));
        page.insert("Contents", Object::Array(contents));
        if resources != self.page.resources {
            page.insert("Resources", Object::Dict(resources));
        }
        transaction.put(page_id, Object::Dict(page))?;
        let kept = annotation_list(transaction, page_id)?
            .into_iter()
            .filter(|entry| entry.as_reference().is_none_or(|id| !self.removed_annotations.contains(&id)))
            .collect();
        write_annotation_list(transaction, page_id, kept)?;
        DeadLinks::prune_widgets(transaction, self.removed_annotations.clone())
    }
}

fn covered_glyphs(text: &PageText, space: &DisplaySpace, areas: &[Quad]) -> (Vec<GlyphOrigin>, Vec<String>) {
    let mut removed = Vec::new();
    let mut runs = Vec::new();
    for line in text.layout.lines() {
        let mut run = String::new();
        for cluster in &line.clusters {
            let corners = [
                (cluster.left, line.top),
                (cluster.right, line.top),
                (cluster.right, line.bottom),
                (cluster.left, line.bottom),
            ];
            let quad =
                Quad { corners: corners.map(|corner| space.point_to_user(to_display_space(line.angle, corner))) };
            let covered = !cluster.origins.is_empty() && areas.iter().any(|area| area.overlaps(&quad));
            if covered {
                removed.extend(cluster.origins.iter().copied());
                run.push_str(&cluster.text);
            } else if !cluster.is_space() || run.is_empty() {
                runs.push(std::mem::take(&mut run));
            } else {
                run.push(' ');
            }
        }
        runs.push(run);
    }
    let runs = runs
        .into_iter()
        .map(|run| run.trim().to_string())
        .filter(|run| run.chars().filter(|character| character.is_alphanumeric()).count() >= SHORTEST_QUOTED)
        .collect();
    (removed, runs)
}

fn marked_text_rewrites(
    text: &PageText,
    touched: &HashSet<(Option<ObjId>, usize)>,
) -> Vec<(Option<ObjId>, usize, Vec<u8>)> {
    let scopes: HashSet<Option<ObjId>> = touched.iter().map(|(scope, _)| *scope).collect();
    let mut rewrites = Vec::new();
    for scope in scopes {
        let Some(content) = text.content(scope) else { continue };
        let mut open: Vec<usize> = Vec::new();
        let mut quoting = HashSet::new();
        for (index, operation) in content.operations.iter().enumerate() {
            match operation.operator.as_str() {
                "BDC" | "BMC" => open.push(index),
                "EMC" => {
                    open.pop();
                }
                _ if touched.contains(&(scope, operation.ordinal)) => quoting.extend(open.iter().copied()),
                _ => {}
            }
        }
        for index in quoting {
            let operation = &content.operations[index];
            let (Some(tag), Some(Object::Dict(properties))) = (operation.operands.first(), operation.operands.get(1))
            else {
                continue;
            };
            if !MARKED_TEXT_KEYS.iter().any(|key| properties.get(key).is_some()) {
                continue;
            }
            let mut cleaned = properties.clone();
            for key in MARKED_TEXT_KEYS {
                cleaned.remove(key);
            }
            let mut writer = ContentWriter::new();
            writer.operation("BDC", &[tag.clone(), Object::Dict(cleaned)]);
            rewrites.push((scope, operation.ordinal, writer.finish()));
        }
    }
    rewrites
}

fn stream(content: Vec<u8>) -> Object {
    Object::Stream(Stream { dict: Dict::new(), raw: Rc::from(content) })
}

fn page_contents(transaction: &Transaction, page: &Dict) -> Result<Vec<ObjId>> {
    Ok(match page.get("Contents") {
        Some(Object::Ref(id)) => match &*transaction.object(*id)? {
            Object::Array(items) => items.iter().filter_map(Object::as_reference).collect(),
            _ => vec![*id],
        },
        Some(Object::Array(items)) => items.iter().filter_map(Object::as_reference).collect(),
        _ => Vec::new(),
    })
}
