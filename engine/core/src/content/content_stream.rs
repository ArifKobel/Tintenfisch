use crate::content::{ContentParser, Operation};
use crate::error::Result;
use crate::pages::Page;
use crate::store::ObjectStore;
use crate::syntax::ObjId;

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub stream: ObjId,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContentStream {
    pub segments: Vec<Segment>,
    pub operations: Vec<Operation>,
}

impl ContentStream {
    pub fn load(store: &ObjectStore, page: &Page) -> Result<ContentStream> {
        let mut segments = Vec::with_capacity(page.contents.len());
        for &id in &page.contents {
            let object = store.object(id)?;
            if let Some(stream) = object.as_stream() {
                segments.push(Segment { stream: id, data: store.decode_stream(stream)?.data });
            }
        }
        Ok(ContentStream::parse(segments))
    }

    pub fn parse(segments: Vec<Segment>) -> ContentStream {
        let mut operations = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            let parsed = ContentParser::new(&segment.data, index).parse(operations.len());
            operations.extend(parsed);
        }
        ContentStream { segments, operations }
    }
}
