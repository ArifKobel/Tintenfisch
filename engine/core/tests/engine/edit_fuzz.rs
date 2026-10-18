use crate::support;

use support::fixtures;
use support::random::Random;
use support::rendering::library;
use tintenfisch::document::Document;
use tintenfisch::edit::ParagraphEdit;
use tintenfisch::error::ErrorKind;
use tintenfisch::text::{LayoutLine, split_graphemes};

const ALPHABET: [&str; 16] = ["a", "e", "x", "Q", "7", " ", "é", "ß", "€", "Ω", "-", ".", "m", "W", "\n", " "];

fn page_text(lines: &[LayoutLine]) -> String {
    lines.iter().flat_map(|line| &line.clusters).map(|cluster| cluster.text.as_str()).collect()
}

fn ink(text: &str) -> String {
    text.chars().filter(|character| !character.is_whitespace()).collect()
}

fn sorted_ink(text: &str) -> Vec<char> {
    let mut characters: Vec<char> = ink(text).chars().collect();
    characters.sort_unstable();
    characters
}

fn mutated(random: &mut Random, text: &str) -> String {
    let mut clusters = split_graphemes(text);
    let length = clusters.len();
    let at = random.below(length + 1);
    let letter = |random: &mut Random| ALPHABET[random.below(ALPHABET.len())].to_string();
    match random.below(4) {
        3 => {
            let word: String = (0..3 + random.below(9)).map(|_| letter(random)).collect();
            clusters.insert(at, format!("{word} "));
        }
        1 if length > 1 => {
            let start = at.min(length - 1);
            let end = (start + 1 + random.below(3)).min(length);
            clusters.drain(start..end);
        }
        2 if length > 0 => clusters[at.min(length - 1)] = letter(random),
        _ => {
            for offset in 0..1 + random.below(3) {
                let inserted = letter(random);
                clusters.insert(at + offset, inserted);
            }
        }
    }
    if clusters.iter().all(|cluster| cluster.trim().is_empty()) {
        clusters.push("z".to_string());
    }
    clusters.concat()
}

#[test]
fn random_edits_land_as_typed_and_undo_completely() {
    let library = library();
    let (mut edited, mut refused) = (0, 0);
    for fixture in fixtures::all() {
        for seed in 0..6 {
            let mut document = Document::open(fixture.bytes.clone(), "").unwrap();
            let opened = document.save(false).unwrap();
            let Some(page) = document.view(&library).pages().unwrap().first().map(|page| page.id) else { continue };
            let mut random = Random::new(seed);
            let mut overlapping = false;
            for step in 0..8 {
                let lines = document.view(&library).text_layout(page).unwrap().lines().to_vec();
                if lines.is_empty() {
                    break;
                }
                let line = random.below(lines.len());
                let paragraph = match document.paragraph_text(page, line, &library) {
                    Ok(paragraph) => paragraph,
                    Err(error) if error.kind == ErrorKind::Unsupported => {
                        refused += 1;
                        continue;
                    }
                    Err(error) => panic!("{} seed {seed} step {step}: {error}", fixture.name),
                };
                let old = paragraph.clusters.concat();
                let text = mutated(&mut random, &old);
                let mut edit = ParagraphEdit::new(page, line, text.clone());
                if random.chance(20) {
                    overlapping = true;
                    edit = edit.moved_by(random.below(41) as f64 - 20.0, random.below(41) as f64 - 20.0);
                }
                let context = format!("{} seed {seed} step {step}: {old:?} -> {text:?}", fixture.name);
                match document.edit(&edit, &library, "") {
                    Ok(()) => edited += 1,
                    Err(error) if error.kind == ErrorKind::Unsupported => {
                        refused += 1;
                        continue;
                    }
                    Err(error) => panic!("{context}: {error}"),
                }
                let after = page_text(document.view(&library).text_layout(page).unwrap().lines());
                let end = paragraph.line + paragraph.line_starts.len();
                let expected = format!("{}{}{}", page_text(&lines[..paragraph.line]), text, page_text(&lines[end..]));
                assert_eq!(sorted_ink(&after), sorted_ink(&expected), "{context}: glyphs were lost or duplicated");
                if !overlapping {
                    assert_eq!(ink(&after), ink(&expected), "{context}: the page reads out of order");
                }
            }
            while document.undo().is_some() {}
            assert!(document.save(false).unwrap() == opened, "{} seed {seed}: undo left changes behind", fixture.name);
        }
    }
    assert!(refused * 20 < edited, "{refused} of {} edits were refused", edited + refused);
}
