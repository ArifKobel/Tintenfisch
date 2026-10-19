use std::rc::Rc;

use crate::content::{ContentParser, ContentStream, Operation, resource};
use crate::error::Result;
use crate::fonts::{Font, FontCache, FontLibrary};
use crate::geometry::{Matrix, Rect};
use crate::pages::Page;
use crate::render::DisplaySpace;
use crate::store::ObjectStore;
use crate::syntax::{Dict, Name, ObjId, Object};
use crate::text::text_glyph::to_line_space;
use crate::text::{
    BLOCK_TAG, FillColor, GlyphOrigin, RecordedGlyph, ShowItem, ShowRecord, TextGlyph, TextMachine, TextState,
};

const MAX_NESTING: usize = 12;
const UNMAPPED: &str = "\u{FFFD}";

#[derive(Clone)]
struct ExtractionState {
    ctm: Matrix,
    fill: FillColor,
    text: TextState,
    font_name: Option<Name>,
}

pub struct TextExtractor<'a> {
    store: &'a ObjectStore,
    fonts: &'a FontCache,
    library: &'a FontLibrary,
}

struct Walk {
    display: Matrix,
    state: ExtractionState,
    saved: Vec<ExtractionState>,
    machine: TextMachine,
    forms: Vec<ObjId>,
    glyphs: Vec<TextGlyph>,
    records: Vec<ShowRecord>,
    marks: Vec<Option<i64>>,
}

impl Walk {
    fn block(&self) -> Option<i64> {
        self.marks.iter().rev().find_map(|block| *block)
    }
}

impl<'a> TextExtractor<'a> {
    pub fn new(store: &'a ObjectStore, fonts: &'a FontCache, library: &'a FontLibrary) -> Self {
        Self { store, fonts, library }
    }

    pub fn page(&self, page: &Page) -> Result<Vec<TextGlyph>> {
        Ok(self.page_with_records(page)?.0)
    }

    pub fn page_with_records(&self, page: &Page) -> Result<(Vec<TextGlyph>, Vec<ShowRecord>)> {
        let display = DisplaySpace::of(page)?.user_to_display();
        let content = ContentStream::load(self.store, page)?;
        let mut walk = Walk {
            display,
            state: ExtractionState {
                ctm: Matrix::IDENTITY,
                fill: FillColor::default(),
                text: TextState::default(),
                font_name: None,
            },
            saved: Vec::new(),
            machine: TextMachine::default(),
            forms: Vec::new(),
            glyphs: Vec::new(),
            records: Vec::new(),
            marks: Vec::new(),
        };
        self.run(&mut walk, &content.operations, &page.resources, None);
        Ok((walk.glyphs, walk.records))
    }

    fn run(&self, walk: &mut Walk, operations: &[Operation], resources: &Dict, form: Option<ObjId>) {
        for operation in operations {
            match operation.operator.as_str() {
                "q" => walk.saved.push(walk.state.clone()),
                "Q" => {
                    if let Some(state) = walk.saved.pop() {
                        walk.state = state;
                    }
                }
                "cm" => {
                    let numbers: Vec<f64> = operation.operands.iter().filter_map(Object::as_number).collect();
                    if let Some(matrix) = Matrix::from_numbers(&numbers) {
                        walk.state.ctm = matrix.then(&walk.state.ctm);
                    }
                }
                "Tf" => {
                    walk.state.font_name = operation.operands.first().and_then(Object::as_name).cloned();
                    walk.state.text.font = operation
                        .operands
                        .first()
                        .and_then(|name| resource(self.store, resources, "Font", name))
                        .and_then(|font| self.fonts.font(&font, self.store, self.library).ok())
                        .or(walk.state.text.font.clone());
                }
                "g" | "rg" | "k" | "cs" | "sc" | "scn" => {
                    walk.state.fill.apply(&operation.operator, &operation.operands)
                }
                "Do" => self.form(walk, operation, resources),
                "BDC" | "BMC" => {
                    let tagged =
                        operation.operands.first().and_then(Object::as_name).is_some_and(|tag| tag.is(BLOCK_TAG));
                    let id =
                        operation.operands.get(1).and_then(Object::as_dict).and_then(|properties| properties.int("Id"));
                    let block = if tagged { id } else { walk.block() };
                    walk.marks.push(block.or(walk.block()));
                }
                "EMC" => {
                    walk.marks.pop();
                }
                _ => {}
            }
            let mut index = 0;
            let items = walk.machine.apply(operation, &mut walk.state.text);
            if items.is_empty() {
                continue;
            }
            let start = walk.machine.text_matrix();
            let mut record = ShowRecord {
                form,
                ordinal: operation.ordinal,
                operator: operation.operator.clone(),
                font_name: walk.state.font_name.clone(),
                font: walk.state.text.font.clone(),
                state: walk.state.text.clone(),
                fill: walk.state.fill.clone(),
                text_matrix: start,
                line_matrix: walk.machine.line_matrix(),
                to_display: start.then(&walk.state.ctm).then(&walk.display),
                glyphs: Vec::new(),
                end: 0.0,
                block: walk.block(),
            };
            for item in items {
                match item {
                    ShowItem::Text(bytes) => {
                        let origin = GlyphOrigin { form, ordinal: operation.ordinal, index: 0 };
                        self.show(walk, bytes, origin, &mut index, &mut record);
                    }
                    ShowItem::Shift(amount) => walk.machine.shift(amount, &walk.state.text),
                }
            }
            record.end = pen_offset(&start, &walk.machine.text_matrix());
            walk.records.push(record);
        }
    }

    fn show(&self, walk: &mut Walk, bytes: &[u8], origin: GlyphOrigin, index: &mut usize, record: &mut ShowRecord) {
        let Some(font) = walk.state.text.font.clone() else { return };
        let (ascent, descent) = font.vertical_extent();
        for glyph in font.decode(bytes) {
            let glyph_to_display =
                walk.machine.glyph_to_user(&walk.state.text).then(&walk.state.ctm).then(&walk.display);
            let origin_point = glyph_to_display.apply(0.0, 0.0);
            let direction = glyph_to_display.apply(1.0, 0.0);
            let angle = (direction.1 - origin_point.1).atan2(direction.0 - origin_point.0);
            let line = |x: f64, y: f64| to_line_space(angle, glyph_to_display.apply(x, y));
            let corners =
                [line(0.0, descent), line(glyph.width, descent), line(0.0, ascent), line(glyph.width, ascent)];
            let (start, baseline) = line(0.0, 0.0);
            let (end, _) = line(glyph.width, 0.0);
            let (_, top) = line(0.0, 1.0);
            walk.glyphs.push(TextGlyph {
                origin: GlyphOrigin { index: *index, ..origin },
                text: text_of(&font, glyph.code, glyph.is_word_space),
                angle,
                bounds: bounds_of(&corners),
                baseline,
                start,
                end,
                size: (baseline - top).abs(),
                hidden: matches!(walk.state.text.render_mode, 3 | 7),
                block: record.block,
            });
            *index += 1;
            let before = pen_offset(&record.text_matrix, &walk.machine.text_matrix());
            walk.machine.advance(&glyph, &walk.state.text);
            let after = pen_offset(&record.text_matrix, &walk.machine.text_matrix());
            record.glyphs.push(RecordedGlyph {
                code: glyph.code,
                byte_length: glyph.byte_length,
                x: before,
                advance: after - before,
            });
        }
    }

    fn form(&self, walk: &mut Walk, operation: &Operation, resources: &Dict) {
        let Some(id) = operation
            .operands
            .first()
            .and_then(|name| resource(self.store, resources, "XObject", name))
            .and_then(|object| object.as_reference())
        else {
            return;
        };
        if walk.forms.len() >= MAX_NESTING || walk.forms.contains(&id) {
            return;
        }
        let Ok(object) = self.store.object(id) else { return };
        let Some(stream) =
            object.as_stream().filter(|stream| stream.dict.name("Subtype").is_some_and(|subtype| subtype.is("Form")))
        else {
            return;
        };
        let Ok(decoded) = self.store.decode_stream(stream) else { return };
        let form_resources = stream
            .dict
            .get("Resources")
            .and_then(|own| self.store.resolve(own).ok())
            .and_then(|own| own.as_dict().cloned())
            .unwrap_or_else(|| resources.clone());
        let matrix = stream.dict.get("Matrix").and_then(Matrix::from_object).unwrap_or(Matrix::IDENTITY);
        let saved = (walk.state.clone(), walk.machine);
        walk.state.ctm = matrix.then(&walk.state.ctm);
        walk.forms.push(id);
        self.run(walk, &ContentParser::new(&decoded.data, 0).parse(0), &form_resources, Some(id));
        walk.forms.pop();
        (walk.state, walk.machine) = saved;
    }
}

fn pen_offset(start: &Matrix, now: &Matrix) -> f64 {
    start.invert().map_or(0.0, |inverse| now.then(&inverse).e)
}

fn text_of(font: &Rc<Font>, code: u32, is_word_space: bool) -> String {
    let text = font.unicode(code).unwrap_or_else(|| if is_word_space { " ".to_string() } else { UNMAPPED.to_string() });
    let mut expanded = String::with_capacity(text.len());
    for character in text.chars() {
        match ligature_letters(character) {
            Some(letters) => expanded.push_str(letters),
            None => expanded.push(character),
        }
    }
    expanded
}

fn ligature_letters(character: char) -> Option<&'static str> {
    Some(match character {
        '\u{FB00}' => "ff",
        '\u{FB01}' => "fi",
        '\u{FB02}' => "fl",
        '\u{FB03}' => "ffi",
        '\u{FB04}' => "ffl",
        '\u{FB05}' | '\u{FB06}' => "st",
        _ => return None,
    })
}

fn bounds_of(corners: &[(f64, f64); 4]) -> Rect {
    let xs = corners.map(|corner| corner.0);
    let ys = corners.map(|corner| corner.1);
    Rect::new(
        xs.iter().copied().fold(f64::INFINITY, f64::min),
        ys.iter().copied().fold(f64::INFINITY, f64::min),
        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}
