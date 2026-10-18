use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use tintenfisch::file::SourceFile;
use tintenfisch::store::ObjectStore;
use tintenfisch::syntax::{Dict, ObjId, Object, Stream};

pub trait ObjectSource {
    fn trailer(&self) -> &Dict;
    fn object(&self, id: ObjId) -> Rc<Object>;
    fn decoded(&self, stream: &Stream) -> Vec<u8>;
}

impl ObjectSource for SourceFile {
    fn trailer(&self) -> &Dict {
        SourceFile::trailer(self)
    }
    fn object(&self, id: ObjId) -> Rc<Object> {
        SourceFile::object(self, id).unwrap()
    }
    fn decoded(&self, stream: &Stream) -> Vec<u8> {
        self.decode_stream(stream).unwrap().data
    }
}

impl ObjectSource for ObjectStore {
    fn trailer(&self) -> &Dict {
        ObjectStore::trailer(self)
    }
    fn object(&self, id: ObjId) -> Rc<Object> {
        ObjectStore::object(self, id).unwrap()
    }
    fn decoded(&self, stream: &Stream) -> Vec<u8> {
        self.decode_stream(stream).unwrap().data
    }
}

pub fn canonical_graph(source: &impl ObjectSource) -> Vec<Object> {
    let mut numbers: HashMap<ObjId, u32> = HashMap::new();
    let mut queue = VecDeque::new();
    let mut graph = Vec::new();
    for key in ["Root", "Info"] {
        if let Some(id) = source.trailer().get(key).and_then(Object::as_reference) {
            visit(id, source, &mut numbers, &mut queue);
        }
    }
    while let Some(id) = queue.pop_front() {
        let object = source.object(id);
        graph.push(canonical(&object, source, &mut numbers, &mut queue));
    }
    graph
}

fn visit(
    id: ObjId,
    source: &impl ObjectSource,
    numbers: &mut HashMap<ObjId, u32>,
    queue: &mut VecDeque<ObjId>,
) -> Object {
    if *source.object(id) == Object::Null {
        return Object::Null;
    }
    let next = numbers.len() as u32;
    let number = *numbers.entry(id).or_insert_with(|| {
        queue.push_back(id);
        next
    });
    Object::Ref(ObjId::new(number, 0))
}

fn canonical(
    object: &Object,
    source: &impl ObjectSource,
    numbers: &mut HashMap<ObjId, u32>,
    queue: &mut VecDeque<ObjId>,
) -> Object {
    match object {
        Object::Ref(id) => visit(*id, source, numbers, queue),
        Object::Array(items) => {
            Object::Array(items.iter().map(|item| canonical(item, source, numbers, queue)).collect())
        }
        Object::Dict(dict) => Object::Dict(canonical_dict(dict, source, numbers, queue)),
        Object::Stream(stream) => {
            let mut dict = canonical_dict(&stream.dict, source, numbers, queue);
            for key in ["Filter", "DecodeParms", "Length"] {
                dict.remove(key);
            }
            Object::Stream(Stream { dict, raw: Rc::from(source.decoded(stream)) })
        }
        other => other.clone(),
    }
}

fn canonical_dict(
    dict: &Dict,
    source: &impl ObjectSource,
    numbers: &mut HashMap<ObjId, u32>,
    queue: &mut VecDeque<ObjId>,
) -> Dict {
    dict.iter()
        .map(|(key, value)| (key.clone(), canonical(value, source, numbers, queue)))
        .filter(|(_, value)| *value != Object::Null)
        .collect()
}
