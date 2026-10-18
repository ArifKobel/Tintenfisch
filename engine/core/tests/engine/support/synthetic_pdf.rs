use std::collections::BTreeMap;
use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;

#[derive(Clone, Copy)]
pub enum Row {
    Free(u16),
    InUse(u32),
    Compressed(u32, u32),
}

pub struct SyntheticPdf {
    bytes: Vec<u8>,
    offsets: BTreeMap<u32, usize>,
}

impl SyntheticPdf {
    pub fn new(version: &str) -> Self {
        let mut bytes = format!("%PDF-{version}\n").into_bytes();
        bytes.extend_from_slice(b"%\xe2\xe3\xcf\xd3\n");
        Self { bytes, offsets: BTreeMap::new() }
    }

    pub fn position(&self) -> usize {
        self.bytes.len()
    }

    pub fn offset_of(&self, num: u32) -> usize {
        self.offsets[&num]
    }

    pub fn mark(&mut self, num: u32) -> &mut Self {
        self.offsets.insert(num, self.position());
        self
    }

    pub fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.bytes.extend_from_slice(bytes);
        self
    }

    pub fn object(&mut self, num: u32, generation: u16, body: &str) -> &mut Self {
        self.offsets.insert(num, self.position());
        self.raw(format!("{num} {generation} obj\n{body}\nendobj\n").as_bytes())
    }

    pub fn stream(&mut self, num: u32, entries: &str, data: &[u8]) -> &mut Self {
        self.offsets.insert(num, self.position());
        self.raw(format!("{num} 0 obj\n<< {entries} /Length {} >>\nstream\n", data.len()).as_bytes())
            .raw(data)
            .raw(b"\nendstream\nendobj\n")
    }

    pub fn object_stream(&mut self, num: u32, members: &[(u32, &str)]) -> &mut Self {
        let mut header = String::new();
        let mut body = String::new();
        for (member, source) in members {
            header.push_str(&format!("{member} {} ", body.len()));
            body.push_str(source);
            body.push('\n');
        }
        let data = deflate(format!("{header}{body}").as_bytes());
        let entries = format!("/Type /ObjStm /N {} /First {} /Filter /FlateDecode", members.len(), header.len());
        self.stream(num, &entries, &data)
    }

    pub fn xref_table(&mut self, rows: &[(u32, Row)], trailer: &str) -> usize {
        let start = self.position();
        self.raw(b"xref\n");
        let mut index = 0;
        while index < rows.len() {
            let run = rows[index..]
                .iter()
                .enumerate()
                .take_while(|(step, (num, _))| *num == rows[index].0 + *step as u32)
                .count();
            self.raw(format!("{} {}\n", rows[index].0, run).as_bytes());
            for (_, row) in &rows[index..index + run] {
                let line = match *row {
                    Row::Free(next) => format!("{:010} {:05} f\r\n", 0, next),
                    Row::InUse(num) => format!("{:010} {:05} n\r\n", self.offsets[&num], 0),
                    Row::Compressed(..) => panic!("a table cannot hold compressed entries"),
                };
                self.raw(line.as_bytes());
            }
            index += run;
        }
        self.raw(format!("trailer\n<< {trailer} >>\n").as_bytes());
        start
    }

    pub fn xref_stream(&mut self, num: u32, rows: &[(u32, Row)], entries: &str) -> usize {
        let start = self.position();
        self.offsets.insert(num, start);
        let mut data = Vec::new();
        let mut index = Vec::new();
        let mut previous = [0u8; 4];
        for (position, (row_num, row)) in rows.iter().enumerate() {
            if position == 0 || rows[position - 1].0 + 1 != *row_num {
                index.push(vec![*row_num, 0]);
            }
            index.last_mut().unwrap()[1] += 1;
            let fields: [u8; 4] = match *row {
                Row::Free(next) => [0, 0, 0, next as u8],
                Row::InUse(target) => {
                    let offset = if target == num { start } else { self.offsets[&target] } as u16;
                    [1, (offset >> 8) as u8, offset as u8, 0]
                }
                Row::Compressed(stream, slot) => [2, (stream >> 8) as u8, stream as u8, slot as u8],
            };
            data.push(2);
            data.extend(fields.iter().zip(previous).map(|(value, above)| value.wrapping_sub(above)));
            previous = fields;
        }
        let index: Vec<String> = index.iter().map(|range| format!("{} {}", range[0], range[1])).collect();
        let compressed = deflate(&data);
        let dict = format!(
            "/Type /XRef /W [1 2 1] /Index [{}] /Filter /FlateDecode /DecodeParms << /Predictor 12 /Columns 4 >> {entries}",
            index.join(" ")
        );
        self.raw(format!("{num} 0 obj\n<< {dict} /Length {} >>\nstream\n", compressed.len()).as_bytes())
            .raw(&compressed)
            .raw(b"\nendstream\nendobj\n");
        start
    }

    pub fn startxref(&mut self, offset: usize) -> &mut Self {
        self.raw(format!("startxref\n{offset}\n%%EOF\n").as_bytes())
    }

    pub fn finish(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

pub fn deflate(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}
