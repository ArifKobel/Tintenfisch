use std::fs;
use std::path::PathBuf;

use crate::support::jbig2_writer::{
    DEFAULT_ADAPTIVE, NewSymbol, Picture, Placed, Region, RegionCoding, TextLayout, embedded_stream,
    generic_region_data, halftone_region, page_information, pattern_dictionary, refinement_region, segment,
    symbol_dictionary, text_region,
};
use crate::support::synthetic_pdf::{Row, SyntheticPdf, deflate};

pub struct Fixture {
    pub name: String,
    pub bytes: Vec<u8>,
}

pub const CATALOG: &str = "<< /Type /Catalog /Pages 2 0 R >>";
pub const PAGES: &str = "<< /Type /Pages /Kids [3 0 R] /Count 1 >>";
pub const PAGE: &str = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >>";
pub const CONTENT: &[u8] = b"BT /F1 12 Tf 20 100 Td (Synthetic) Tj ET";

pub fn committed() -> Vec<Fixture> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut fixtures: Vec<Fixture> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .map(|path| Fixture {
            name: path.file_name().unwrap().to_string_lossy().into_owned(),
            bytes: fs::read(&path).unwrap(),
        })
        .collect();
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    fixtures
}

pub fn synthetic() -> Vec<Fixture> {
    [
        ("incremental-freed.pdf", incremental_with_freed_object()),
        ("xref-stream-objstm.pdf", xref_stream_with_object_streams()),
        ("hybrid.pdf", hybrid()),
        ("table-then-stream.pdf", table_then_stream_update()),
        ("wrong-offset.pdf", one_wrong_offset()),
        ("no-xref.pdf", without_xref()),
        ("indirect-length.pdf", indirect_length()),
        ("nested-page-tree.pdf", nested_page_tree()),
        ("shared-page.pdf", shared_page_with_cycle()),
        ("render-features.pdf", render_features()),
        ("image-features.pdf", image_features()),
        ("shading-features.pdf", shading_features()),
        ("annotation-features.pdf", annotation_features()),
        ("editing-features.pdf", editing_features()),
        ("reflow-features.pdf", reflow_features()),
        ("form-features.pdf", form_features()),
        ("organize-features.pdf", organize_features()),
        ("fillable-form.pdf", fillable_form()),
        ("redaction-features.pdf", redaction_features()),
        ("scan-features.pdf", scan_features("shapes")),
        ("jbig2-features.pdf", jbig2_features("shapes")),
        ("jbig2-symbol-features.pdf", jbig2_symbol_features()),
        ("jpx-features.pdf", jpx_features()),
        ("image-placement-features.pdf", image_placement_features()),
        ("navigation-features.pdf", navigation_features()),
    ]
    .into_iter()
    .map(|(name, bytes)| Fixture { name: name.to_string(), bytes })
    .collect()
}

pub fn all() -> Vec<Fixture> {
    committed().into_iter().chain(synthetic()).collect()
}

fn single_page() -> SyntheticPdf {
    let mut pdf = SyntheticPdf::new("1.4");
    pdf.object(1, 0, CATALOG).object(2, 0, PAGES).object(3, 0, PAGE).stream(4, "", CONTENT);
    pdf
}

fn single_page_rows() -> Vec<(u32, Row)> {
    vec![(0, Row::Free(65535)), (1, Row::InUse(1)), (2, Row::InUse(2)), (3, Row::InUse(3)), (4, Row::InUse(4))]
}

pub fn incremental_with_freed_object() -> Vec<u8> {
    let mut pdf = single_page();
    pdf.object(5, 0, "(first revision note)");
    let mut rows = single_page_rows();
    rows.push((5, Row::InUse(5)));
    let first = pdf.xref_table(&rows, "/Size 6 /Root 1 0 R");
    pdf.startxref(first);
    pdf.object(3, 0, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Contents 4 0 R >>").object(
        6,
        0,
        "(second revision note)",
    );
    let second = pdf.xref_table(
        &[(0, Row::Free(65535)), (3, Row::InUse(3)), (5, Row::Free(1)), (6, Row::InUse(6))],
        &format!("/Size 7 /Root 1 0 R /Prev {first}"),
    );
    pdf.startxref(second).finish()
}

pub fn xref_stream_with_object_streams() -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.5");
    pdf.object_stream(10, &[(1, CATALOG), (2, PAGES), (3, PAGE)]).stream(4, "/Filter /FlateDecode", &deflate(CONTENT));
    let start = pdf.xref_stream(
        11,
        &[
            (0, Row::Free(255)),
            (1, Row::Compressed(10, 0)),
            (2, Row::Compressed(10, 1)),
            (3, Row::Compressed(10, 2)),
            (4, Row::InUse(4)),
            (10, Row::InUse(10)),
            (11, Row::InUse(11)),
        ],
        "/Size 12 /Root 1 0 R",
    );
    pdf.startxref(start).finish()
}

pub fn hybrid() -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.5");
    pdf.object(1, 0, CATALOG).object_stream(10, &[(2, PAGES), (3, PAGE)]).stream(4, "", CONTENT);
    let hidden = pdf.xref_stream(11, &[(2, Row::Compressed(10, 0)), (3, Row::Compressed(10, 1))], "/Size 12");
    let table = pdf.xref_table(
        &[
            (0, Row::Free(65535)),
            (1, Row::InUse(1)),
            (2, Row::Free(0)),
            (3, Row::Free(0)),
            (4, Row::InUse(4)),
            (10, Row::InUse(10)),
        ],
        &format!("/Size 12 /Root 1 0 R /XRefStm {hidden}"),
    );
    pdf.startxref(table).finish()
}

pub fn table_then_stream_update() -> Vec<u8> {
    let mut pdf = single_page();
    let first = pdf.xref_table(&single_page_rows(), "/Size 5 /Root 1 0 R");
    pdf.startxref(first);
    pdf.object_stream(5, &[(6, "(compressed in the update)")]).object(
        1,
        0,
        "<< /Type /Catalog /Pages 2 0 R /Note 6 0 R >>",
    );
    let second = pdf.xref_stream(
        7,
        &[(1, Row::InUse(1)), (5, Row::InUse(5)), (6, Row::Compressed(5, 0)), (7, Row::InUse(7))],
        &format!("/Size 8 /Root 1 0 R /Prev {first}"),
    );
    pdf.startxref(second).finish()
}

pub fn one_wrong_offset() -> Vec<u8> {
    let mut pdf = single_page();
    let page = pdf.offset_of(3);
    let start = pdf.xref_table(&single_page_rows(), "/Size 5 /Root 1 0 R");
    pdf.startxref(start);
    let correct = format!("{page:010} 00000 n").into_bytes();
    let wrong = format!("{:010} 00000 n", page + 7).into_bytes();
    let mut bytes = pdf.finish();
    let at = bytes.windows(correct.len()).position(|window| window == correct).unwrap();
    bytes[at..at + wrong.len()].copy_from_slice(&wrong);
    bytes
}

pub fn without_xref() -> Vec<u8> {
    let mut pdf = single_page();
    pdf.raw(b"trailer\n<< /Size 5 /Root 1 0 R >>\n%%EOF\n").finish()
}

pub fn indirect_length() -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.4");
    pdf.object(1, 0, CATALOG).object(2, 0, PAGES).object(3, 0, PAGE);
    pdf.mark(4).raw(b"4 0 obj\n<< /Length 5 0 R >>\nstream\n").raw(CONTENT).raw(b"endstream\nendobj\n").object(
        5,
        0,
        &CONTENT.len().to_string(),
    );
    let mut rows = single_page_rows();
    rows.push((5, Row::InUse(5)));
    let start = pdf.xref_table(&rows, "/Size 6 /Root 1 0 R");
    pdf.startxref(start).finish()
}

pub fn nested_page_tree() -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.4");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 3 /MediaBox [0 0 500 700] /Resources 10 0 R /Rotate 90 >>")
        .object(3, 0, "<< /Type /Pages /Parent 2 0 R /Kids [4 0 R 5 0 R] /Count 2 /CropBox [10 10 490 690] /Rotate -90 >>")
        .object(4, 0, "<< /Type /Page /Parent 3 0 R /Contents 7 0 R >>")
        .object(5, 0, "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 300 300] /Rotate 180 /Resources << /Font << >> >> /Contents [7 0 R 8 0 R] >>")
        .object(6, 0, "<< /Type /Page /Parent 2 0 R >>")
        .stream(7, "", b"q 1 0 0 RG 0 0 m 100 100 l S Q")
        .stream(8, "", b"BT /F1 9 Tf (second segment) Tj ET")
        .object(10, 0, "<< /ProcSet [/PDF /Text] >>");
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain((1..=8).map(|num| (num, Row::InUse(num))))
        .chain([(9, Row::Free(0)), (10, Row::InUse(10))])
        .collect();
    let start = pdf.xref_table(&rows, "/Size 11 /Root 1 0 R");
    pdf.startxref(start).finish()
}

pub fn shared_page_with_cycle() -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.4");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R 3 0 R 4 0 R] /Count 2 >>")
        .object(3, 0, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 5 0 R >>")
        .object(4, 0, "<< /Type /Pages /Parent 2 0 R /Kids [2 0 R] /Count 0 >>")
        .stream(5, "", b"0 0 50 50 re f");
    let rows: Vec<(u32, Row)> =
        std::iter::once((0, Row::Free(65535))).chain((1..=5).map(|num| (num, Row::InUse(num)))).collect();
    let start = pdf.xref_table(&rows, "/Size 6 /Root 1 0 R");
    pdf.startxref(start).finish()
}

const GRAPHICS: &[u8] = b"\
0.8 0.1 0.1 rg 20 320 60 60 re f
0 0 1 0 k 90 320 60 60 re f
0.35 g 160 320 60 60 re f
/Icc cs 0.1 0.6 0.3 sc 230 320 60 60 re f
/Palette cs 1 sc 300 320 60 60 re f
/Spot cs 0.7 scn 20 250 60 60 re f
/Duo cs 0.2 0.9 scn 90 250 60 60 re f
q /Half gs 0 0 1 rg 130 230 80 80 re f Q
0 0 0 RG 4 w 1 J 1 j 230 260 m 260 300 l 290 260 l S
2 w 0 J 0 j [6 3] 0 d 300 250 m 380 300 l S [] 0 d
1 0 0 RG 0.5 w 20 200 m 60 240 100 160 140 200 c S
0 0.5 0 rg 160 160 m 200 240 l 240 160 l 150 210 l 250 210 l h f*
q 270 160 80 40 re W n 0 0.6 0.9 rg 250 140 120 120 re f Q
q 1 0 0 1 20 20 cm /Badge Do Q
";

const TEXT: &[u8] = b"\
BT /F1 18 Tf 20 360 Td (Helvetica substitute) Tj ET
BT /F2 14 Tf 20 335 Td (Times bold) Tj 0 -20 Td /F3 12 Tf (Courier mono) Tj ET
BT /F1 12 Tf 20 280 Td 3 Tc 10 Tw (spaced out words) Tj ET
BT /F1 12 Tf 20 260 Td 0 Tc 0 Tw 150 Tz (wide) Tj 100 Tz 5 Ts (raised) Tj 0 Ts ET
BT /F1 12 Tf 20 240 Td [(Ker) -400 (ning)] TJ ET
BT /F4 14 Tf 20 215 Td (\\311t\\351 \\374ber) Tj ET
BT 1 Tr 0 0 1 RG 0.5 w /F1 24 Tf 20 180 Td (Outline) Tj 3 Tr (Hidden) Tj ET
q BT 7 Tr /F1 30 Tf 200 180 Td (CLIP) Tj ET 0.9 0.4 0 rg 190 170 120 40 re f Q
BT /T3 10 Tf 20 120 Td (abab) Tj ET
";

const SQUARES: &[u8] = b"1 0 0 rg 10 10 30 20 re f 0 0 1 rg 60 40 20 30 re f 0 g 0 0 100 2 re f";

pub fn render_features() -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.6");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R 6 0 R] /Count 4 >>")
        .object(3, 0, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 10 0 R /Resources 20 0 R >>")
        .object(4, 0, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 11 0 R /Resources 21 0 R >>")
        .object(5, 0, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 80] /Contents 12 0 R >>")
        .object(6, 0, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 80] /Rotate 90 /Contents 12 0 R >>")
        .stream(10, "", GRAPHICS)
        .stream(11, "", TEXT)
        .stream(12, "", SQUARES)
        .object(
            20,
            0,
            "<< /ColorSpace << /Icc [/ICCBased 30 0 R] /Palette [/Indexed /DeviceRGB 1 <ff000000ff00>] \
             /Spot [/Separation /Gold /DeviceCMYK 31 0 R] /Duo [/DeviceN [/A /B] /DeviceRGB 32 0 R] >> \
             /ExtGState << /Half << /ca 0.5 >> >> /XObject << /Badge 33 0 R >> >>",
        )
        .object(
            21,
            0,
            "<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> \
             /F2 << /Type /Font /Subtype /Type1 /BaseFont /Times-Bold >> \
             /F3 << /Type /Font /Subtype /Type1 /BaseFont /Courier >> \
             /F4 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >> \
             /T3 40 0 R >> >>",
        )
        .stream(30, "/N 3", b"synthetic profile")
        .object(31, 0, "<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0.2 0.9 0.1] /N 1 >>")
        .stream(32, "/FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1]", b"{ 2 copy add 2 div }")
        .stream(
            33,
            "/Type /XObject /Subtype /Form /BBox [0 0 50 50] /Matrix [0.5 0 0 0.5 0 0]",
            b"0.5 0 0.5 rg 0 0 100 100 re f 1 g 10 10 30 30 re f",
        )
        .object(
            40,
            0,
            "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] /FontMatrix [0.001 0 0 0.001 0 0] \
             /CharProcs << /square 41 0 R /triangle 42 0 R >> /Encoding << /Differences [97 /square /triangle] >> \
             /FirstChar 97 /LastChar 98 /Widths [1100 1100] >>",
        )
        .stream(41, "", b"1000 0 d0 0 0 1000 1000 re f")
        .stream(42, "", b"1000 0 d0 0 0 m 1000 0 l 500 1000 l f");
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain([1, 2, 3, 4, 5, 6, 10, 11, 12, 20, 21, 30, 31, 32, 33, 40, 41, 42].map(|num| (num, Row::InUse(num))))
        .collect();
    let start = pdf.xref_table(&rows, "/Size 43 /Root 1 0 R");
    pdf.startxref(start).finish()
}

const JPEG_RGB: &[u8] = include_bytes!("../../fixtures/images/gradient-rgb.jpg");
const JPEG_GRAY: &[u8] = include_bytes!("../../fixtures/images/gradient-gray.jpg");
const JPEG_CMYK: &[u8] = include_bytes!("../../fixtures/images/gradient-cmyk.jpg");

pub fn image_features() -> Vec<u8> {
    let mut content =
        b"q 120 0 0 80 20 200 cm /Rgb Do Q q 120 0 0 80 150 200 cm /Gray Do Q q 120 0 0 80 280 200 cm /Cmyk Do Q \
        q 100 0 0 60 20 110 cm /Masked Do Q q 100 0 0 60 140 110 cm /Keyed Do Q \
        q 1 0 0 rg 100 0 0 60 260 110 cm BI /W 8 /H 2 /IM true ID "
            .to_vec();
    content.extend_from_slice(&[0b0011_1100, 0b1100_0011]);
    content.extend_from_slice(b" EI Q q 60 0 0 40 20 40 cm BI /W 2 /H 2 /CS /RGB /BPC 8 ID ");
    content.extend_from_slice(&[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0]);
    content.extend_from_slice(
        b" EI Q q 60 0 0 40 100 40 cm /Palette Do Q q 60 0 0 40 180 40 cm /Jpx Do Q q 60 0 0 40 260 40 cm /Stenciled Do Q",
    );
    let mut pdf = SyntheticPdf::new("1.5");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
        .object(
            3,
            0,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 300] /Contents 4 0 R /Resources << /XObject << \
             /Rgb 10 0 R /Gray 11 0 R /Cmyk 12 0 R /Masked 13 0 R /Keyed 15 0 R /Palette 16 0 R /Jpx 17 0 R /Stenciled 18 0 R >> >> >>",
        )
        .stream(4, "", &content)
        .stream(10, "/Type /XObject /Subtype /Image /Width 48 /Height 32 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode", JPEG_RGB)
        .stream(11, "/Type /XObject /Subtype /Image /Width 48 /Height 32 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /DCTDecode", JPEG_GRAY)
        .stream(12, "/Type /XObject /Subtype /Image /Width 48 /Height 32 /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Decode [1 0 1 0 1 0 1 0] /Filter /DCTDecode", JPEG_CMYK)
        .stream(13, "/Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 14 0 R", &[0, 0, 200].repeat(16))
        .stream(14, "/Type /XObject /Subtype /Image /Width 4 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8", &[0, 85, 170, 255])
        .stream(15, "/Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask [0 10 240 255 0 10]", &[255, 0, 0, 0, 255, 0])
        .stream(
            16,
            "/Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace [/Indexed /DeviceRGB 1 <ff00000000ff>] /BitsPerComponent 1",
            &[0b1010_0000, 0b0101_0000, 0b1010_0000, 0b0101_0000],
        )
        .stream(17, "/Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /JPXDecode", b"not a codestream")
        .stream(18, "/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask 19 0 R", &[255, 140, 0].repeat(4))
        .stream(19, "/Type /XObject /Subtype /Image /Width 2 /Height 2 /ImageMask true", &[0b0100_0000, 0b1000_0000]);
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain([1, 2, 3, 4].map(|num| (num, Row::InUse(num))))
        .chain((10..=19).map(|num| (num, Row::InUse(num))))
        .collect();
    let start = pdf.xref_table(&rows, "/Size 20 /Root 1 0 R");
    pdf.startxref(start).finish()
}

fn mesh_bytes(values: &[u32], bits: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut accumulator = 0u64;
    let mut filled = 0;
    for &value in values {
        accumulator = accumulator << bits | value as u64;
        filled += bits;
        while filled >= 8 {
            filled -= 8;
            bytes.push((accumulator >> filled) as u8);
        }
    }
    if filled > 0 {
        bytes.push((accumulator << (8 - filled)) as u8);
    }
    bytes
}

pub fn shading_features() -> Vec<u8> {
    let content = b"q 20 300 160 80 re W n /Axial sh Q \
        /Pattern cs /Glow scn 290 340 m 290 368 268 390 240 390 c 212 390 190 368 190 340 c 190 312 212 290 240 290 c 268 290 290 312 290 340 c f \
        q 300 300 80 80 re W n /Plane sh Q \
        /Gouraud sh /Coons sh \
        /Pattern cs /Stripes scn 20 20 160 100 re f \
        /Spots cs 1 0 0 /Dots scn 200 20 80 100 re f 0 0.6 0 /Dots scn 290 20 90 100 re f \
        /Pattern CS /Rainbow SCN 12 w 1 J 30 150 m 370 150 l S";
    let gouraud = mesh_bytes(&[0, 20, 180, 255, 0, 0, 0, 160, 180, 0, 255, 0, 0, 90, 280, 0, 0, 255], 8);
    let coons_points: [(u32, u32); 12] = [
        (200, 180),
        (200, 205),
        (200, 230),
        (200, 255),
        (230, 255),
        (260, 255),
        (290, 255),
        (290, 230),
        (290, 205),
        (290, 180),
        (260, 180),
        (230, 180),
    ];
    let mut coons = vec![0u32];
    coons.extend(coons_points.iter().flat_map(|&(x, y)| [x, y]));
    coons.extend([255, 0, 0, 255, 255, 0, 0, 0, 255, 0, 255, 0]);
    let coons = mesh_bytes(&coons, 8);
    let mut pdf = SyntheticPdf::new("1.6");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
        .object(
            3,
            0,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 4 0 R /Resources << \
             /Shading << /Axial 10 0 R /Plane 12 0 R /Gouraud 14 0 R /Coons 15 0 R >> \
             /Pattern << /Glow 16 0 R /Stripes 17 0 R /Dots 18 0 R /Rainbow 19 0 R >> \
             /ColorSpace << /Spots [/Pattern /DeviceRGB] >> >> >>",
        )
        .stream(4, "", content)
        .object(10, 0, "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [40 0 160 0] /Function 11 0 R /Extend [true true] >>")
        .object(11, 0, "<< /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >>")
        .object(12, 0, "<< /ShadingType 1 /ColorSpace /DeviceRGB /Domain [300 380 300 380] /Function 13 0 R >>")
        .stream(13, "/FunctionType 4 /Domain [300 380 300 380] /Range [0 1 0 1 0 1]", b"{ 300 sub 80 div exch 300 sub 80 div exch 0.5 }")
        .stream(14, "/ShadingType 4 /ColorSpace /DeviceRGB /BitsPerCoordinate 8 /BitsPerComponent 8 /BitsPerFlag 8 /Decode [0 255 0 255 0 1 0 1 0 1]", &gouraud)
        .stream(15, "/ShadingType 6 /ColorSpace /DeviceRGB /BitsPerCoordinate 8 /BitsPerComponent 8 /BitsPerFlag 8 /Decode [0 255 0 255 0 1 0 1 0 1]", &coons)
        .object(16, 0, "<< /PatternType 2 /Shading << /ShadingType 3 /ColorSpace /DeviceRGB /Coords [240 340 0 240 340 50] /Function << /FunctionType 2 /Domain [0 1] /C0 [1 1 0] /C1 [0.8 0 0.4] /N 1 >> >> >>")
        .stream(17, "/PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 20 20] /XStep 20 /YStep 20 /Resources << >>", b"0 0 0.8 rg 0 0 10 20 re f 1 0.8 0 rg 10 0 10 20 re f")
        .stream(18, "/PatternType 1 /PaintType 2 /TilingType 1 /BBox [0 0 16 16] /XStep 16 /YStep 16 /Resources << >>", b"0 0 1 rg 8 8 m 13 8 l 13 13 l 8 13 l h f")
        .object(19, 0, "<< /PatternType 2 /Shading << /ShadingType 2 /ColorSpace /DeviceRGB /Coords [30 0 370 0] /Extend [true true] /Function << /FunctionType 2 /Domain [0 1] /C0 [0 0.7 0.3] /C1 [0.9 0.1 0.6] /N 1 >> >> >>");
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain([1, 2, 3, 4].map(|num| (num, Row::InUse(num))))
        .chain((10..=19).map(|num| (num, Row::InUse(num))))
        .collect();
    let start = pdf.xref_table(&rows, "/Size 20 /Root 1 0 R");
    pdf.startxref(start).finish()
}

pub fn annotation_features() -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.6");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
        .object(3, 0, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Contents 4 0 R /Annots [10 0 R 11 0 R 14 0 R 16 0 R 18 0 R 20 0 R] >>")
        .stream(4, "", b"0.9 g 0 0 300 200 re f")
        .object(10, 0, "<< /Type /Annot /Subtype /Square /Rect [20 20 120 80] /AP << /N 21 0 R >> >>")
        .object(11, 0, "<< /Type /Annot /Subtype /Widget /FT /Btn /Rect [150 20 180 50] /AS /On /AP << /N << /On 12 0 R /Off 13 0 R >> >> >>")
        .stream(12, "/Type /XObject /Subtype /Form /BBox [0 0 30 30]", b"0 0 0 RG 1 w 0.5 0.5 29 29 re S 0 0.5 0 rg 6 6 18 18 re f")
        .stream(13, "/Type /XObject /Subtype /Form /BBox [0 0 30 30]", b"0 0 0 RG 1 w 0.5 0.5 29 29 re S")
        .object(14, 0, "<< /Type /Annot /Subtype /Square /F 2 /Rect [200 20 280 80] /AP << /N 15 0 R >> >>")
        .stream(15, "/Type /XObject /Subtype /Form /BBox [0 0 80 60]", b"0 g 0 0 80 60 re f")
        .object(16, 0, "<< /Type /Annot /Subtype /Stamp /Rect [20 100 180 140] /AP << /N 17 0 R >> >>")
        .stream(17, "/Type /XObject /Subtype /Form /BBox [0 0 40 160] /Matrix [0 1 -1 0 0 0]", b"0 0 1 rg 0 0 40 160 re f 1 g 5 5 10 10 re f")
        .object(18, 0, "<< /Type /Annot /Subtype /Text /F 32 /Rect [200 100 280 180] /AP << /N 15 0 R >> >>")
        .object(20, 0, "<< /Type /Annot /Subtype /Popup /Rect [200 100 280 180] /AP << /N 15 0 R >> >>")
        .stream(21, "/Type /XObject /Subtype /Form /BBox [0 0 100 60]", b"1 0 0 RG 3 w 1.5 1.5 97 57 re S");
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain([1, 2, 3, 4].map(|num| (num, Row::InUse(num))))
        .chain((10..=18).map(|num| (num, Row::InUse(num))))
        .chain([(19, Row::Free(0)), (20, Row::InUse(20)), (21, Row::InUse(21))])
        .collect();
    let start = pdf.xref_table(&rows, "/Size 22 /Root 1 0 R");
    pdf.startxref(start).finish()
}

const EDITING_FIRST: &[u8] = b"\
BT /F1 12 Tf 20 360 Td (Plain line to edit) Tj ET
BT /F1 12 Tf 80 Tz 20 330 Td (Condensed line) Tj 100 Tz ET
q 0.866 0.5 -0.5 0.866 200 200 cm BT /F1 12 Tf 0 0 Td (Rotated line) Tj ET Q
q 20 280 60 20 re W n BT /F1 12 Tf 20 285 Td (Clipped words here) Tj ET Q
/Span << /ActualText (Actual) >> BDC BT /F1 12 Tf 20 250 Td (Marked text) Tj ET EMC
BT 3 Tr /F1 12 Tf 20 220 Td (Invisible OCR layer) Tj ET
";

const EDITING_SECOND: &[u8] = b"\
BT /F1 12 Tf 20 190 Td (Second stream line) Tj ET
BT /F1 12 Tf 20 160 Td [(Ker) -200 (ned) 300 ( runs)] TJ ET
BT /F1 12 Tf 20 130 Td (First) Tj ( continued) Tj ET
";

pub fn editing_features() -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.6");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /Resources << /Font << /F1 5 0 R >> >> /MediaBox [0 0 400 400] >>")
        .object(3, 0, "<< /Type /Page /Parent 2 0 R /Contents [10 0 R 11 0 R] >>")
        .object(4, 0, "<< /Type /Page /Parent 2 0 R /Contents 10 0 R >>")
        .object(5, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
        .stream(10, "", EDITING_FIRST)
        .stream(11, "", EDITING_SECOND);
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain([1, 2, 3, 4, 5].map(|num| (num, Row::InUse(num))))
        .chain((6..=9).map(|num| (num, Row::Free(0))))
        .chain([10, 11].map(|num| (num, Row::InUse(num))))
        .collect();
    let start = pdf.xref_table(&rows, "/Size 12 /Root 1 0 R");
    pdf.startxref(start).finish()
}

const REFLOW_CONTENT: &[u8] = b"\
BT /F1 12 Tf 14 TL 20 360 Td (Lines of one block wrap at the) Tj T* (widest of them and ) Tj /F2 12 Tf (flow) Tj
/F1 12 Tf ( on when) Tj T* (text grows.) Tj 0 -40 Td (Positioned after the block) Tj ET
";

pub fn reflow_features() -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.6");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 400 400] >>")
        .object(
            3,
            0,
            "<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents 6 0 R >>",
        )
        .object(4, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
        .object(5, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>")
        .stream(6, "", REFLOW_CONTENT);
    let rows: Vec<(u32, Row)> =
        std::iter::once((0, Row::Free(65535))).chain((1..=6).map(|num| (num, Row::InUse(num)))).collect();
    let start = pdf.xref_table(&rows, "/Size 7 /Root 1 0 R");
    pdf.startxref(start).finish()
}

pub fn embedded_font_names() -> Vec<u8> {
    let program = include_bytes!("../../../../../public/fonts/LiberationSans-Bold.ttf");
    let mut pdf = SyntheticPdf::new("1.6");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 400 400] >>")
        .object(3, 0, "<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R /F2 5 0 R /F3 9 0 R >> >> /Contents 6 0 R >>")
        .object(4, 0, "<< /Type /Font /Subtype /TrueType /BaseFont /AAAAAA+false /FirstChar 32 /LastChar 126 /FontDescriptor 7 0 R /Encoding /WinAnsiEncoding >>")
        .object(5, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /BAAAAA+Helvetica /Encoding /WinAnsiEncoding >>")
        .stream(6, "", b"BT /F1 12 Tf 20 300 Td (Bold) Tj /F2 12 Tf 0 -20 Td (Plain) Tj ET")
        .object(7, 0, "<< /Type /FontDescriptor /FontName /AAAAAA+false /Flags 32 /FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 900 /Descent -200 /CapHeight 700 /StemV 80 /FontFile2 8 0 R >>")
        .stream(8, "/Filter /FlateDecode", &deflate(program))
        .object(9, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /CAAAAA+false /Encoding /WinAnsiEncoding >>");
    let rows: Vec<(u32, Row)> =
        std::iter::once((0, Row::Free(65535))).chain((1..=9).map(|num| (num, Row::InUse(num)))).collect();
    let start = pdf.xref_table(&rows, "/Size 10 /Root 1 0 R");
    pdf.startxref(start).finish()
}

const FORM_FONT: &str = "/Resources << /Font << /F1 5 0 R >> >>";

pub fn form_features() -> Vec<u8> {
    let font = "/Font << /F1 5 0 R >>";
    let mut pdf = SyntheticPdf::new("1.6");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R 4 0 R 12 0 R] /Count 3 /MediaBox [0 0 400 400] >>")
        .object(
            3,
            0,
            &format!("<< /Type /Page /Parent 2 0 R /Resources << {font} /XObject << /Shared 6 0 R /Outer 7 0 R >> >> /Contents 10 0 R >>"),
        )
        .object(4, 0, &format!("<< /Type /Page /Parent 2 0 R /Resources << {font} /XObject << /Shared 6 0 R >> >> /Contents 11 0 R >>"))
        .object(5, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
        .stream(
            6,
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 300 100] /Matrix [1 0 0 1 20 250] {FORM_FONT}"),
            b"BT /F1 12 Tf 0 60 Td (Shared form text) Tj ET",
        )
        .stream(
            7,
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 300 100] /Matrix [1 0 0 1 20 100] /Resources << {font} /XObject << /Inner 8 0 R >> >>"),
            b"BT /F1 12 Tf 0 50 Td (Outer form line) Tj ET /Inner Do",
        )
        .stream(
            8,
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 300 100] /Matrix [1 0 0 1 0 5] {FORM_FONT}"),
            b"BT /F1 12 Tf 0 20 Td (Inner form line) Tj ET",
        )
        .stream(10, "", b"BT /F1 12 Tf 20 360 Td (Page line) Tj 0 -36 Td (Page line close above the form) Tj ET /Shared Do /Outer Do")
        .stream(11, "", b"/Shared Do")
        .object(12, 0, &format!("<< /Type /Page /Parent 2 0 R /Resources << {font} /XObject << /Twice 13 0 R >> >> /Contents 14 0 R >>"))
        .stream(
            13,
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 300 100] {FORM_FONT}"),
            b"BT /F1 12 Tf 20 300 Td (Drawn twice) Tj ET",
        )
        .stream(14, "", b"/Twice Do q 1 0 0 1 0 -100 cm /Twice Do Q");
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain((1..=8).map(|num| (num, Row::InUse(num))))
        .chain(std::iter::once((9, Row::Free(0))))
        .chain((10..=14).map(|num| (num, Row::InUse(num))))
        .collect();
    let start = pdf.xref_table(&rows, "/Size 15 /Root 1 0 R");
    pdf.startxref(start).finish()
}

pub fn organize_features() -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.6");
    pdf.object(
        1,
        0,
        "<< /Type /Catalog /Pages 2 0 R /Outlines 20 0 R /Dests << /two [4 0 R /Fit] /three [5 0 R /Fit] >> \
         /Names << /Dests << /Names [(p2) [4 0 R /Fit] (p3) [5 0 R /Fit]] >> >> /AcroForm << /Fields [30 0 R] >> >>",
    )
    .object(2, 0, "<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 3 /MediaBox [0 0 300 300] /Resources << /Font << /F1 7 0 R >> >> >>")
    .object(3, 0, "<< /Type /Page /Parent 2 0 R /Contents 10 0 R /Annots [31 0 R] >>")
    .object(4, 0, "<< /Type /Page /Parent 6 0 R /Contents 11 0 R /Annots [30 0 R] >>")
    .object(5, 0, "<< /Type /Page /Parent 6 0 R /Contents 12 0 R /MediaBox [0 0 200 300] >>")
    .object(6, 0, "<< /Type /Pages /Parent 2 0 R /Kids [4 0 R 5 0 R] /Count 2 /Rotate 90 >>")
    .object(7, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
    .stream(10, "", b"BT /F1 18 Tf 30 250 Td (Page one) Tj ET")
    .stream(11, "", b"BT /F1 18 Tf 30 250 Td (Page two) Tj ET")
    .stream(12, "", b"BT /F1 18 Tf 30 250 Td (Page three) Tj ET")
    .object(20, 0, "<< /Type /Outlines /First 21 0 R /Last 22 0 R /Count 4 >>")
    .object(21, 0, "<< /Title (Chapter two) /Parent 20 0 R /Next 22 0 R /Dest [4 0 R /Fit] >>")
    .object(22, 0, "<< /Title (Part) /Parent 20 0 R /Prev 21 0 R /First 23 0 R /Last 24 0 R /Count 2 /A << /S /GoTo /D [4 0 R /Fit] >> >>")
    .object(23, 0, "<< /Title (Section two) /Parent 22 0 R /Next 24 0 R /Dest /two >>")
    .object(24, 0, "<< /Title (Section three) /Parent 22 0 R /Prev 23 0 R /Dest (p3) >>")
    .object(30, 0, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /Rect [20 20 120 40] /P 4 0 R >>")
    .object(31, 0, "<< /Type /Annot /Subtype /Link /Rect [20 240 120 270] /Dest [4 0 R /Fit] >>");
    let used = [1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 20, 21, 22, 23, 24, 30, 31];
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain((1..=31).map(|num| (num, if used.contains(&num) { Row::InUse(num) } else { Row::Free(0) })))
        .collect();
    let start = pdf.xref_table(&rows, "/Size 32 /Root 1 0 R");
    pdf.startxref(start).finish()
}

const FORM_LABELS: &[u8] =
    b"BT /F1 10 Tf 20 345 Td (First name) Tj 0 -30 Td (Last name) Tj 0 -60 Td (Notes) Tj 0 -60 Td (Zip) Tj \
0 -30 Td (I agree) Tj 0 -30 Td (Color) Tj 0 -30 Td (Language) Tj 0 -60 Td (Fruits) Tj ET";
const CHECK_ON: &[u8] = b"0 G 1 w 0.5 0.5 15 15 re S 3 8 m 6 4 l 13 13 l S";
const CHECK_OFF: &[u8] = b"0 G 1 w 0.5 0.5 15 15 re S";
const RADIO_ON: &[u8] = b"0 G 1 w 7 0.5 m 13 0.5 13 13.5 7 13.5 c 1 13.5 1 0.5 7 0.5 c S 0 g 7 4 m 9.5 4 9.5 10 7 10 c 4.5 10 4.5 4 7 4 c f";
const RADIO_OFF: &[u8] = b"0 G 1 w 7 0.5 m 13 0.5 13 13.5 7 13.5 c 1 13.5 1 0.5 7 0.5 c S";

pub fn fillable_form() -> Vec<u8> {
    let widget = |rect: &str| format!("/Type /Annot /Subtype /Widget /Rect [{rect}] /P 4 0 R /F 4");
    let mut pdf = SyntheticPdf::new("1.7");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R /AcroForm 3 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [4 0 R] /Count 1 >>")
        .object(3, 0, "<< /Fields [10 0 R 20 0 R 30 0 R 40 0 R 50 0 R 60 0 R 70 0 R 80 0 R] /DR << /Font << /Helv 5 0 R >> >> /DA (/Helv 0 Tf 0 g) /NeedAppearances true >>")
        .object(
            4,
            0,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 6 0 R /Resources << /Font << /F1 5 0 R >> >> \
             /Annots [11 0 R 12 0 R 20 0 R 30 0 R 40 0 R 51 0 R 52 0 R 60 0 R 70 0 R 80 0 R] >>",
        )
        .object(5, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
        .stream(6, "", FORM_LABELS)
        .object(10, 0, "<< /T (person) /Kids [11 0 R 12 0 R] >>")
        .object(11, 0, &format!("<< /T (first) /FT /Tx /Parent 10 0 R {} /DA (/Helv 12 Tf 0 0 0.6 rg) /V (Ada) /MK << /BC [0 0 0] >> >>", widget("120 338 300 358")))
        .object(12, 0, &format!("<< /T (last) /FT /Tx /Parent 10 0 R {} /Q 2 /MK << /BC [0 0 0] /BG [0.95 0.95 1] >> >>", widget("120 308 300 328")))
        .object(20, 0, &format!("<< /T (notes) /FT /Tx /Ff 4096 {} /MK << /BC [0.5 0.5 0.5] >> >>", widget("120 228 300 298")))
        .object(30, 0, &format!("<< /T (zip) /FT /Tx /Ff 16777216 /MaxLen 5 {} /MK << /BC [0 0 0] >> >>", widget("120 188 220 208")))
        .object(40, 0, &format!("<< /T (agree) /FT /Btn {} /AP << /N << /Yes 41 0 R /Off 42 0 R >> >> /AS /Off /V /Off >>", widget("120 160 136 176")))
        .stream(41, "/Type /XObject /Subtype /Form /BBox [0 0 16 16]", CHECK_ON)
        .stream(42, "/Type /XObject /Subtype /Form /BBox [0 0 16 16]", CHECK_OFF)
        .object(50, 0, "<< /T (color) /FT /Btn /Ff 49152 /Kids [51 0 R 52 0 R] /V /Red >>")
        .object(51, 0, &format!("<< {} /Parent 50 0 R /AS /Red /AP << /N << /Red 53 0 R /Off 54 0 R >> >> >>", widget("120 130 134 144")))
        .object(52, 0, &format!("<< {} /Parent 50 0 R /AS /Off /AP << /N << /Blue 55 0 R /Off 54 0 R >> >> >>", widget("160 130 174 144")))
        .stream(53, "/Type /XObject /Subtype /Form /BBox [0 0 14 14]", RADIO_ON)
        .stream(54, "/Type /XObject /Subtype /Form /BBox [0 0 14 14]", RADIO_OFF)
        .stream(55, "/Type /XObject /Subtype /Form /BBox [0 0 14 14]", RADIO_ON)
        .object(60, 0, &format!("<< /T (language) /FT /Ch /Ff 131072 /Opt [[(en) (English)] [(de) (Deutsch)]] /V (en) {} /MK << /BC [0 0 0] >> >>", widget("120 98 250 116")))
        .object(70, 0, &format!("<< /T (fruits) /FT /Ch /Opt [(Apple) (Banana) (Cherry)] /V [(Banana)] /Ff 2097152 {} /MK << /BC [0 0 0] >> >>", widget("120 28 250 88")))
        .object(80, 0, &format!("<< /T (id) /FT /Tx /Ff 1 /V (A-17) {} >>", widget("300 28 380 44")));
    let used = [1, 2, 3, 4, 5, 6, 10, 11, 12, 20, 30, 40, 41, 42, 50, 51, 52, 53, 54, 55, 60, 70, 80];
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain((1..=80).map(|num| (num, if used.contains(&num) { Row::InUse(num) } else { Row::Free(0) })))
        .collect();
    let start = pdf.xref_table(&rows, "/Size 81 /Root 1 0 R");
    pdf.startxref(start).finish()
}

fn gradient(width: usize, height: usize) -> Vec<u8> {
    (0..width * height)
        .flat_map(|index| {
            let (x, y) = (index % width, index / width);
            [(40 + x * 25) as u8, (60 + y * 20) as u8, 200]
        })
        .collect()
}

pub fn redaction_features() -> Vec<u8> {
    let mut content = b"BT /F1 12 Tf 20 360 Td (Account number 4711 0815) Tj ET\n\
BT /F1 12 Tf 20 330 Td (Public line stays) Tj ET\n\
/Span << /ActualText (SECRET-ALT) >> BDC BT /F1 12 Tf 20 60 Td (Marked secret) Tj ET EMC\n\
q BT 3 Tr /F1 12 Tf 20 80 Td (Hidden secret code) Tj ET Q\n\
q 80 0 0 80 220 180 cm /Img Do Q\n\
/Panel Do\n\
q 0 0 1 RG 2 w 250 90 m 350 120 l S Q\n\
q 1 0 0 rg 330 130 40 30 re f Q\n\
q 12 w 100 141 m 160 141 l S Q\n\
q 200 60 40 40 re W n 0 1 0 rg 150 20 200 200 re f Q\n\
q 40 0 0 40 300 60 cm BI /W 4 /H 4 /CS /G /BPC 8 ID "
        .to_vec();
    content.extend([0x80u8; 16]);
    content.extend(b" EI Q\n");
    let mut pdf = SyntheticPdf::new("1.7");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R /Outlines 30 0 R /AcroForm << /Fields [42 0 R] >> >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
        .object(
            3,
            0,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> /XObject << /Img 6 0 R /Panel 7 0 R >> >> \
             /Annots [40 0 R 43 0 R 41 0 R 42 0 R 50 0 R 51 0 R 52 0 R] >>",
        )
        .stream(4, "", &content)
        .object(5, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
        .stream(6, "/Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceRGB /BitsPerComponent 8", &gradient(8, 8))
        .stream(7, "/Type /XObject /Subtype /Form /BBox [0 0 200 60] /Matrix [1 0 0 1 20 100] /Resources << /Font << /F1 5 0 R >> >>", b"BT /F1 12 Tf 0 30 Td (Form secret) Tj ET")
        .object(30, 0, "<< /Type /Outlines /First 31 0 R /Last 32 0 R /Count 2 >>")
        .object(31, 0, "<< /Title (Account number 4711 0815) /Parent 30 0 R /Next 32 0 R /Dest [3 0 R /Fit] >>")
        .object(32, 0, "<< /Title (Public section) /Parent 30 0 R /Prev 31 0 R /Dest [3 0 R /Fit] >>")
        .object(40, 0, "<< /Type /Annot /Subtype /Text /Rect [150 110 170 130] /Contents (Note on the secret) /Popup 43 0 R >>")
        .object(41, 0, "<< /Type /Annot /Subtype /Link /Rect [20 325 120 345] /Dest [3 0 R /Fit] >>")
        .object(42, 0, "<< /Type /Annot /Subtype /Widget /FT /Tx /T (secretfield) /V (top secret) /Rect [180 105 260 125] /P 3 0 R >>")
        .object(43, 0, "<< /Type /Annot /Subtype /Popup /Rect [300 300 380 340] /Parent 40 0 R >>")
        .object(50, 0, "<< /Type /Annot /Subtype /Redact /Rect [109 352 220 374] /IC [0 0 0] >>")
        .object(51, 0, "<< /Type /Annot /Subtype /Redact /Rect [260 170 320 270] /IC [0 0 0] \
             /QuadPoints [260 270 320 270 260 170 320 170] >>")
        .object(52, 0, "<< /Type /Annot /Subtype /Redact /Rect [10 50 400 145] /IC [0.2 0.2 0.2] >>");
    let used = [1, 2, 3, 4, 5, 6, 7, 30, 31, 32, 40, 41, 42, 43, 50, 51, 52];
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain((1..=52).map(|num| (num, if used.contains(&num) { Row::InUse(num) } else { Row::Free(0) })))
        .collect();
    let start = pdf.xref_table(&rows, "/Size 53 /Root 1 0 R");
    pdf.startxref(start).finish()
}

pub fn image_placement_features() -> Vec<u8> {
    let mut content = b"BT /F1 12 Tf 20 370 Td (Caption above the pictures) Tj ET\n\
q 80 0 0 60 40 240 cm /Photo Do Q\n\
q 0 0 1 rg 300 300 60 40 re f Q\n\
/Panel Do\n\
q 40 0 0 40 300 60 cm BI /W 4 /H 4 /CS /G /BPC 8 ID "
        .to_vec();
    content.extend([0x40u8, 0x80, 0xC0, 0xFF].repeat(4));
    content.extend(b" EI Q\nq 1 0 0 1 120 40 cm 60 0 0 30 0 0 cm /Photo Do 0 1 0 rg 0 1.2 1 0.5 re f Q\n");
    let mut pdf = SyntheticPdf::new("1.7");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R 9 0 R] /Count 2 >>")
        .object(
            3,
            0,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> /XObject << /Photo 6 0 R /Panel 7 0 R >> >> >>",
        )
        .stream(4, "", &content)
        .object(5, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
        .stream(6, "/Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceRGB /BitsPerComponent 8", &gradient(8, 8))
        .stream(
            7,
            "/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Matrix [1 0 0 1 200 150] /Resources << /XObject << /Logo 8 0 R >> >>",
            b"q 50 0 0 50 0 0 cm /Logo Do Q",
        )
        .stream(8, "/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8", &[20, 160, 60].repeat(4))
        .object(
            9,
            0,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 300] /Rotate 90 /Contents 10 0 R \
             /Resources << /XObject << /Panel 7 0 R >> >> >>",
        )
        .stream(10, "", b"/Panel Do");
    let rows: Vec<(u32, Row)> =
        std::iter::once((0, Row::Free(65535))).chain((1..=10).map(|num| (num, Row::InUse(num)))).collect();
    let start = pdf.xref_table(&rows, "/Size 11 /Root 1 0 R");
    pdf.startxref(start).finish()
}

pub fn navigation_features() -> Vec<u8> {
    let mut pdf = SyntheticPdf::new("1.7");
    pdf.object(1, 0, "<< /Type /Catalog /Pages 2 0 R /Outlines 10 0 R /Dests 30 0 R /Names << /Dests 31 0 R >> >>")
        .object(2, 0, "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>")
        .object(
            3,
            0,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 6 0 R /Resources << /Font << /F1 7 0 R >> >> /Annots [20 0 R 21 0 R] >>",
        )
        .object(4, 0, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 8 0 R /Resources << /Font << /F1 7 0 R >> >> >>")
        .object(5, 0, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 300] /Rotate 90 /Contents 9 0 R /Resources << /Font << /F1 7 0 R >> >> >>")
        .stream(6, "", b"BT /F1 14 Tf 30 350 Td (Contents) Tj 0 -30 Td (Go to page three) Tj 0 -30 Td (Visit the website) Tj ET")
        .object(7, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
        .stream(8, "", b"BT /F1 14 Tf 30 350 Td (Page two) Tj ET")
        .stream(9, "", b"BT /F1 14 Tf 30 250 Td (Page three) Tj ET")
        .object(10, 0, "<< /Type /Outlines /First 11 0 R /Last 15 0 R /Count 5 >>")
        .object(11, 0, "<< /Title (Chapter 1) /Parent 10 0 R /Next 14 0 R /First 12 0 R /Last 13 0 R /Count 2 /Dest [3 0 R /XYZ 0 380 0] >>")
        .object(12, 0, "<< /Title (Section 1.1) /Parent 11 0 R /Next 13 0 R /A << /S /GoTo /D [4 0 R /FitH 250] >> >>")
        .object(13, 0, "<< /Title (Section 1.2) /Parent 11 0 R /Prev 12 0 R /Dest /sec12 >>")
        .object(14, 0, "<< /Title (Chapter 2) /Parent 10 0 R /Prev 11 0 R /Next 15 0 R /Dest (chapter2) >>")
        .object(15, 0, "<< /Title (Website) /Parent 10 0 R /Prev 14 0 R /A << /S /URI /URI (https://example.org/) >> >>")
        .object(20, 0, "<< /Type /Annot /Subtype /Link /Rect [28 286 160 304] /Border [0 0 0] /Dest [5 0 R /XYZ null 200 null] >>")
        .object(21, 0, "<< /Type /Annot /Subtype /Link /Rect [28 256 160 274] /Border [0 0 0] /A << /S /URI /URI (https://example.org/visit) >> >>")
        .object(30, 0, "<< /sec12 << /D [4 0 R /XYZ 0 120 0] >> >>")
        .object(31, 0, "<< /Kids [32 0 R] >>")
        .object(32, 0, "<< /Limits [(chapter2) (chapter2)] /Names [(chapter2) [5 0 R /Fit]] >>");
    let used = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 20, 21, 30, 31, 32];
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain((1..=32).map(|num| (num, if used.contains(&num) { Row::InUse(num) } else { Row::Free(0) })))
        .collect();
    let start = pdf.xref_table(&rows, "/Size 33 /Root 1 0 R");
    pdf.startxref(start).finish()
}

pub const FAX_VARIANTS: [(&str, &str); 7] = [
    ("g4", "/K -1"),
    ("g3", "/K 0 /EndOfLine true"),
    ("g3-2d", "/K 2 /EndOfLine true"),
    ("g3-fill", "/K 0 /EndOfLine true /EncodedByteAlign true"),
    ("g3-2d-fill", "/K 2 /EndOfLine true /EncodedByteAlign true"),
    ("rle", "/K 0 /EncodedByteAlign true"),
    ("g4", "/K -1 /BlackIs1 true"),
];

pub fn scan(name: &str) -> (usize, usize, Vec<u8>) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/scans");
    let bitmap = fs::read(directory.join(format!("{name}.pbm"))).unwrap();
    let header: Vec<&[u8]> = bitmap.splitn(4, |byte| byte.is_ascii_whitespace()).collect();
    let number = |part: &[u8]| String::from_utf8_lossy(part).parse::<usize>().unwrap();
    (number(header[1]), number(header[2]), header[3].to_vec())
}

pub fn fax_data(name: &str, variant: &str) -> Vec<u8> {
    fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/scans/{name}-{variant}.fax"))).unwrap()
}

pub fn scan_features(name: &str) -> Vec<u8> {
    let (width, height, _) = scan(name);
    let gap = height + 10;
    let page_height = gap * FAX_VARIANTS.len() + 10;
    let content: String = (0..FAX_VARIANTS.len())
        .map(|index| format!("q {width} 0 0 {height} 10 {} cm /Fax{index} Do Q\n", page_height - (index + 1) * gap))
        .collect();
    let mut pdf = SyntheticPdf::new("1.5");
    let xobjects: String = (0..FAX_VARIANTS.len()).map(|index| format!("/Fax{index} {} 0 R ", 10 + index)).collect();
    pdf.object(1, 0, CATALOG).object(2, 0, PAGES).object(
        3,
        0,
        &format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {page_height}] /Contents 4 0 R /Resources << /XObject << {xobjects}>> >> >>", width + 20),
    );
    pdf.stream(4, "", content.as_bytes());
    for (index, (variant, params)) in FAX_VARIANTS.iter().enumerate() {
        let entries = format!(
            "/Type /XObject /Subtype /Image /Width {width} /Height {height} /ColorSpace /DeviceGray /BitsPerComponent 1 \
             /Filter /CCITTFaxDecode /DecodeParms << {params} /Columns {width} /Rows {height} >>"
        );
        pdf.stream(10 + index as u32, &entries, &fax_data(name, variant));
    }
    let last = 9 + FAX_VARIANTS.len() as u32;
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain((1..=last).map(|num| (num, if num <= 4 || num >= 10 { Row::InUse(num) } else { Row::Free(0) })))
        .collect();
    let start = pdf.xref_table(&rows, &format!("/Size {} /Root 1 0 R", last + 1));
    pdf.startxref(start).finish()
}

fn arithmetic(template: u8, typical_prediction: bool, adaptive: &[(i8, i8)]) -> RegionCoding<'_> {
    RegionCoding::Arithmetic { template, typical_prediction, adaptive }
}

pub fn picture(name: &str) -> Picture {
    let (width, height, bits) = scan(name);
    let row_bytes = width.div_ceil(8);
    let black = (0..width * height)
        .map(|index| (bits[(index / width) * row_bytes + (index % width) / 8] >> (7 - index % width % 8)) & 1 == 1)
        .collect();
    Picture { width, height, black }
}

pub const JBIG2_COMBINATIONS: [(&str, u8); 4] = [("and", 1), ("xor", 2), ("xnor", 3), ("replace", 4)];
pub const JBIG2_SHIFT: (u32, u32) = (5, 3);

pub fn jbig2_variants(name: &str) -> Vec<(String, Vec<u8>, usize)> {
    let picture = picture(name);
    let mmr = fax_data(name, "g4");
    let custom = [(4, -1), (-4, -1), (3, -2), (-3, -2)];
    let codings: Vec<(&str, RegionCoding, usize)> = vec![
        ("template 0", arithmetic(0, false, &DEFAULT_ADAPTIVE[0]), 0),
        ("template 0 moved pixels", arithmetic(0, true, &custom), 0),
        ("template 1", arithmetic(1, true, &DEFAULT_ADAPTIVE[1]), 0),
        ("template 2", arithmetic(2, true, &DEFAULT_ADAPTIVE[2]), 0),
        ("template 3", arithmetic(3, true, &DEFAULT_ADAPTIVE[3]), 0),
        ("mmr", RegionCoding::Mmr(&mmr), 0),
        ("placed", arithmetic(0, true, &DEFAULT_ADAPTIVE[0]), 7),
    ];
    let (width, height) = (picture.width, picture.height);
    let mut variants: Vec<(String, Vec<u8>, usize)> = codings
        .into_iter()
        .map(|(label, coding, margin)| {
            let region = Region { picture: &picture, left: margin as u32, top: margin as u32, combination: 0, coding };
            (label.to_string(), embedded_stream(width + 2 * margin, height + 2 * margin, &[region]), margin)
        })
        .collect();
    for (label, combination) in JBIG2_COMBINATIONS {
        let first = Region {
            picture: &picture,
            left: 0,
            top: 0,
            combination: 0,
            coding: arithmetic(2, false, &DEFAULT_ADAPTIVE[2]),
        };
        let second = Region {
            picture: &picture,
            left: JBIG2_SHIFT.0,
            top: JBIG2_SHIFT.1,
            combination,
            coding: RegionCoding::Mmr(&mmr),
        };
        variants.push((label.to_string(), embedded_stream(width, height, &[first, second]), 0));
    }
    variants
}

pub fn jbig2_features(name: &str) -> Vec<u8> {
    let variants = jbig2_variants(name);
    let tallest = variants.iter().map(|(_, _, margin)| picture(name).height + 2 * margin).max().unwrap();
    let widest = variants.iter().map(|(_, _, margin)| picture(name).width + 2 * margin).max().unwrap();
    let gap = tallest + 10;
    let page_height = gap * variants.len() + 10;
    let mut content = String::new();
    let mut xobjects = String::new();
    let mut pdf = SyntheticPdf::new("1.5");
    for (index, (_, stream, margin)) in variants.iter().enumerate() {
        let (width, height) = (picture(name).width + 2 * margin, picture(name).height + 2 * margin);
        content
            .push_str(&format!("q {width} 0 0 {height} 10 {} cm /Scan{index} Do Q\n", page_height - (index + 1) * gap));
        xobjects.push_str(&format!("/Scan{index} {} 0 R ", 10 + index));
        let entries = format!(
            "/Type /XObject /Subtype /Image /Width {width} /Height {height} /ColorSpace /DeviceGray /BitsPerComponent 1 /Filter /JBIG2Decode"
        );
        pdf.stream(10 + index as u32, &entries, stream);
    }
    pdf.object(1, 0, CATALOG).object(2, 0, PAGES).object(
        3,
        0,
        &format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {page_height}] /Contents 4 0 R /Resources << /XObject << {xobjects}>> >> >>", widest + 20),
    );
    pdf.stream(4, "", content.as_bytes());
    let last = 9 + variants.len() as u32;
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain((1..=last).map(|num| (num, if num <= 4 || num >= 10 { Row::InUse(num) } else { Row::Free(0) })))
        .collect();
    let start = pdf.xref_table(&rows, &format!("/Size {} /Root 1 0 R", last + 1));
    pdf.startxref(start).finish()
}

pub fn test_symbols() -> Vec<Picture> {
    let sizes = [(7, 8), (9, 8), (6, 10), (11, 12), (8, 12), (10, 12)];
    sizes
        .iter()
        .enumerate()
        .map(|(index, (width, height))| {
            let mut symbol = Picture::blank(*width, *height);
            for y in 0..*height as i64 {
                for x in 0..*width as i64 {
                    let stroke =
                        x == 0 || y == *height as i64 - 1 || (x + y * (index as i64 + 2)) % (index as i64 + 3) == 0;
                    symbol.set(x, y, stroke);
                }
            }
            symbol
        })
        .collect()
}

pub struct Jbig2Scenario {
    pub label: String,
    pub globals: Option<Vec<u8>>,
    pub stream: Vec<u8>,
    pub expected: Picture,
}

pub fn jbig2_symbol_scenarios() -> Vec<Jbig2Scenario> {
    let symbols = test_symbols();
    let drawn: Vec<NewSymbol> = symbols.iter().map(NewSymbol::Drawn).collect();
    let globals = segment(0, 0, &[], false, &symbol_dictionary(&[], &drawn, false, &vec![true; drawn.len()]));
    let (width, height) = (480, 60);
    let page = || segment(1, 48, &[], true, &page_information(width, height));
    let mut scenarios = Vec::new();

    let mut widened = symbols[3].clone();
    widened.width += 2;
    widened.black = (0..widened.width * widened.height)
        .map(|index| {
            let (x, y) = ((index % widened.width) as i64, (index / widened.width) as i64);
            symbols[3].get(x - 1, y) == 1 || (x == 12 && y < 6)
        })
        .collect();
    let mut narrowed = Picture::blank(symbols[4].width - 1, symbols[4].height - 1);
    narrowed.stamp(&symbols[4], -1, 0);
    let placements: Vec<Placed> = vec![
        Placed { id: 0, left: 3, top: 4, refined: None },
        Placed { id: 1, left: 12, top: 4, refined: None },
        Placed { id: 3, left: 24, top: 2, refined: Some((&widened, 0, 0)) },
        Placed { id: 4, left: 60, top: 4, refined: Some((&narrowed, 1, -1)) },
        Placed { id: 2, left: 430, top: 20, refined: None },
        Placed { id: 1, left: 5, top: -5000, refined: None },
        Placed { id: 2, left: 40, top: 3, refined: None },
        Placed { id: 5, left: 5, top: 30, refined: None },
        Placed { id: 4, left: 18, top: 31, refined: None },
        Placed { id: 0, left: 70, top: 33, refined: None },
    ];
    let mut expected = Picture::blank(width, height);
    for instance in &placements {
        expected.stamp(
            instance.refined.map_or(&symbols[instance.id], |(picture, _, _)| picture),
            instance.left,
            instance.top,
        );
    }
    let mut stream = page();
    stream.extend(segment(2, 6, &[0], true, &text_region(width, height, &symbols, &placements, &TextLayout::plain())));
    scenarios.push(Jbig2Scenario {
        label: "text with a refined instance".to_string(),
        globals: Some(globals.clone()),
        stream,
        expected,
    });

    for (corner, transposed, strips, s_offset) in
        [(2u8, true, 4i64, -1i64), (0, false, 2, 2), (3, false, 8, 0), (1, true, 1, 1)]
    {
        let layout = TextLayout { strips, corner, transposed, s_offset };
        let placements: Vec<Placed> = (0..9)
            .map(|index| Placed {
                id: index % symbols.len(),
                left: 4 + (index as i64 % 3) * 35 + index as i64,
                top: 3 + (index as i64 / 3) * 18,
                refined: None,
            })
            .collect();
        let mut expected = Picture::blank(width, height);
        for instance in &placements {
            expected.stamp(&symbols[instance.id], instance.left, instance.top);
        }
        let mut stream = page();
        stream.extend(segment(2, 6, &[0], true, &text_region(width, height, &symbols, &placements, &layout)));
        let label = format!("text corner {corner} transposed {transposed} strips {strips} offset {s_offset}");
        scenarios.push(Jbig2Scenario { label, globals: Some(globals.clone()), stream, expected });
    }

    let mut thickened = symbols[1].clone();
    thickened.black = (0..thickened.width * thickened.height)
        .map(|index| {
            let (x, y) = ((index % thickened.width) as i64, (index / thickened.width) as i64);
            symbols[1].get(x, y) == 1 || symbols[1].get(x - 1, y) == 1
        })
        .collect();
    let mut pair = Picture::blank(symbols[0].width + symbols[2].width + 1, symbols[2].height);
    pair.stamp(&symbols[0], 0, 2);
    pair.stamp(&symbols[2], symbols[0].width as i64 + 1, 0);
    let refined = vec![
        NewSymbol::Refined { picture: &thickened, of: 1, offset: (1, -1) },
        NewSymbol::Aggregate { picture: &pair, parts: vec![(0, 0, 2), (2, symbols[0].width as i64 + 1, 0)] },
    ];
    let mut stream = page();
    let exported = [false, true, false, false, false, false, true, true];
    stream.extend(segment(2, 0, &[0], true, &symbol_dictionary(&symbols, &refined, true, &exported)));
    let mut known: Vec<Picture> = symbols.clone();
    known.push(symbols[1].clone());
    known.push(thickened.clone());
    known.push(pair.clone());
    let placements = vec![
        Placed { id: symbols.len() + 1, left: 10, top: 10, refined: None },
        Placed { id: symbols.len() + 2, left: 40, top: 20, refined: None },
        Placed { id: symbols.len(), left: 60, top: 5, refined: None },
        Placed { id: 4, left: 80, top: 30, refined: None },
    ];
    let mut expected = Picture::blank(width, height);
    for instance in &placements {
        expected.stamp(&known[instance.id], instance.left, instance.top);
    }
    stream.extend(segment(3, 6, &[0, 2], true, &text_region(width, height, &known, &placements, &TextLayout::plain())));
    scenarios.push(Jbig2Scenario {
        label: "refined and aggregate symbols".to_string(),
        globals: Some(globals.clone()),
        stream,
        expected,
    });

    let first = picture("odd");
    let mut changed = first.clone();
    for x in 0..first.width as i64 {
        changed.set(x, 4, x % 3 != 0);
    }
    let first_region = |kind: u8, left: u32, top: u32| {
        let region = Region {
            picture: &first,
            left,
            top,
            combination: 0,
            coding: RegionCoding::Arithmetic { template: 0, typical_prediction: true, adaptive: &DEFAULT_ADAPTIVE[0] },
        };
        segment(2, kind, &[], true, &generic_region_data(&region))
    };
    let mut under = Picture::blank(first.width, first.height);
    under.stamp(&first, 0, 0);
    for (template, typical_prediction, left, top) in
        [(1u8, false, 6, 5), (0, false, 6, 5), (1, true, 6, 5), (0, true, 6, 5), (1, true, 0, 0), (0, true, 0, 0)]
    {
        let mut stream = page();
        stream.extend(first_region(38, left, top));
        stream.extend(segment(
            3,
            42,
            &[],
            true,
            &refinement_region(&changed, &under, left, top, 4, template, typical_prediction),
        ));
        let mut expected = Picture::blank(width, height);
        expected.stamp(&changed, left as i64, top as i64);
        let label = format!(
            "refinement of the page at ({left}, {top}), template {template}, typical prediction {typical_prediction}"
        );
        scenarios.push(Jbig2Scenario { label, globals: None, stream, expected });
    }

    let mut stream = page();
    stream.extend(first_region(36, 6, 5));
    stream.extend(segment(3, 42, &[2], true, &refinement_region(&changed, &first, 6, 5, 0, 0, false)));
    let mut expected = Picture::blank(width, height);
    expected.stamp(&changed, 6, 5);
    scenarios.push(Jbig2Scenario {
        label: "refinement of an intermediate region".to_string(),
        globals: None,
        stream,
        expected,
    });

    let (gray, patterns, (grid_x, grid_y), (step, skew), expected) = halftone_scene();
    let size = patterns[0].width;
    let mut stream = page();
    stream.extend(segment(2, 16, &[], true, &pattern_dictionary(&patterns)));
    stream.extend(segment(
        3,
        22,
        &[2],
        true,
        &halftone_region(width, height, &gray, patterns.len(), (grid_x, grid_y), (step, skew), size),
    ));
    scenarios.push(Jbig2Scenario { label: "halftone".to_string(), globals: None, stream, expected });
    scenarios
}

#[allow(clippy::type_complexity)]
pub fn halftone_scene() -> (Vec<Vec<usize>>, Vec<Picture>, (i32, i32), (u16, u16), Picture) {
    let size = 4;
    let patterns: Vec<Picture> = (0..5)
        .map(|level| {
            let mut pattern = Picture::blank(size, size);
            for index in 0..level * 3 + level / 2 {
                pattern.set((index * 5 % 16 % 4) as i64, (index * 5 % 16 / 4) as i64, true);
            }
            pattern
        })
        .collect();
    let gray: Vec<Vec<usize>> = (0..16).map(|row| (0..32).map(|column| (row + column / 3) % 5).collect()).collect();
    let (grid_x, grid_y, step, skew) = (-4 * 256, -256, (size as u16) << 8, 64u16);
    let mut expected = Picture::blank(480, 60);
    for (row, values) in gray.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            let x = (grid_x as i64 + row as i64 * skew as i64 + column as i64 * step as i64) >> 8;
            let y = (grid_y as i64 + row as i64 * step as i64 - column as i64 * skew as i64) >> 8;
            expected.stamp(&patterns[*value], x, y);
        }
    }
    (gray, patterns, (grid_x, grid_y), (step, skew), expected)
}

pub fn jbig2_symbol_features() -> Vec<u8> {
    jbig2_scenario_features(&jbig2_symbol_scenarios())
}

pub fn jbig2_scenario_features(scenarios: &[Jbig2Scenario]) -> Vec<u8> {
    let (width, height) = (scenarios[0].expected.width, scenarios[0].expected.height);
    let gap = height + 10;
    let page_height = gap * scenarios.len() + 10;
    let mut content = String::new();
    let mut xobjects = String::new();
    let mut pdf = SyntheticPdf::new("1.5");
    let mut last = 10;
    for (index, scenario) in scenarios.iter().enumerate() {
        content
            .push_str(&format!("q {width} 0 0 {height} 10 {} cm /Scan{index} Do Q\n", page_height - (index + 1) * gap));
        let image = last;
        let globals = scenario.globals.as_ref().map(|globals| {
            pdf.stream(image + 1, "", globals);
            format!(" /DecodeParms << /JBIG2Globals {} 0 R >>", image + 1)
        });
        let entries = format!(
            "/Type /XObject /Subtype /Image /Width {width} /Height {height} /ColorSpace /DeviceGray /BitsPerComponent 1 /Filter /JBIG2Decode{}",
            globals.clone().unwrap_or_default()
        );
        pdf.stream(image, &entries, &scenario.stream);
        xobjects.push_str(&format!("/Scan{index} {image} 0 R "));
        last += if globals.is_some() { 2 } else { 1 };
    }
    pdf.object(1, 0, CATALOG).object(2, 0, PAGES).object(
        3,
        0,
        &format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {page_height}] /Contents 4 0 R /Resources << /XObject << {xobjects}>> >> >>", width + 20),
    );
    pdf.stream(4, "", content.as_bytes());
    let rows: Vec<(u32, Row)> = std::iter::once((0, Row::Free(65535)))
        .chain((1..last).map(|num| (num, if num <= 4 || num >= 10 { Row::InUse(num) } else { Row::Free(0) })))
        .collect();
    let start = pdf.xref_table(&rows, &format!("/Size {last} /Root 1 0 R"));
    pdf.startxref(start).finish()
}

pub fn jpx_features() -> Vec<u8> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/jpx");
    let read = |name: &str| fs::read(directory.join(name)).unwrap();
    let mut pdf = SyntheticPdf::new("1.5");
    pdf.object(1, 0, CATALOG)
        .object(2, 0, PAGES)
        .object(3, 0, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 420 160] /Contents 4 0 R /Resources << /XObject << /A 5 0 R /B 6 0 R /C 7 0 R >> >> >>")
        .stream(4, "", b"q 1 0.85 0.2 rg 280 10 130 140 re f Q q 0 0.4 0.9 rg 300 60 90 20 re f Q q 120 0 0 76 10 40 cm /A Do Q q 120 0 0 90 145 35 cm /B Do Q q 130 0 0 82 280 40 cm /C Do Q")
        .stream(5, "/Type /XObject /Subtype /Image /Width 97 /Height 61 /BitsPerComponent 8 /Filter /JPXDecode", &read("rgb-lossy.j2k"))
        .stream(6, "/Type /XObject /Subtype /Image /Width 96 /Height 72 /BitsPerComponent 8 /Filter /JPXDecode", &read("gray-jp2.jp2"))
        .stream(7, "/Type /XObject /Subtype /Image /Width 97 /Height 61 /BitsPerComponent 8 /SMaskInData 1 /Filter /JPXDecode", &read("rgba-jp2.jp2"));
    let rows: Vec<(u32, Row)> =
        std::iter::once((0, Row::Free(65535))).chain((1..=7).map(|num| (num, Row::InUse(num)))).collect();
    let start = pdf.xref_table(&rows, "/Size 8 /Root 1 0 R");
    pdf.startxref(start).finish()
}
