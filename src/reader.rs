use std::time::Duration;

use unicode_segmentation::UnicodeSegmentation;

use crate::doc::Block;
use crate::text::truncate_end;

pub struct Chunk {
    pub text: String,
    pub kind: ChunkKind,
    pub multiplier: f32,
    pub orp: usize,
    pub image_url: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
                text: truncate_end(url, 60),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orp_index_buckets() {
        assert_eq!(orp_index(""), 0);
        assert_eq!(orp_index("a"), 0);
        assert_eq!(orp_index("hi"), 1);
        assert_eq!(orp_index("hello"), 1);
        assert_eq!(orp_index("mornings"), 2);
        assert_eq!(orp_index("understand"), 3);
        assert_eq!(orp_index("supercalifragilistic"), 4);
    }

    #[test]
    fn from_blocks_produces_word_chunks_with_paragraph_separators() {
        let blocks = vec![
            Block::Text("hello world".into()),
            Block::Text("second para".into()),
        ];
        let r = Reader::from_blocks(blocks);
        let kinds: Vec<_> = r.chunks.iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ChunkKind::Word,
                ChunkKind::Word,
                ChunkKind::Paragraph,
                ChunkKind::Word,
                ChunkKind::Word,
            ]
        );
    }

    #[test]
    fn advance_stops_at_end_and_retreat_saturates_at_zero() {
        let r_blocks = vec![Block::Text("a b c".into())];
        let mut r = Reader::from_blocks(r_blocks);
        assert_eq!(r.index, 0);
        r.advance(100);
        assert_eq!(r.index, r.chunks.len() - 1);
        assert!(r.at_end());
        r.retreat(100);
        assert_eq!(r.index, 0);
    }

    #[test]
    fn empty_reader_reports_end_and_advance_is_noop() {
        let mut r = Reader::empty();
        assert!(r.at_end());
        r.advance(5);
        assert_eq!(r.index, 0);
    }

    #[test]
    fn chunk_duration_scales_inversely_with_wpm() {
        let r = Reader::from_blocks(vec![Block::Text("word".into())]);
        let fast = r.chunk_duration(600);
        let slow = r.chunk_duration(300);
        assert!(slow > fast, "slower wpm must yield longer duration");
    }

    #[test]
    fn chunk_duration_has_minimum_floor() {
        let r = Reader::from_blocks(vec![Block::Text("word".into())]);
        let d = r.chunk_duration(10_000);
        assert!(d >= Duration::from_millis(20));
    }

    #[test]
    fn image_block_produces_image_chunk_and_pauses_on_advance() {
        let mut r = Reader::from_blocks(vec![
            Block::Text("a".into()),
            Block::Image("file.png".into()),
        ]);
        r.playing = true;
        // chunks: [Word("a"), Paragraph, Image]
        r.advance(2);
        assert_eq!(r.current().map(|c| c.kind), Some(ChunkKind::Image));
        assert!(!r.playing, "landing on image should pause playback");
    }
}
