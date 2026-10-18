use crate::support;

use support::fixtures;
use support::rendering::{TestRenderer, to_png};

#[test]
#[ignore]
fn write_previews() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    for fixture in fixtures::all() {
        let renderer = TestRenderer::new(fixture.bytes);
        let raster = renderer.render(0, 1.0, None);
        std::fs::write(format!("{directory}/{}.png", fixture.name), to_png(&raster)).unwrap();
    }
}

#[test]
#[ignore]
fn write_fixture_files() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    for fixture in fixtures::synthetic() {
        std::fs::write(format!("{directory}/{}", fixture.name), fixture.bytes).unwrap();
    }
}

#[test]
#[ignore]
fn write_feature_pages() {
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let renderer = TestRenderer::new(fixtures::render_features());
    for page in 0..renderer.page_count() {
        std::fs::write(format!("{directory}/features-{page}.png"), to_png(&renderer.render(page, 2.0, None))).unwrap();
    }
}

#[test]
#[ignore]
fn print_text_lines() {
    for fixture in fixtures::all() {
        let renderer = TestRenderer::new(fixture.bytes);
        for page in 0..renderer.page_count() {
            for line in renderer.layout(page).lines() {
                let text: String = line.clusters.iter().map(|cluster| cluster.text.as_str()).collect();
                println!(
                    "{} p{page} [{}] {:>6.1} {:>6.1}..{:>6.1} {text}",
                    fixture.name,
                    line.paragraph,
                    line.baseline,
                    line.left(),
                    line.right()
                );
            }
        }
    }
}

#[test]
#[ignore]
fn write_edited_preview() {
    use tintenfisch::document::Document;
    use tintenfisch::edit::ParagraphEdit;
    let directory = std::env::var("PREVIEW_DIR").unwrap();
    let library = support::rendering::library();
    let mut document = Document::open(fixtures::committed().remove(0).bytes, "").unwrap();
    let page = document.view(&library).pages().unwrap()[0].id;
    for (line, from, to) in [
        (0, "Structure", "Architecture"),
        (1, "a sequence of", "a list of"),
        (9, "Modern writers emit", "Modern PDF writers like to emit"),
    ] {
        let text: String = document.paragraph_text(page, line, &library).unwrap().clusters.concat();
        document.edit(&ParagraphEdit::new(page, line, text.replace(from, to)), &library, "").unwrap();
    }
    let view = document.view(&library);
    for line in view.text_layout(page).unwrap().lines().iter().take(16) {
        println!("{}", line.clusters.iter().map(|cluster| cluster.text.as_str()).collect::<String>());
    }
    std::fs::write(format!("{directory}/edited.png"), to_png(&view.render(page, 1.5, None).unwrap())).unwrap();
    std::fs::write(format!("{directory}/edited.pdf"), document.save(false).unwrap()).unwrap();

    let mut document = Document::open(fixtures::editing_features(), "").unwrap();
    let page = document.view(&library).pages().unwrap()[0].id;
    document.edit(&ParagraphEdit::new(page, 0, "Plain line to edit \u{3a9}\u{e9}"), &library, "").unwrap();
    std::fs::write(format!("{directory}/edited-fallback.pdf"), document.save(false).unwrap()).unwrap();

    let mut document = Document::open(fixtures::committed().remove(0).bytes, "").unwrap();
    let page = document.view(&library).pages().unwrap()[0].id;
    let text: String = document.paragraph_text(page, 9, &library).unwrap().clusters.concat();
    let edit = ParagraphEdit::new(page, 9, text).moved_by(40.0, 60.0).with_width(300.0);
    document.edit(&edit, &library, "").unwrap();
    std::fs::write(format!("{directory}/edited-box.pdf"), document.save(false).unwrap()).unwrap();

    let mut document = Document::open(fixtures::form_features(), "").unwrap();
    let page = document.view(&library).pages().unwrap()[0].id;
    for (from, to) in [("Shared form text", "Shared form text, copied"), ("Inner form line", "Inner form line \u{3a9}")]
    {
        let layout = document.view(&library).text_layout(page).unwrap();
        let line = layout
            .lines()
            .iter()
            .position(|line| line.clusters.iter().map(|cluster| cluster.text.as_str()).collect::<String>() == from)
            .unwrap();
        document.edit(&ParagraphEdit::new(page, line, to), &library, "").unwrap();
    }
    std::fs::write(format!("{directory}/edited-forms.pdf"), document.save(false).unwrap()).unwrap();
    std::fs::write(
        format!("{directory}/edited-forms.png"),
        to_png(&document.view(&library).render(page, 2.0, None).unwrap()),
    )
    .unwrap();
    std::fs::write(
        format!("{directory}/edited-box.png"),
        to_png(&document.view(&library).render(page, 1.5, None).unwrap()),
    )
    .unwrap();
}
