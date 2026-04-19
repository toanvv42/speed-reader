use std::time::Duration;

use unicode_segmentation::UnicodeSegmentation;

use crate::doc::{Block, SectionStart};
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ChapterTarget {
    pub title: String,
    pub level: u8,
    pub chunk_index: usize,
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
        let (chunks, _) = tokenize_with_block_starts(&blocks);
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
    }

    pub fn retreat(&mut self, n: usize) {
        self.index = self.index.saturating_sub(n);
    }

    pub fn at_end(&self) -> bool {
        self.chunks.is_empty() || self.index + 1 >= self.chunks.len()
    }

    pub fn chunk_duration(&self, wpm: u32, image_pause: Duration) -> Duration {
        if let Some(chunk) = self.current() {
            match chunk.kind {
                ChunkKind::Image => return image_pause,
                ChunkKind::Code => return code_pause_duration(chunk, image_pause),
                _ => {}
            }
        }
        let base_ms = 60_000.0 / (wpm.max(1) as f32);
        let m = self.current().map(|c| c.multiplier).unwrap_or(1.0);
        Duration::from_millis((base_ms * m).max(20.0) as u64)
    }

    pub fn remaining_duration_from(
        &self,
        start: usize,
        wpm: u32,
        image_pause: Duration,
    ) -> Duration {
        if self.chunks.is_empty() || start >= self.chunks.len() {
            return Duration::ZERO;
        }

        self.chunks[start..]
            .iter()
            .map(|chunk| match chunk.kind {
                ChunkKind::Image => image_pause,
                ChunkKind::Code => code_pause_duration(chunk, image_pause),
                _ => {
                    let base_ms = 60_000.0 / (wpm.max(1) as f32);
                    Duration::from_millis((base_ms * chunk.multiplier).max(20.0) as u64)
                }
            })
            .fold(Duration::ZERO, |acc, d| acc.saturating_add(d))
    }
}

pub fn chapter_targets(blocks: &[Block], sections: &[SectionStart]) -> Vec<ChapterTarget> {
    let (_, block_starts) = tokenize_with_block_starts(blocks);
    sections
        .iter()
        .map(|section| ChapterTarget {
            title: section.title.clone(),
            level: section.level,
            chunk_index: block_starts
                .get(section.block_index)
                .copied()
                .unwrap_or_else(|| block_starts.last().copied().unwrap_or(0)),
        })
        .collect()
}

fn tokenize_with_block_starts(blocks: &[Block]) -> (Vec<Chunk>, Vec<usize>) {
    let mut out = Vec::new();
    let mut starts = Vec::with_capacity(blocks.len());
    for (i, block) in blocks.iter().enumerate() {
        starts.push(out.len());
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
            Block::Code(s) => out.push(Chunk {
                text: s.clone(),
                kind: ChunkKind::Code,
                multiplier: 1.0,
                orp: 0,
                image_url: None,
            }),
            Block::Image(url) => out.push(Chunk {
                text: truncate_end(url, 60),
                kind: ChunkKind::Image,
                multiplier: 3.0,
                orp: 0,
                image_url: Some(url.clone()),
            }),
        }
    }
    (out, starts)
}

fn tokenize_text(out: &mut Vec<Chunk>, text: &str, kind: ChunkKind, base_mult: f32) {
    for word in text.split_whitespace() {
        let mult = base_mult * punctuation_multiplier(word);
        let clean = word.trim_matches(|c| c == '.' || c == ',');
        if clean.is_empty() {
            continue;
        }

        let orp = orp_index(clean);
        out.push(Chunk {
            text: clean.to_string(),
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

fn code_pause_duration(chunk: &Chunk, image_pause: Duration) -> Duration {
    let lines = chunk.text.lines().count().max(1) as f32;
    let width = chunk
        .text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f32;
    let baseline = image_pause.as_secs_f32().max(3.0);
    let seconds = (baseline + (lines * 0.6) + (width / 48.0)).clamp(baseline, 12.0);
    Duration::from_secs_f32(seconds)
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
    fn tokenize_text_handles_long_punctuation() {
        let mut out = Vec::new();
        tokenize_text(&mut out, "hello .......... world", ChunkKind::Word, 1.0);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].text, "hello");
        assert_eq!(out[1].text, "world");
    }

    #[test]
    fn tokenize_text_strips_trailing_punctuation_but_keeps_multiplier() {
        let mut out = Vec::new();
        tokenize_text(&mut out, "Hello, world...", ChunkKind::Word, 1.0);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].text, "Hello");
        assert_eq!(out[0].multiplier, 1.5);
        assert_eq!(out[1].text, "world");
        assert_eq!(out[1].multiplier, 2.0);
    }

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
    fn code_blocks_are_single_chunks() {
        let r = Reader::from_blocks(vec![Block::Code(
            "fn main() {\n    println!(\"hi\");\n}".into(),
        )]);
        assert_eq!(r.chunks.len(), 1);
        assert_eq!(r.chunks[0].kind, ChunkKind::Code);
        assert_eq!(r.chunks[0].text, "fn main() {\n    println!(\"hi\");\n}");
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
        let pause = Duration::from_secs(3);
        let fast = r.chunk_duration(600, pause);
        let slow = r.chunk_duration(300, pause);
        assert!(slow > fast, "slower wpm must yield longer duration");
    }

    #[test]
    fn chunk_duration_has_minimum_floor() {
        let r = Reader::from_blocks(vec![Block::Text("word".into())]);
        let d = r.chunk_duration(10_000, Duration::from_secs(3));
        assert!(d >= Duration::from_millis(20));
    }

    #[test]
    fn image_chunk_uses_configured_pause_duration() {
        let mut r = Reader::from_blocks(vec![
            Block::Text("a".into()),
            Block::Image("file.png".into()),
        ]);
        // chunks: [Word("a"), Paragraph, Image]
        r.advance(2);
        assert_eq!(r.current().map(|c| c.kind), Some(ChunkKind::Image));
        let pause = Duration::from_millis(1234);
        assert_eq!(r.chunk_duration(300, pause), pause);
    }

    #[test]
    fn remaining_duration_sums_chunks_from_index() {
        let r = Reader::from_blocks(vec![Block::Text("one two three".into())]);
        let all = r.remaining_duration_from(0, 600, Duration::from_secs(3));
        let tail = r.remaining_duration_from(1, 600, Duration::from_secs(3));
        assert!(all > tail);
        assert_eq!(
            r.remaining_duration_from(999, 600, Duration::from_secs(3)),
            Duration::ZERO
        );
    }

    #[test]
    fn advance_onto_image_does_not_pause_playback() {
        let mut r = Reader::from_blocks(vec![
            Block::Text("a".into()),
            Block::Image("file.png".into()),
        ]);
        r.playing = true;
        r.advance(2);
        assert_eq!(r.current().map(|c| c.kind), Some(ChunkKind::Image));
        assert!(r.playing, "image should auto-advance, not force-pause");
    }

    #[test]
    fn chapter_targets_map_heading_blocks_to_chunk_positions() {
        let blocks = vec![
            Block::Heading(1, "Intro".into()),
            Block::Text("alpha beta".into()),
            Block::Heading(2, "Details".into()),
            Block::Text("gamma".into()),
        ];
        let sections = vec![
            SectionStart {
                title: "Intro".into(),
                level: 1,
                block_index: 0,
            },
            SectionStart {
                title: "Details".into(),
                level: 2,
                block_index: 2,
            },
        ];

        let targets = chapter_targets(&blocks, &sections);
        assert_eq!(targets[0].chunk_index, 0);
        assert_eq!(targets[1].chunk_index, 4);
    }

    #[test]
    fn chapter_targets_follow_tokenizer_rules_for_punctuation_only_words() {
        let blocks = vec![
            Block::Text("alpha .......... beta".into()),
            Block::Heading(1, "Gamma".into()),
        ];
        let sections = vec![SectionStart {
            title: "Gamma".into(),
            level: 1,
            block_index: 1,
        }];

        let targets = chapter_targets(&blocks, &sections);
        assert_eq!(targets[0].chunk_index, 2);
    }
}
