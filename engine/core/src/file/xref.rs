use std::collections::BTreeMap;

use crate::syntax::{Dict, ObjId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XrefEntry {
    Free { next_generation: u16 },
    InUse { offset: usize, generation: u16 },
    Compressed { stream: u32, index: u32 },
}

impl XrefEntry {
    pub fn generation(&self) -> u16 {
        match self {
            XrefEntry::Free { next_generation } => *next_generation,
            XrefEntry::InUse { generation, .. } => *generation,
            XrefEntry::Compressed { .. } => 0,
        }
    }

    pub fn is_free(&self) -> bool {
        matches!(self, XrefEntry::Free { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionFormat {
    Table,
    Stream,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LatestSection {
    pub format: SectionFormat,
    pub offset: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct XrefSection {
    pub format: SectionFormat,
    pub entries: BTreeMap<u32, XrefEntry>,
    pub trailer: Dict,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Xref {
    entries: BTreeMap<u32, XrefEntry>,
    trailer: Dict,
    latest: Option<LatestSection>,
}

impl Xref {
    pub fn new(entries: BTreeMap<u32, XrefEntry>, trailer: Dict, latest: Option<LatestSection>) -> Self {
        Self { entries, trailer, latest }
    }

    pub fn from_sections(sections: Vec<XrefSection>, latest_offset: usize) -> Self {
        let latest = sections.first().map(|section| LatestSection { format: section.format, offset: latest_offset });
        let mut entries = BTreeMap::new();
        let mut trailer = Dict::new();
        for section in sections {
            for (num, entry) in section.entries {
                entries.entry(num).or_insert(entry);
            }
            for (key, value) in section.trailer.iter() {
                trailer.insert_missing(key.clone(), value.clone());
            }
        }
        for key in ["Prev", "XRefStm", "Type", "W", "Index", "Filter", "DecodeParms", "Length"] {
            trailer.remove(key);
        }
        Self { entries, trailer, latest }
    }

    pub fn entry(&self, num: u32) -> Option<XrefEntry> {
        self.entries.get(&num).copied()
    }

    pub fn entries(&self) -> impl Iterator<Item = (u32, XrefEntry)> + '_ {
        self.entries.iter().map(|(&num, &entry)| (num, entry))
    }

    pub fn ids(&self) -> impl Iterator<Item = ObjId> + '_ {
        self.entries().filter(|(_, entry)| !entry.is_free()).map(|(num, entry)| ObjId::new(num, entry.generation()))
    }

    pub fn trailer(&self) -> &Dict {
        &self.trailer
    }

    pub fn latest(&self) -> Option<LatestSection> {
        self.latest
    }

    pub fn size(&self) -> u32 {
        let declared = self.trailer.int("Size").and_then(|size| u32::try_from(size).ok()).unwrap_or(0);
        let highest = self.entries.keys().next_back().map_or(0, |num| num + 1);
        declared.max(highest)
    }
}
