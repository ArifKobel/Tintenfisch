use std::collections::{HashMap, HashSet};

use crate::content::{ContentStream, Segment};
use crate::error::{Error, Result};
use crate::fonts::{FontCache, FontLibrary};
use crate::pages::{Page, PageId, PageTree};
use crate::store::ObjectStore;
use crate::syntax::ObjId;
use crate::text::{GlyphOrigin, LayoutLine, RecordedGlyph, ShowRecord, TextExtractor, TextLayout};

pub struct PageText {
    pub page: Page,
    pub layout: TextLayout,
    records: Vec<ShowRecord>,
    by_key: HashMap<(Option<ObjId>, usize), usize>,
    contents: HashMap<Option<ObjId>, ContentStream>,
    repeated: HashSet<ObjId>,
}

impl PageText {
    pub fn extract(store: &ObjectStore, fonts: &FontCache, library: &FontLibrary, page: PageId) -> Result<Self> {
        let page = PageTree::new(store)
            .pages()?
            .into_iter()
            .find(|candidate| candidate.id == page)
            .ok_or_else(|| Error::new(format!("no page {page}")))?;
        let (glyphs, records) = TextExtractor::new(store, fonts, library).page_with_records(&page)?;
        let mut by_key = HashMap::new();
        let mut repeated = HashSet::new();
        for (index, record) in records.iter().enumerate() {
            if by_key.insert((record.form, record.ordinal), index).is_some() {
                repeated.extend(record.form);
            }
        }
        let mut contents = HashMap::from([(None, ContentStream::load(store, &page)?)]);
        for form in records.iter().filter_map(|record| record.form).collect::<HashSet<_>>() {
            let object = store.object(form)?;
            let stream = object.as_stream().ok_or_else(|| Error::new("a form XObject is not a stream"))?;
            let data = store.decode_stream(stream)?.data;
            contents.insert(Some(form), ContentStream::parse(vec![Segment { stream: form, data }]));
        }
        Ok(Self { page, layout: TextLayout::new(&glyphs), records, by_key, contents, repeated })
    }

    pub fn next_block(&self) -> i64 {
        self.records.iter().filter_map(|record| record.block).max().map_or(1, |block| block + 1)
    }

    pub fn line(&self, index: usize) -> Result<&LayoutLine> {
        self.layout.lines().get(index).ok_or_else(|| Error::new("no such line"))
    }

    pub fn record(&self, form: Option<ObjId>, ordinal: usize) -> Result<&ShowRecord> {
        self.by_key
            .get(&(form, ordinal))
            .map(|&index| &self.records[index])
            .ok_or_else(|| Error::new("the edited operation was not recorded"))
    }

    pub fn glyph(&self, origin: &GlyphOrigin) -> Result<(&ShowRecord, &RecordedGlyph)> {
        let record = self.record(origin.form, origin.ordinal)?;
        let glyph = record.glyphs.get(origin.index).ok_or_else(|| Error::new("the edited glyph was not recorded"))?;
        Ok((record, glyph))
    }

    pub fn draws_repeatedly(&self, form: ObjId) -> bool {
        self.repeated.contains(&form)
    }

    pub fn content(&self, form: Option<ObjId>) -> Option<&ContentStream> {
        self.contents.get(&form)
    }

    pub fn operation_bytes(&self, form: Option<ObjId>, ordinal: usize) -> Result<Vec<u8>> {
        let content = self.contents.get(&form).ok_or_else(|| Error::new("the edited content is gone"))?;
        let operation = content
            .operations
            .iter()
            .find(|operation| operation.ordinal == ordinal)
            .ok_or_else(|| Error::new("the edited operation is gone"))?;
        Ok(content.segments[operation.segment].data[operation.span.clone()].to_vec())
    }
}
