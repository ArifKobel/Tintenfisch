use std::io::Write;
use std::rc::Rc;

use crate::error::{Error, Result};
use crate::file::{LatestSection, SectionFormat, XrefEntry};
use crate::save::document_id::DocumentId;
use crate::save::object_output::{compress_unfiltered, for_output};
use crate::store::{ObjectStore, Slot};
use crate::syntax::{Dict, Name, ObjId, Object, Serializer, Stream};

#[derive(Clone, Copy)]
enum Entry {
    Free { next: u32, generation: u16 },
    InUse { offset: usize, generation: u16 },
}

pub struct IncrementalWriter<'a> {
    store: &'a ObjectStore,
}

impl<'a> IncrementalWriter<'a> {
    pub fn new(store: &'a ObjectStore) -> Self {
        Self { store }
    }

    pub fn write(&self) -> Result<Vec<u8>> {
        let source = self.store.source();
        if !self.store.is_modified() {
            return Ok(source.bytes().to_vec());
        }
        let latest = source
            .xref()
            .latest()
            .filter(|_| !source.is_repaired())
            .ok_or_else(|| Error::new("a repaired file cannot be updated incrementally"))?;
        let mut out = source.bytes().to_vec();
        if !out.ends_with(b"\n") {
            out.push(b'\n');
        }
        let mut entries = self.write_objects(&mut out)?;
        let freed: Vec<u32> =
            entries.iter().filter(|(_, entry)| matches!(entry, Entry::Free { .. })).map(|(num, _)| *num).collect();
        let head = freed.first().copied().unwrap_or(0);
        for (num, entry) in entries.iter_mut() {
            if let Entry::Free { next, .. } = entry {
                *next = freed.iter().copied().find(|other| other > num).unwrap_or(0);
            }
        }
        entries.insert(0, (0, Entry::Free { next: head, generation: u16::MAX }));
        match latest.format {
            SectionFormat::Table => self.write_table(&mut out, &entries, latest),
            SectionFormat::Stream => self.write_stream(&mut out, &entries, latest),
        }
        Ok(out)
    }

    fn write_objects(&self, out: &mut Vec<u8>) -> Result<Vec<(u32, Entry)>> {
        let mut entries = Vec::new();
        for id in self.orphaned_members() {
            let offset = out.len();
            let object = self.store.object(id)?;
            Serializer::new(out).indirect(id, &for_output(&object, id, id, self.store.source().security()));
            entries.push((id.num, Entry::InUse { offset, generation: id.generation }));
        }
        for (num, slot) in self.store.changes() {
            match slot {
                Slot::Object { generation, object } => {
                    let offset = out.len();
                    let id = ObjId::new(num, *generation);
                    Serializer::new(out).indirect(id, &for_output(object, id, id, self.store.source().security()));
                    entries.push((num, Entry::InUse { offset, generation: *generation }));
                }
                Slot::Freed { next_generation } => {
                    entries.push((num, Entry::Free { next: 0, generation: *next_generation }));
                }
            }
        }
        entries.sort_by_key(|(num, _)| *num);
        Ok(entries)
    }

    fn orphaned_members(&self) -> Vec<ObjId> {
        let xref = self.store.source().xref();
        let changed: Vec<u32> = self.store.changes().iter().map(|(num, _)| *num).collect();
        xref.entries()
            .filter(|(num, entry)| {
                matches!(entry, XrefEntry::Compressed { stream, .. } if changed.contains(stream))
                    && !changed.contains(num)
            })
            .map(|(num, _)| ObjId::new(num, 0))
            .collect()
    }

    fn trailer(&self, size: u32, latest: LatestSection, written: &[u8]) -> Dict {
        let source_trailer = self.store.trailer();
        let mut trailer = Dict::new();
        trailer.insert("Size", Object::Int(size as i64));
        for key in ["Root", "Info", "Encrypt"] {
            if let Some(value) = source_trailer.get(key) {
                trailer.insert(key, value.clone());
            }
        }
        trailer.insert("ID", DocumentId::next(source_trailer, written));
        trailer.insert("Prev", Object::Int(latest.offset as i64));
        trailer
    }

    fn size(&self) -> u32 {
        self.store.next_num().max(self.store.source().xref().size())
    }

    fn write_table(&self, out: &mut Vec<u8>, entries: &[(u32, Entry)], latest: LatestSection) {
        let start = out.len();
        out.extend_from_slice(b"xref\n");
        for run in runs(entries) {
            let _ = writeln!(out, "{} {}", run[0].0, run.len());
            for (_, entry) in run {
                let _ = match entry {
                    Entry::Free { next, generation } => write!(out, "{next:010} {generation:05} f\r\n"),
                    Entry::InUse { offset, generation } => write!(out, "{offset:010} {generation:05} n\r\n"),
                };
            }
        }
        let trailer = self.trailer(self.size(), latest, out);
        out.extend_from_slice(b"trailer\n");
        Serializer::new(out).object(&Object::Dict(trailer));
        let _ = write!(out, "\nstartxref\n{start}\n%%EOF\n");
    }

    fn write_stream(&self, out: &mut Vec<u8>, entries: &[(u32, Entry)], latest: LatestSection) {
        let start = out.len();
        let stream_num = self.size();
        let mut entries = entries.to_vec();
        entries.push((stream_num, Entry::InUse { offset: start, generation: 0 }));
        let offset_width = byte_width(start as u64);
        let mut rows = Vec::new();
        for (_, entry) in &entries {
            let (kind, second, third) = match *entry {
                Entry::Free { next, generation } => (0u8, next as u64, generation),
                Entry::InUse { offset, generation } => (1u8, offset as u64, generation),
            };
            rows.push(kind);
            rows.extend_from_slice(&second.to_be_bytes()[8 - offset_width..]);
            rows.extend_from_slice(&third.to_be_bytes());
        }
        let index: Vec<Object> = runs(&entries)
            .iter()
            .flat_map(|run| [Object::Int(run[0].0 as i64), Object::Int(run.len() as i64)])
            .collect();
        let mut dict = self.trailer(stream_num + 1, latest, out);
        dict.insert("Type", Object::Name(Name::from("XRef")));
        dict.insert("W", Object::Array(vec![Object::Int(1), Object::Int(offset_width as i64), Object::Int(2)]));
        dict.insert("Index", Object::Array(index));
        let stream = Object::Stream(Stream { dict, raw: Rc::from(rows) });
        Serializer::new(out).indirect(ObjId::new(stream_num, 0), &compress_unfiltered(&stream));
        let _ = write!(out, "startxref\n{start}\n%%EOF\n");
    }
}

fn runs(entries: &[(u32, Entry)]) -> Vec<&[(u32, Entry)]> {
    let mut runs = Vec::new();
    let mut start = 0;
    for index in 1..=entries.len() {
        if index == entries.len() || entries[index].0 != entries[index - 1].0 + 1 {
            runs.push(&entries[start..index]);
            start = index;
        }
    }
    runs
}

fn byte_width(value: u64) -> usize {
    (8 - value.leading_zeros() as usize / 8).max(1)
}
