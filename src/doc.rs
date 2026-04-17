#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use std::io::{Cursor, Read};

use anyhow::{Context, Result};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use quick_xml::Reader as XmlReader;
use quick_xml::events::Event as XmlEvent;
use zip::ZipArchive;

use crate::text::sanitize;

#[derive(Debug, PartialEq, Eq)]
pub enum Block {
    Text(String),
    #[allow(dead_code)] // u8 level used in PR 2 for heading-size scaling
    Heading(u8, String),
    Code(String),
    Image(String),
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load(path: &Path) -> Result<Vec<Block>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "md" | "markdown" | "mdx" => {
            let src = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            Ok(parse_markdown(&src))
        }
        "docx" => {
            let bytes = std::fs::read(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            parse_docx(&bytes)
        }
        "pdf" => {
            let bytes = std::fs::read(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let text = pdf_extract::extract_text_from_mem(&bytes)
                .with_context(|| format!("failed to extract text from {}", path.display()))?;
            Ok(blocks_from_plain_text(&text))
        }
        _ => {
            let src = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            Ok(blocks_from_plain_text(&src))
        }
    }
}

/// Splits plain text on blank lines into paragraph `Block::Text` entries
/// after running it through `sanitize`.
pub fn blocks_from_plain_text(src: &str) -> Vec<Block> {
    let cleaned = sanitize(src);
    let mut blocks = Vec::new();
    for para in cleaned.split("\n\n") {
        let flat = para.replace('\n', " ");
        let trimmed = flat.trim();
        if !trimmed.is_empty() {
            blocks.push(Block::Text(trimmed.to_string()));
        }
    }
    blocks
}

pub fn parse_markdown(src: &str) -> Vec<Block> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(src, opts);

    let mut blocks = Vec::new();
    let mut buf = String::new();
    let mut heading_level: Option<u8> = None;
    let mut in_code = false;
    let mut in_image = false;
    let mut code_buf = String::new();

    for ev in parser {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_paragraph(&mut blocks, &mut buf);
                heading_level = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(lvl) = heading_level.take() {
                    let s = std::mem::take(&mut buf);
                    let cleaned = sanitize(s.trim());
                    if !cleaned.is_empty() {
                        blocks.push(Block::Heading(lvl, cleaned));
                    }
                }
            }
            Event::Start(Tag::CodeBlock(_)) => {
                flush_paragraph(&mut blocks, &mut buf);
                in_code = true;
                code_buf.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                let s = std::mem::take(&mut code_buf);
                if !s.trim().is_empty() {
                    blocks.push(Block::Code(s.trim().to_string()));
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                flush_paragraph(&mut blocks, &mut buf);
                blocks.push(Block::Image(dest_url.to_string()));
                in_image = true;
            }
            Event::End(TagEnd::Image) => {
                in_image = false;
            }
            Event::Text(t) => {
                if in_code {
                    code_buf.push_str(&t);
                } else if !in_image {
                    buf.push_str(&t);
                }
            }
            Event::Code(t) if !in_image => {
                buf.push_str(&t);
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_code {
                    code_buf.push('\n');
                } else if !in_image {
                    buf.push(' ');
                }
            }
            Event::End(TagEnd::Paragraph) | Event::End(TagEnd::Item) => {
                flush_paragraph(&mut blocks, &mut buf);
            }
            _ => {}
        }
    }
    flush_paragraph(&mut blocks, &mut buf);
    blocks
}

fn flush_paragraph(blocks: &mut Vec<Block>, buf: &mut String) {
    let cleaned = sanitize(buf.trim());
    if !cleaned.is_empty() {
        blocks.push(Block::Text(cleaned));
    }
    buf.clear();
}

/// Parses a .docx (ZIP of OOXML) byte slice into blocks. Headings are
/// recognized via the paragraph's `w:pStyle` value (e.g. "Heading1").
pub fn parse_docx(bytes: &[u8]) -> Result<Vec<Block>> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).context("not a valid docx (zip) file")?;
    let mut file = archive
        .by_name("word/document.xml")
        .context("docx is missing word/document.xml")?;
    let mut xml = String::new();
    file.read_to_string(&mut xml)
        .context("failed to read word/document.xml")?;
    Ok(parse_docx_xml(&xml))
}

fn parse_docx_xml(xml: &str) -> Vec<Block> {
    let mut reader = XmlReader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut blocks = Vec::new();
    let mut buf: Vec<u8> = Vec::new();
    let mut cur_text = String::new();
    let mut heading_level: Option<u8> = None;
    let mut in_t = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(e)) => match local_name(e.name().as_ref()) {
                b"p" => {
                    cur_text.clear();
                    heading_level = None;
                }
                b"t" => in_t = true,
                b"pStyle" => heading_level = heading_level.or_else(|| read_heading_val(&e)),
                _ => {}
            },
            Ok(XmlEvent::Empty(e)) => match local_name(e.name().as_ref()) {
                b"pStyle" => heading_level = heading_level.or_else(|| read_heading_val(&e)),
                b"tab" => cur_text.push(' '),
                b"br" => cur_text.push(' '),
                _ => {}
            },
            Ok(XmlEvent::End(e)) => match local_name(e.name().as_ref()) {
                b"p" => {
                    let cleaned = sanitize(cur_text.trim());
                    if !cleaned.is_empty() {
                        if let Some(lvl) = heading_level {
                            blocks.push(Block::Heading(lvl, cleaned));
                        } else {
                            blocks.push(Block::Text(cleaned));
                        }
                    }
                    cur_text.clear();
                    heading_level = None;
                }
                b"t" => in_t = false,
                _ => {}
            },
            Ok(XmlEvent::Text(t)) if in_t => {
                if let Ok(s) = t.unescape() {
                    cur_text.push_str(&s);
                }
            }
            Ok(XmlEvent::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    blocks
}

fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|&b| b == b':') {
        Some(i) => &name[i + 1..],
        None => name,
    }
}

fn read_heading_val(e: &quick_xml::events::BytesStart) -> Option<u8> {
    for attr in e.attributes().flatten() {
        if local_name(attr.key.as_ref()) == b"val"
            && let Ok(v) = std::str::from_utf8(&attr.value)
        {
            return parse_heading_style(v);
        }
    }
    None
}

fn parse_heading_style(s: &str) -> Option<u8> {
    let normalized: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect();
    if normalized == "title" {
        return Some(1);
    }
    if normalized == "subtitle" {
        return Some(2);
    }
    let digits = normalized.strip_prefix("heading")?;
    let n: u8 = digits.parse().ok()?;
    Some(n.clamp(1, 6))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_paragraph_becomes_text_block() {
        let blocks = parse_markdown("hello world");
        assert_eq!(blocks, vec![Block::Text("hello world".into())]);
    }

    #[test]
    fn heading_captures_level_and_text() {
        let blocks = parse_markdown("## Section title");
        assert_eq!(blocks, vec![Block::Heading(2, "Section title".into())]);
    }

    #[test]
    fn fenced_code_block_captured_as_code() {
        let md = "```\nlet x = 1;\nlet y = 2;\n```";
        let blocks = parse_markdown(md);
        assert_eq!(blocks, vec![Block::Code("let x = 1;\nlet y = 2;".into())]);
    }

    #[test]
    fn image_becomes_image_block_with_url() {
        let blocks = parse_markdown("![alt](https://example.com/a.png)");
        assert_eq!(
            blocks,
            vec![Block::Image("https://example.com/a.png".into())]
        );
    }

    #[test]
    fn mixed_document_preserves_block_order() {
        let md = "# Title\n\nfirst para\n\n```\ncode\n```\n\n![](img.png)\n\nlast para";
        let blocks = parse_markdown(md);
        assert_eq!(
            blocks,
            vec![
                Block::Heading(1, "Title".into()),
                Block::Text("first para".into()),
                Block::Code("code".into()),
                Block::Image("img.png".into()),
                Block::Text("last para".into()),
            ]
        );
    }

    #[test]
    fn image_alt_text_does_not_leak_into_text_block() {
        let blocks = parse_markdown("![do not include me](x.png)");
        assert_eq!(blocks, vec![Block::Image("x.png".into())]);
    }

    #[test]
    fn soft_break_joins_lines_with_space() {
        let blocks = parse_markdown("line one\nline two");
        assert_eq!(blocks, vec![Block::Text("line one line two".into())]);
    }

    #[test]
    fn empty_input_produces_no_blocks() {
        assert!(parse_markdown("").is_empty());
        assert!(parse_markdown("   \n\n  ").is_empty());
    }

    #[test]
    fn markdown_text_is_sanitized_of_zero_width_chars() {
        let blocks = parse_markdown("he\u{200B}llo wor\u{00AD}ld");
        assert_eq!(blocks, vec![Block::Text("hello world".into())]);
    }

    #[test]
    fn plain_text_splits_on_blank_lines() {
        let blocks = blocks_from_plain_text("first para\nline two\n\nsecond para\n\n\nthird");
        assert_eq!(
            blocks,
            vec![
                Block::Text("first para line two".into()),
                Block::Text("second para".into()),
                Block::Text("third".into()),
            ]
        );
    }

    #[test]
    fn plain_text_strips_zero_width_and_collapses_space() {
        let blocks = blocks_from_plain_text("hel\u{200B}lo    world");
        assert_eq!(blocks, vec![Block::Text("hello world".into())]);
    }

    #[test]
    fn heading_style_parsing_accepts_common_forms() {
        assert_eq!(parse_heading_style("Heading1"), Some(1));
        assert_eq!(parse_heading_style("heading 3"), Some(3));
        assert_eq!(parse_heading_style("Heading9"), Some(6)); // clamped
        assert_eq!(parse_heading_style("Title"), Some(1));
        assert_eq!(parse_heading_style("Subtitle"), Some(2));
        assert_eq!(parse_heading_style("Normal"), None);
    }

    #[test]
    fn docx_xml_extracts_paragraphs_and_heading() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="urn">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Big Title</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t xml:space="preserve">hello </w:t></w:r>
      <w:r><w:t>world</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading 2"/></w:pPr>
      <w:r><w:t>Sub</w:t></w:r>
    </w:p>
    <w:p><w:r><w:t>plain paragraph</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        let blocks = parse_docx_xml(xml);
        assert_eq!(
            blocks,
            vec![
                Block::Heading(1, "Big Title".into()),
                Block::Text("hello world".into()),
                Block::Heading(2, "Sub".into()),
                Block::Text("plain paragraph".into()),
            ]
        );
    }

    #[test]
    fn docx_xml_skips_empty_paragraphs() {
        let xml = r#"<w:document xmlns:w="urn"><w:body>
            <w:p></w:p>
            <w:p><w:r><w:t>   </w:t></w:r></w:p>
            <w:p><w:r><w:t>content</w:t></w:r></w:p>
        </w:body></w:document>"#;
        assert_eq!(parse_docx_xml(xml), vec![Block::Text("content".into())]);
    }
}
