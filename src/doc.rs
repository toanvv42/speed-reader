#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use anyhow::{Context, Result};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

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
    let src = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let is_md = matches!(ext.as_str(), "md" | "markdown" | "mdx");
    if is_md {
        Ok(parse_markdown(&src))
    } else {
        Ok(vec![Block::Text(src)])
    }
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
                    if !s.trim().is_empty() {
                        blocks.push(Block::Heading(lvl, s.trim().to_string()));
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
    let s = buf.trim();
    if !s.is_empty() {
        blocks.push(Block::Text(s.to_string()));
    }
    buf.clear();
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
}
