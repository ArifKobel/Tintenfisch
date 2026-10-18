use std::fs;
use std::path::PathBuf;

use crate::support::fixtures::{Jbig2Scenario, halftone_scene, test_symbols};
use crate::support::jbig2_huffman::{
    CodeTable, Collective, CustomTable, HuffmanTextRegion, RefinedSymbol, TextCodes, collective_picture,
    halftone_mmr_region, huffman_refined_dictionary, huffman_symbol_dictionary, huffman_text_region,
    text_huffman_flags,
};
use crate::support::jbig2_writer::{
    NewSymbol, Picture, Placed, TextLayout, page_information, pattern_dictionary, segment, symbol_dictionary,
};

const PAGE: (usize, usize) = (480, 60);

pub fn mmr_sources() -> Vec<(String, Picture)> {
    let symbols = test_symbols();
    let mut sources = vec![
        ("collective-8".to_string(), collective_picture(&symbols[0..2])),
        ("collective-12".to_string(), collective_picture(&symbols[3..6])),
    ];
    let (gray, patterns, ..) = halftone_scene();
    let planes = usize::BITS - patterns.len().saturating_sub(1).leading_zeros();
    for plane in (0..planes).rev() {
        let mut bits = Picture::blank(gray[0].len(), gray.len());
        for (row, values) in gray.iter().enumerate() {
            for (column, value) in values.iter().enumerate() {
                bits.set(column as i64, row as i64, ((value >> plane) ^ (value >> (plane + 1))) & 1 == 1);
            }
        }
        sources.push((format!("plane-{plane}"), bits));
    }
    sources
}

fn mmr(name: &str) -> Vec<u8> {
    fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/scans/huffman/{name}.g4"))).unwrap()
}

fn standard_codes(numbers: [usize; 8]) -> TextCodes {
    let table = CodeTable::standard;
    TextCodes {
        first_s: table(numbers[0]),
        s_delta: table(numbers[1]),
        strip_delta: table(numbers[2]),
        refinement_width: table(numbers[3]),
        refinement_height: table(numbers[4]),
        refinement_x: table(numbers[5]),
        refinement_y: table(numbers[6]),
        refinement_size: table(numbers[7]),
    }
}

fn expected_page(symbols: &[Picture], placed: &[Placed]) -> Picture {
    let mut expected = Picture::blank(PAGE.0, PAGE.1);
    for instance in placed {
        expected.stamp(
            instance.refined.map_or(&symbols[instance.id], |(picture, _, _)| picture),
            instance.left,
            instance.top,
        );
    }
    expected
}

fn spread(symbols: usize, count: usize) -> Vec<Placed<'static>> {
    (0..count)
        .map(|index| Placed {
            id: index % symbols,
            left: 4 + (index as i64 % 3) * 37 + index as i64 * 2,
            top: 3 + (index as i64 / 3) * 18,
            refined: None,
        })
        .collect()
}

pub fn jbig2_huffman_scenarios() -> Vec<Jbig2Scenario> {
    let symbols = test_symbols();
    let mut ordered = symbols.clone();
    ordered.sort_by_key(|symbol| (symbol.height, symbol.width));
    let page = || segment(1, 48, &[], true, &page_information(PAGE.0, PAGE.1));
    let drawn: Vec<NewSymbol> = symbols.iter().map(NewSymbol::Drawn).collect();
    let globals = segment(0, 0, &[], false, &symbol_dictionary(&[], &drawn, false, &vec![true; drawn.len()]));
    let mut scenarios = Vec::new();

    let placed = spread(ordered.len(), 9);
    let mut stream = page();
    let dictionary = huffman_symbol_dictionary(
        &ordered,
        &[Collective::Stored, Collective::Stored, Collective::Stored],
        &[true; 6],
        0,
        (&CodeTable::standard(4), &CodeTable::standard(2)),
    );
    stream.extend(segment(2, 0, &[], true, &dictionary));
    let text = HuffmanTextRegion {
        width: PAGE.0,
        height: PAGE.1,
        symbols: &ordered,
        placed: &placed,
        layout: &TextLayout::plain(),
        refine: false,
        huffman_flags: 0,
        codes: &standard_codes([6, 8, 11, 14, 14, 14, 14, 1]),
        id_lengths: &[3, 3, 3, 3, 2, 2],
    };
    stream.extend(segment(3, 6, &[2], true, &huffman_text_region(&text)));
    scenarios.push(Jbig2Scenario {
        label: "Huffman dictionary with stored bitmaps and Huffman text".to_string(),
        globals: None,
        stream,
        expected: expected_page(&ordered, &placed),
    });

    let (collective_8, collective_12) = (mmr("collective-8"), mmr("collective-12"));
    let mut widened = symbols[3].clone();
    widened.width += 2;
    widened.black = (0..widened.width * widened.height)
        .map(|index| {
            let (x, y) = ((index % widened.width) as i64, (index / widened.width) as i64);
            symbols[3].get(x - 1, y) == 1 || (x == 12 && y < 6)
        })
        .collect();
    let mut placed = spread(symbols.len(), 8);
    placed.push(Placed { id: 3, left: 300, top: 30, refined: Some((&widened, 0, 0)) });
    let mut stream = page();
    let dictionary = huffman_symbol_dictionary(
        &symbols,
        &[Collective::Mmr(&collective_8), Collective::Stored, Collective::Mmr(&collective_12)],
        &[true; 6],
        1 << 2 | 1 << 4,
        (&CodeTable::standard(5), &CodeTable::standard(3)),
    );
    stream.extend(segment(2, 0, &[], true, &dictionary));
    let layout = TextLayout { strips: 4, corner: 2, transposed: false, s_offset: 1 };
    let text = HuffmanTextRegion {
        width: PAGE.0,
        height: PAGE.1,
        symbols: &symbols,
        placed: &placed,
        layout: &layout,
        refine: true,
        huffman_flags: text_huffman_flags(1, 1, 1, [1; 4], 0),
        codes: &standard_codes([7, 9, 12, 15, 15, 15, 15, 1]),
        id_lengths: &[2, 3, 3, 3, 3, 2],
    };
    stream.extend(segment(3, 6, &[2], true, &huffman_text_region(&text)));
    scenarios.push(Jbig2Scenario {
        label: "MMR collective bitmaps, other tables, strips and a refined instance".to_string(),
        globals: None,
        stream,
        expected: expected_page(&symbols, &placed),
    });

    let heights =
        CustomTable { low: -4, ranges: vec![(2, 2), (2, 3), (3, 4), (3, 5)], lower: 4, upper: 4, out_of_band: None };
    let first_s = CustomTable { low: -8, ranges: vec![(2, 3), (2, 5), (2, 7)], lower: 3, upper: 3, out_of_band: None };
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
    let mut known = symbols.clone();
    known.extend(ordered.iter().cloned());
    let new = vec![
        RefinedSymbol::Refined { picture: &thickened, of: 7, offset: (1, -1) },
        RefinedSymbol::Aggregate {
            picture: &pair,
            parts: vec![
                Placed { id: 0, left: 0, top: 2, refined: None },
                Placed { id: 8, left: symbols[0].width as i64 + 1, top: 0, refined: None },
            ],
        },
    ];
    let mut exported = vec![true; known.len()];
    exported.extend([true, true]);
    let mut all = known.clone();
    all.push(thickened.clone());
    all.push(pair.clone());
    let placed = vec![
        Placed { id: 12, left: 10, top: 10, refined: None },
        Placed { id: 13, left: 40, top: 20, refined: None },
        Placed { id: 0, left: 90, top: 30, refined: None },
        Placed { id: 13, left: 200, top: 35, refined: None },
    ];
    let mut id_lengths = vec![0; all.len()];
    id_lengths[0] = 1;
    id_lengths[12] = 2;
    id_lengths[13] = 2;
    let mut codes = standard_codes([6, 10, 13, 14, 14, 14, 14, 1]);
    codes.first_s = first_s.codes();
    let mut stream = page();
    stream.extend(segment(2, 53, &[], true, &heights.segment_data(3, 3)));
    stream.extend(segment(3, 53, &[], true, &first_s.segment_data(2, 3)));
    let dictionary = huffman_symbol_dictionary(
        &ordered,
        &[Collective::Stored, Collective::Stored, Collective::Stored],
        &[true; 6],
        3 << 2,
        (&heights.codes(), &CodeTable::standard(2)),
    );
    stream.extend(segment(4, 0, &[2], true, &dictionary));
    stream.extend(segment(5, 0, &[0, 4], true, &huffman_refined_dictionary(&known, &new, &exported)));
    let text = HuffmanTextRegion {
        width: PAGE.0,
        height: PAGE.1,
        symbols: &all,
        placed: &placed,
        layout: &TextLayout::plain(),
        refine: true,
        huffman_flags: text_huffman_flags(3, 2, 2, [0; 4], 0),
        codes: &codes,
        id_lengths: &id_lengths,
    };
    stream.extend(segment(6, 6, &[5, 3], true, &huffman_text_region(&text)));
    scenarios.push(Jbig2Scenario {
        label: "custom tables, refined and aggregate Huffman symbols".to_string(),
        globals: Some(globals.clone()),
        stream,
        expected: expected_page(&all, &placed),
    });

    let (gray, patterns, origin, steps, expected) = halftone_scene();
    let planes: Vec<Vec<u8>> =
        mmr_sources().iter().filter(|(name, _)| name.starts_with("plane")).map(|(name, _)| mmr(name)).collect();
    let mut stream = page();
    stream.extend(segment(2, 16, &[], true, &pattern_dictionary(&patterns)));
    stream.extend(segment(
        3,
        22,
        &[2],
        true,
        &halftone_mmr_region(PAGE.0, PAGE.1, (gray[0].len(), gray.len()), origin, steps, &planes),
    ));
    scenarios.push(Jbig2Scenario { label: "MMR halftone".to_string(), globals: None, stream, expected });
    scenarios
}
