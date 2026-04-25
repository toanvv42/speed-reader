use wasm_bindgen::prelude::*;

use serde::Deserialize;

use crate::doc::{
    Block, SectionStart, blocks_from_plain_text, parse_docx, parse_markdown, sections_from_blocks,
};
use crate::preset::Preset;
use crate::reader::{ChapterTarget, ChunkKind, Reader, chapter_targets};

#[wasm_bindgen]
pub struct WebReader {
    inner: Reader,
    chapters: Vec<ChapterTarget>,
}

#[derive(Deserialize)]
struct WebSectionInput {
    title: String,
    level: u8,
    block_index: usize,
}

#[wasm_bindgen]
impl WebReader {
    #[wasm_bindgen(constructor)]
    pub fn new(markdown: &str, last_index: Option<usize>) -> WebReader {
        let blocks = parse_markdown(markdown);
        Self::from_blocks(blocks, last_index)
    }

    /// Build a reader from a plain-text source (no markdown syntax).
    #[wasm_bindgen(js_name = fromText)]
    pub fn from_text(text: &str, last_index: Option<usize>) -> WebReader {
        let blocks = blocks_from_plain_text(text);
        Self::from_blocks_with_sections(blocks, Vec::new(), last_index)
    }

    /// Build a reader from plain text with precomputed section starts.
    #[wasm_bindgen(js_name = fromTextWithSections)]
    pub fn from_text_with_sections(
        text: &str,
        sections: JsValue,
        last_index: Option<usize>,
    ) -> Result<WebReader, JsError> {
        let raw_sections: Vec<WebSectionInput> = serde_wasm_bindgen::from_value(sections)
            .map_err(|e| JsError::new(&format!("invalid sections: {e}")))?;
        let blocks = blocks_from_plain_text(text);
        let sections = raw_sections
            .into_iter()
            .map(|section| SectionStart {
                title: section.title,
                level: section.level,
                block_index: section.block_index,
            })
            .collect();
        Ok(Self::from_blocks_with_sections(
            blocks, sections, last_index,
        ))
    }

    /// Build a reader from the raw bytes of a .docx file.
    #[wasm_bindgen(js_name = fromDocx)]
    pub fn from_docx(bytes: &[u8], last_index: Option<usize>) -> Result<WebReader, JsError> {
        let blocks = parse_docx(bytes).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self::from_blocks(blocks, last_index))
    }

    pub fn advance(&mut self) {
        self.inner.advance(1);
    }

    pub fn retreat(&mut self) {
        self.inner.retreat(1);
    }

    #[wasm_bindgen(js_name = setIndex)]
    pub fn set_index(&mut self, i: usize) {
        if i < self.inner.chunks.len() {
            self.inner.index = i;
        }
    }

    pub fn index(&self) -> usize {
        self.inner.index
    }

    pub fn total(&self) -> usize {
        self.inner.chunks.len()
    }

    #[wasm_bindgen(js_name = chapters)]
    pub fn chapters(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(&self.chapters)
            .map_err(|e| JsError::new(&format!("failed to serialize chapters: {e}")))
    }

    #[wasm_bindgen(js_name = atEnd)]
    pub fn at_end(&self) -> bool {
        self.inner.at_end()
    }

    pub fn text(&self) -> String {
        self.inner
            .current()
            .map(|c| c.text.clone())
            .unwrap_or_default()
    }

    pub fn orp(&self) -> usize {
        self.inner.current().map(|c| c.orp).unwrap_or(0)
    }

    pub fn kind(&self) -> String {
        self.inner
            .current()
            .map(|c| match c.kind {
                ChunkKind::Word => "word",
                ChunkKind::Heading => "heading",
                ChunkKind::Code => "code",
                ChunkKind::Paragraph => "paragraph",
                ChunkKind::Image => "image",
                ChunkKind::Table => "table",
            })
            .unwrap_or("word")
            .to_string()
    }

    #[wasm_bindgen(js_name = imageUrl)]
    pub fn image_url(&self) -> Option<String> {
        self.inner.current().and_then(|c| c.image_url.clone())
    }

    #[wasm_bindgen(js_name = tableData)]
    pub fn table_data(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(&self.inner.current().and_then(|c| c.table.clone()))
            .map_err(|e| JsError::new(&format!("failed to serialize table: {e}")))
    }

    #[wasm_bindgen(js_name = chunkDurationMs)]
    pub fn chunk_duration_ms(&self, wpm: u32, image_pause_ms: u32) -> u32 {
        let pause = std::time::Duration::from_millis(image_pause_ms as u64);
        self.inner.chunk_duration(wpm, pause).as_millis() as u32
    }

    #[wasm_bindgen(js_name = chunkDurationWithPresetMs)]
    pub fn chunk_duration_with_preset_ms(
        &self,
        wpm: u32,
        preset: &str,
        image_pause_ms: u32,
    ) -> u32 {
        let pause = std::time::Duration::from_millis(image_pause_ms as u64);
        let base = self.inner.chunk_duration(wpm, pause);
        let preset = preset.parse::<Preset>().unwrap_or(Preset::Standard);
        let multiplier = self
            .inner
            .current()
            .map(|chunk| preset.chunk_multiplier(chunk.kind))
            .unwrap_or(1.0);
        std::time::Duration::from_secs_f32((base.as_secs_f32() * multiplier).max(0.02)).as_millis()
            as u32
    }

    #[wasm_bindgen(js_name = remainingDurationMs)]
    pub fn remaining_duration_ms(&self, wpm: u32, image_pause_ms: u32) -> u32 {
        let pause = std::time::Duration::from_millis(image_pause_ms as u64);
        self.inner
            .remaining_duration_from(self.inner.index, wpm, pause)
            .as_millis() as u32
    }

    #[wasm_bindgen(js_name = remainingDurationWithPresetMs)]
    pub fn remaining_duration_with_preset_ms(
        &self,
        wpm: u32,
        preset: &str,
        image_pause_ms: u32,
    ) -> u32 {
        let pause = std::time::Duration::from_millis(image_pause_ms as u64);
        let preset = preset.parse::<Preset>().unwrap_or(Preset::Standard);
        self.inner.chunks[self.inner.index..]
            .iter()
            .map(|chunk| {
                let base = match chunk.kind {
                    ChunkKind::Image => pause,
                    ChunkKind::Code | ChunkKind::Table => self.inner.chunk_duration(wpm, pause),
                    _ => {
                        let base_ms = 60_000.0 / (wpm.max(1) as f32);
                        std::time::Duration::from_millis(
                            (base_ms * chunk.multiplier).max(20.0) as u64
                        )
                    }
                };
                std::time::Duration::from_secs_f32(
                    (base.as_secs_f32() * preset.chunk_multiplier(chunk.kind)).max(0.02),
                )
            })
            .fold(std::time::Duration::ZERO, |acc, d| acc.saturating_add(d))
            .as_millis() as u32
    }

    #[wasm_bindgen(js_name = presetDefaultWpm)]
    pub fn preset_default_wpm(preset: &str) -> u32 {
        preset
            .parse::<Preset>()
            .unwrap_or(Preset::Standard)
            .default_wpm()
    }

    #[wasm_bindgen(js_name = isPlaying)]
    pub fn is_playing(&self) -> bool {
        self.inner.playing
    }

    #[wasm_bindgen(js_name = setPlaying)]
    pub fn set_playing(&mut self, playing: bool) {
        self.inner.playing = playing;
    }

    #[wasm_bindgen(js_name = togglePlay)]
    pub fn toggle_play(&mut self) {
        if self.inner.chunks.is_empty() {
            return;
        }
        if self.inner.at_end() && !self.inner.playing {
            self.inner.index = 0;
        }
        self.inner.playing = !self.inner.playing;
    }
}

impl WebReader {
    fn from_blocks(blocks: Vec<Block>, last_index: Option<usize>) -> WebReader {
        let sections = sections_from_blocks(&blocks);
        Self::from_blocks_with_sections(blocks, sections, last_index)
    }

    fn from_blocks_with_sections(
        blocks: Vec<Block>,
        sections: Vec<SectionStart>,
        last_index: Option<usize>,
    ) -> WebReader {
        let chapters = chapter_targets(&blocks, &sections);
        let mut inner = Reader::from_blocks(blocks);
        if let Some(idx) = last_index {
            inner.index = idx.min(inner.chunks.len().saturating_sub(1));
        }
        WebReader { inner, chapters }
    }
}
