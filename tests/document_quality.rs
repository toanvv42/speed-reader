use speed_reader::doc::{Block, TableBlock, blocks_from_plain_text, parse_markdown};

#[test]
fn markdown_fixture_preserves_reader_structure() {
    let blocks = parse_markdown(include_str!("fixtures/real_world_markdown.md"));

    assert!(matches!(blocks.first(), Some(Block::Heading(1, title)) if title == "Field Notes"));
    assert!(blocks.iter().any(|block| matches!(block, Block::Heading(2, title) if title == "Findings")));
    assert!(blocks.iter().any(|block| matches!(block, Block::Code(code) if code.contains("let pace"))));
    assert!(blocks.iter().any(|block| matches!(block, Block::Image(path) if path == "assets/diagram.png")));
    assert!(blocks.iter().any(|block| matches!(block, Block::Text(text) if text.contains("tiếng Việt"))));
}

#[test]
fn markdown_fixture_keeps_tables_as_full_blocks() {
    let blocks = parse_markdown(include_str!("fixtures/real_world_markdown.md"));
    let table = blocks.iter().find_map(|block| match block {
        Block::Table(table) => Some(table),
        _ => None,
    });

    assert_eq!(
        table,
        Some(&TableBlock {
            headers: vec!["Signal".into(), "Result".into(), "Notes".into()],
            rows: vec![
                vec!["Headings".into(), "OK".into(), "chapter picker should work".into()],
                vec!["Tables".into(), "OK".into(), "displayed as full blocks".into()],
            ],
        })
    );
}

#[test]
fn pdf_like_text_fixture_detects_fixed_width_tables() {
    let blocks = blocks_from_plain_text(include_str!("fixtures/pdf_extracted_text.txt"));

    assert!(blocks.iter().any(|block| matches!(block, Block::Text(text) if text.contains("1. Overview"))));
    assert!(blocks.iter().any(|block| matches!(block, Block::Table(table) if table.headers == ["Metric", "Value", "Note"])));
    assert!(blocks.iter().any(|block| matches!(block, Block::Text(text) if text.contains("2. Details"))));
}

