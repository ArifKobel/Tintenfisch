use std::collections::HashSet;

use crate::content::path_draw::PathDraw;
use crate::content::{ContentParser, ContentStream, Operation};
use crate::error::Result;
use crate::geometry::Matrix;
use crate::pages::Page;
use crate::store::ObjectStore;
use crate::syntax::{Dict, Name, ObjId, Object};

const MAX_NESTING: usize = 12;
const MITER_REACH: f64 = 5.0;
const STROKES: [&str; 6] = ["S", "s", "B", "B*", "b", "b*"];
const PAINTS: [&str; 10] = ["S", "s", "f", "F", "f*", "B", "B*", "b", "b*", "n"];

#[derive(Clone, Debug, PartialEq)]
pub enum ImageSource {
    XObject { name: Name, id: ObjId },
    Inline { dict: Dict, data: Vec<u8> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageDraw {
    pub scope: Option<ObjId>,
    pub ordinal: usize,
    pub source: ImageSource,
    pub matrix: Matrix,
    pub resources: Dict,
}

#[derive(Clone, Copy)]
struct DrawState {
    matrix: Matrix,
    line_width: f64,
}

#[derive(Default)]
struct OpenPath {
    ordinals: Vec<usize>,
    points: Vec<(f64, f64)>,
    clips: bool,
}

#[derive(Default)]
pub struct Drawings {
    pub images: Vec<ImageDraw>,
    pub paths: Vec<PathDraw>,
}

struct DrawingWalker<'s> {
    store: &'s ObjectStore,
    drawings: Drawings,
    forms: HashSet<ObjId>,
}

impl Drawings {
    pub fn of(store: &ObjectStore, page: &Page) -> Result<Drawings> {
        let content = ContentStream::load(store, page)?;
        let mut walker = DrawingWalker { store, drawings: Drawings::default(), forms: HashSet::new() };
        let base = DrawState { matrix: Matrix::IDENTITY, line_width: 1.0 };
        walker.walk(&content.operations, &page.resources, base, None);
        Ok(walker.drawings)
    }
}

impl DrawingWalker<'_> {
    fn walk(&mut self, operations: &[Operation], resources: &Dict, base: DrawState, scope: Option<ObjId>) {
        let mut state = base;
        let mut saved = Vec::new();
        let mut path = OpenPath::default();
        for operation in operations {
            let numbers: Vec<f64> = operation.operands.iter().filter_map(Object::as_number).collect();
            let operator = operation.operator.as_str();
            match operator {
                "q" => saved.push(state),
                "Q" => state = saved.pop().unwrap_or(base),
                "cm" => {
                    if let Some(step) = Matrix::from_numbers(&numbers) {
                        state.matrix = step.then(&state.matrix);
                    }
                }
                "w" => state.line_width = numbers.first().copied().unwrap_or(state.line_width),
                "m" | "l" | "c" | "v" | "y" | "h" => {
                    path.ordinals.push(operation.ordinal);
                    path.points.extend(numbers.as_chunks::<2>().0.iter().map(|[x, y]| state.matrix.apply(*x, *y)));
                }
                "re" => {
                    path.ordinals.push(operation.ordinal);
                    if let [x, y, width, height] = numbers[..] {
                        let corners = [(x, y), (x + width, y), (x + width, y + height), (x, y + height)];
                        path.points.extend(corners.map(|(x, y)| state.matrix.apply(x, y)));
                    }
                }
                "W" | "W*" => {
                    path.ordinals.push(operation.ordinal);
                    path.clips = true;
                }
                _ if PAINTS.contains(&operator) => {
                    let mut finished = std::mem::take(&mut path);
                    if !finished.clips {
                        finished.ordinals.push(operation.ordinal);
                        let reach = if STROKES.contains(&operator) {
                            state.line_width.max(1.0) * state.matrix.expansion() * MITER_REACH
                        } else {
                            0.0
                        };
                        let OpenPath { ordinals, points, .. } = finished;
                        self.drawings.paths.push(PathDraw { scope, ordinals, points, reach });
                    }
                }
                "BI" => {
                    if let (Some(Object::Dict(dict)), Some(Object::String(data))) =
                        (operation.operands.first(), operation.operands.get(1))
                    {
                        let source = ImageSource::Inline { dict: dict.clone(), data: data.clone() };
                        self.drawings.images.push(ImageDraw {
                            scope,
                            ordinal: operation.ordinal,
                            source,
                            matrix: state.matrix,
                            resources: resources.clone(),
                        });
                    }
                }
                "Do" => self.draw(operation, resources, state, scope),
                _ => {}
            }
        }
    }

    fn draw(&mut self, operation: &Operation, resources: &Dict, state: DrawState, scope: Option<ObjId>) {
        let Some(name) = operation.operands.first().and_then(Object::as_name) else { return };
        let xobjects = resources.get("XObject").and_then(|xobjects| self.store.resolve(xobjects).ok());
        let Some(id) = xobjects
            .as_ref()
            .and_then(Object::as_dict)
            .and_then(|xobjects| xobjects.get_name(name))
            .and_then(Object::as_reference)
        else {
            return;
        };
        let Ok(object) = self.store.object(id) else { return };
        let Some(stream) = object.as_stream() else { return };
        if stream.dict.name("Subtype").is_some_and(|subtype| subtype.is("Image")) {
            self.drawings.images.push(ImageDraw {
                scope,
                ordinal: operation.ordinal,
                source: ImageSource::XObject { name: name.clone(), id },
                matrix: state.matrix,
                resources: resources.clone(),
            });
            return;
        }
        if self.forms.len() >= MAX_NESTING || !self.forms.insert(id) {
            return;
        }
        let Ok(decoded) = self.store.decode_stream(stream) else { return };
        let form_matrix = stream.dict.get("Matrix").and_then(Matrix::from_object).unwrap_or(Matrix::IDENTITY);
        let form_resources = stream
            .dict
            .get("Resources")
            .and_then(|own| self.store.resolve(own).ok())
            .and_then(|own| own.as_dict().cloned())
            .unwrap_or_else(|| resources.clone());
        let operations = ContentParser::new(&decoded.data, 0).parse(0);
        let inner = DrawState { matrix: form_matrix.then(&state.matrix), ..state };
        self.walk(&operations, &form_resources, inner, Some(id));
        self.forms.remove(&id);
    }
}
