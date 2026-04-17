use std::time::Duration;

use unicode_segmentation::UnicodeSegmentation;

use crate::doc::Block;

pub struct Chunk {
    pub text: String,
    pub kind: ChunkKind,
    pub multiplier: f32,
    pub orp: usize,
    pub image_url: Option<String>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ChunkKind {
    Word,
    Heading,
    Code,
    Paragraph,
    Image,
}

pub struct Reader {
    pub chunks: Vec<Chunk>,
    pub index: usize,
    pub playing: bool,
}

impl Reader {
    pub fn empty() -> Self {
        Self {
            chunks: Vec::new(),
            index: 0,
            playing: false,
        }
    }

    pub fn from_blocks(blocks: Vec<Block>) -> Self {
        let chunks = tokenize(&blocks);
        Self {
            chunks,
            index: 0,
            playing: false,
        }
    }

    pub fn current(&self) -> Option<&Chunk> {
        self.chunks.get(self.index)
    }

    pub fn advance(&mut self, n: usize) {
        if self.chunks.is_empty() {
            return;
        }
        self.index = (self.index + n).min(self.chunks.len() - 1);
        if self.current().map(|c| c.kind) == Some(ChunkKind::Image) {
            // give the reader a beat to look at the picture
            self.playing = false;
        }
    }

    pub fn retreat(&mut self, n: usize) {
        self.index = self.index.saturating_sub(n);
    }

    pub fn at_end(&self) -> bool {
        self.chunks.is_empty() || self.index + 1 >= self.chunks.len()
    }

    pub fn chunk_duration(&self, wpm: u32) -> Duration {
        let base_ms = 60_000.0 / (wpm.max(1) as f32);
        let m = self.current().map(|c| c.multiplier).unwrap_or(1.0);
        Duration::from_millis((base_ms * m).max(20.0) as u64)
    }
}

fn tokenize(blocks: &[Block]) -> Vec<Chunk> {
    let mut out = Vec::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            out.push(Chunk {
                text: String::new(),
                kind: ChunkKind::Paragraph,
                multiplier: 1.5,
                orp: 0,
                image_url: None,
            });
        }
        match block {
            Block::Text(s) => tokenize_text(&mut out, s, ChunkKind::Word, 1.0),
            Block::Heading(_, s) => tokenize_text(&mut out, s, ChunkKind::Heading, 1.3),
            Block::Code(s) => tokenize_text(&mut out, s, ChunkKind::Code, 1.2),
            Block::Image(url) => out.push(Chunk {
                text: truncate(url, 60),
                kind: ChunkKind::Image,
                multiplier: 3.0,
                orp: 0,
                image_url: Some(url.clone()),
            }),
        }
    }
    out
}

fn tokenize_text(out: &mut Vec<Chunk>, text: &str, kind: ChunkKind, base_mult: f32) {
    for word in text.split_whitespace() {
        let mult = base_mult * punctuation_multiplier(word);
        let orp = orp_index(word);
        out.push(Chunk {
            text: word.to_string(),
            kind,
            multiplier: mult,
            orp,
            image_url: None,
        });
    }
}

fn punctuation_multiplier(word: &str) -> f32 {
    match word.chars().last() {
        Some('.') | Some('!') | Some('?') => 2.0,
        Some(',') | Some(';') | Some(':') => 1.5,
        _ => 1.0,
    }
}

pub fn orp_index(word: &str) -> usize {
    let graphemes = word.graphemes(true).count();
    match graphemes {
        0 | 1 => 0,
        2..=5 => 1,
        6..=9 => 2,
        10..=13 => 3,
        _ => 4,
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{}…", t)
    }
}
