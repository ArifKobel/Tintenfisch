use std::fmt::Write;

const OBJECTS: [&str; 2] = ["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [] /Count 0 >>"];

pub fn blank_document() -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in OBJECTS.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let start = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f\r\n", OBJECTS.len() + 1);
    for offset in offsets {
        let _ = write!(out, "{offset:010} 00000 n\r\n");
    }
    let _ = write!(out, "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n", OBJECTS.len() + 1);
    out.into_bytes()
}
