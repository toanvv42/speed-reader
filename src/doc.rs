#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use std::collections::{HashMap, HashSet};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionStart {
    pub title: String,
    pub level: u8,
    pub block_index: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
    pub sections: Vec<SectionStart>,
}

impl Document {
    pub fn from_blocks(blocks: Vec<Block>) -> Self {
        let sections = sections_from_blocks(&blocks);
        Self { blocks, sections }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load(path: &Path) -> Result<Document> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "md" | "markdown" | "mdx" => {
            let src = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            Ok(Document::from_blocks(parse_markdown(&src)))
        }
        "docx" => {
            let bytes = std::fs::read(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let blocks = parse_docx(&bytes)?;
            Ok(Document::from_blocks(blocks))
        }
        "pdf" => {
            let bytes = std::fs::read(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            parse_pdf(&bytes).with_context(|| format!("failed to parse {}", path.display()))
        }
        _ => {
            let src = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            Ok(Document::from_blocks(blocks_from_plain_text(&src)))
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

pub fn sections_from_blocks(blocks: &[Block]) -> Vec<SectionStart> {
    blocks
        .iter()
        .enumerate()
        .filter_map(|(block_index, block)| match block {
            Block::Heading(level, title) => Some(SectionStart {
                title: title.clone(),
                level: *level,
                block_index,
            }),
            _ => None,
        })
        .collect()
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

#[cfg(not(target_arch = "wasm32"))]
fn parse_pdf(bytes: &[u8]) -> Result<Document> {
    let pages = pdf_extract::extract_text_from_mem_by_pages(bytes)
        .context("failed to extract pdf pages")?;
    let (blocks, page_block_starts) = blocks_from_pdf_pages(&pages);
    let mut sections = pdf_outline_sections(bytes, &page_block_starts, blocks.len())
        .context("failed to parse pdf outline")?;
    if sections.is_empty() {
        sections = infer_pdf_sections(&pages, &page_block_starts, blocks.len());
    }
    Ok(Document { blocks, sections })
}

#[cfg(not(target_arch = "wasm32"))]
fn blocks_from_pdf_pages(pages: &[String]) -> (Vec<Block>, Vec<usize>) {
    let mut blocks = Vec::new();
    let mut page_block_starts = Vec::with_capacity(pages.len());
    for page in pages {
        page_block_starts.push(blocks.len());
        blocks.extend(blocks_from_plain_text(page));
    }
    (blocks, page_block_starts)
}

#[cfg(not(target_arch = "wasm32"))]
fn pdf_outline_sections(
    bytes: &[u8],
    page_block_starts: &[usize],
    total_blocks: usize,
) -> Result<Vec<SectionStart>> {
    let doc = pdf_extract::Document::load_mem(bytes)?;
    let page_numbers: HashMap<_, _> = doc
        .get_pages()
        .into_iter()
        .map(|(page_number, object_id)| (object_id, page_number))
        .collect();

    let catalog = doc.catalog()?;
    let outlines_id = match catalog
        .get(b"Outlines")
        .and_then(pdf_extract::Object::as_reference)
    {
        Ok(id) => id,
        Err(_) => return Ok(Vec::new()),
    };
    let outlines = doc.get_dictionary(outlines_id)?;
    let first_id = match outlines
        .get(b"First")
        .and_then(pdf_extract::Object::as_reference)
    {
        Ok(id) => id,
        Err(_) => return Ok(Vec::new()),
    };

    let mut sections = Vec::new();
    collect_outline_sections(
        &doc,
        first_id,
        1,
        &page_numbers,
        page_block_starts,
        total_blocks,
        &mut sections,
        &mut HashSet::new(),
    )?;
    Ok(sections)
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_outline_sections(
    doc: &pdf_extract::Document,
    first_id: pdf_extract::ObjectId,
    level: u8,
    page_numbers: &HashMap<pdf_extract::ObjectId, u32>,
    page_block_starts: &[usize],
    total_blocks: usize,
    sections: &mut Vec<SectionStart>,
    visited: &mut HashSet<pdf_extract::ObjectId>,
) -> Result<()> {
    let mut current = Some(first_id);
    while let Some(item_id) = current {
        if !visited.insert(item_id) {
            break;
        }
        let item = doc.get_dictionary(item_id)?;
        if let Some(title) = outline_title(item)
            && let Some(page_number) = outline_page_number(doc, item, page_numbers)
        {
            sections.push(SectionStart {
                title,
                level,
                block_index: page_to_block_index(page_number, page_block_starts, total_blocks),
            });
        }

        if let Ok(child_id) = item
            .get(b"First")
            .and_then(pdf_extract::Object::as_reference)
        {
            collect_outline_sections(
                doc,
                child_id,
                level.saturating_add(1),
                page_numbers,
                page_block_starts,
                total_blocks,
                sections,
                visited,
            )?;
        }

        current = item
            .get(b"Next")
            .and_then(pdf_extract::Object::as_reference)
            .ok();
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn outline_title(item: &pdf_extract::Dictionary) -> Option<String> {
    match item.get(b"Title").ok()? {
        pdf_extract::Object::String(bytes, _) => {
            let title = decode_pdf_string(bytes);
            (!title.is_empty()).then_some(title)
        }
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn outline_page_number(
    doc: &pdf_extract::Document,
    item: &pdf_extract::Dictionary,
    page_numbers: &HashMap<pdf_extract::ObjectId, u32>,
) -> Option<u32> {
    if let Ok(dest) = item.get(b"Dest") {
        return resolve_destination_page(doc, dest, page_numbers);
    }

    let action = match item.get(b"A").ok()? {
        pdf_extract::Object::Reference(id) => doc.get_dictionary(*id).ok()?,
        pdf_extract::Object::Dictionary(dict) => dict,
        _ => return None,
    };
    resolve_destination_page(doc, action.get(b"D").ok()?, page_numbers)
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_destination_page(
    doc: &pdf_extract::Document,
    dest: &pdf_extract::Object,
    page_numbers: &HashMap<pdf_extract::ObjectId, u32>,
) -> Option<u32> {
    match dest {
        pdf_extract::Object::Array(items) => match items.first()? {
            pdf_extract::Object::Reference(id) => page_numbers.get(id).copied(),
            _ => None,
        },
        pdf_extract::Object::Reference(id) => {
            let obj = doc.get_object(*id).ok()?;
            resolve_destination_page(doc, obj, page_numbers)
        }
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn page_to_block_index(
    page_number: u32,
    page_block_starts: &[usize],
    total_blocks: usize,
) -> usize {
    if total_blocks == 0 {
        return 0;
    }
    let start = page_block_starts
        .get(page_number.saturating_sub(1) as usize)
        .copied()
        .unwrap_or(0);
    start.min(total_blocks.saturating_sub(1))
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return decode_utf16_be(&bytes[2..]);
    }
    if bytes.len() >= 2 && bytes.len() % 2 == 0 && bytes.iter().step_by(2).all(|b| *b == 0) {
        return decode_utf16_be(bytes);
    }
    String::from_utf8_lossy(bytes)
        .replace('\0', "")
        .trim()
        .to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_utf16_be(bytes: &[u8]) -> String {
    let units = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]));
    char::decode_utf16(units)
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn infer_pdf_sections(
    pages: &[String],
    page_block_starts: &[usize],
    total_blocks: usize,
) -> Vec<SectionStart> {
    let mut sections = Vec::new();
    for (page_index, page) in pages.iter().enumerate() {
        let Some(title) = infer_pdf_section_title(page) else {
            continue;
        };
        sections.push(SectionStart {
            title,
            level: 1,
            block_index: page_to_block_index(
                (page_index + 1) as u32,
                page_block_starts,
                total_blocks,
            ),
        });
    }
    sections
}

#[cfg(not(target_arch = "wasm32"))]
fn infer_pdf_section_title(page: &str) -> Option<String> {
    let lines: Vec<&str> = page
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(8)
        .collect();
    for line in lines {
        if looks_like_pdf_section_heading(line) {
            return Some(sanitize(line));
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn looks_like_pdf_section_heading(line: &str) -> bool {
    let word_count = line.split_whitespace().count();
    if word_count == 0 || word_count > 10 {
        return false;
    }

    let lower = line.to_lowercase();
    if lower.starts_with("chapter ") || lower.starts_with("part ") {
        return true;
    }

    let Some((prefix, rest)) = line.split_once('.') else {
        return false;
    };
    let has_numeric_prefix = prefix.chars().all(|c| c.is_ascii_digit())
        || prefix
            .chars()
            .all(|c| matches!(c, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'));
    has_numeric_prefix
        && !rest.trim().is_empty()
        && rest
            .trim()
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
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
    fn document_from_blocks_collects_heading_sections() {
        let doc = Document::from_blocks(vec![
            Block::Heading(1, "One".into()),
            Block::Text("alpha".into()),
            Block::Heading(2, "Two".into()),
        ]);
        assert_eq!(
            doc.sections,
            vec![
                SectionStart {
                    title: "One".into(),
                    level: 1,
                    block_index: 0,
                },
                SectionStart {
                    title: "Two".into(),
                    level: 2,
                    block_index: 2,
                },
            ]
        );
    }

    #[test]
    fn decode_pdf_string_handles_utf16_with_bom() {
        assert_eq!(
            decode_pdf_string(&[0xFE, 0xFF, 0x00, b'T', 0x00, b'e', 0x00, b's', 0x00, b't']),
            "Test"
        );
    }

    #[test]
    fn infer_pdf_section_title_prefers_heading_like_lines() {
        let page = "\n  2. B-Tree Basics\nsome body text\n";
        assert_eq!(
            infer_pdf_section_title(page).as_deref(),
            Some("2. B-Tree Basics")
        );
    }

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
