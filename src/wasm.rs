use wasm_bindgen::prelude::*;

use crate::doc::{blocks_from_plain_text, parse_docx, parse_markdown};
use crate::reader::{ChunkKind, Reader};

#[wasm_bindgen]
pub struct WebReader {
    inner: Reader,
}

#[wasm_bindgen]
impl WebReader {
    #[wasm_bindgen(constructor)]
    pub fn new(markdown: &str) -> WebReader {
        let blocks = parse_markdown(markdown);
        WebReader {
            inner: Reader::from_blocks(blocks),
        }
    }

    /// Build a reader from a plain-text source (no markdown syntax).
    #[wasm_bindgen(js_name = fromText)]
    pub fn from_text(text: &str) -> WebReader {
        WebReader {
            inner: Reader::from_blocks(blocks_from_plain_text(text)),
        }
    }

    /// Build a reader from the raw bytes of a .docx file.
    #[wasm_bindgen(js_name = fromDocx)]
    pub fn from_docx(bytes: &[u8]) -> Result<WebReader, JsError> {
        let blocks = parse_docx(bytes).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WebReader {
            inner: Reader::from_blocks(blocks),
        })
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
            })
            .unwrap_or("word")
            .to_string()
    }

    #[wasm_bindgen(js_name = imageUrl)]
    pub fn image_url(&self) -> Option<String> {
        self.inner.current().and_then(|c| c.image_url.clone())
    }

    #[wasm_bindgen(js_name = chunkDurationMs)]
    pub fn chunk_duration_ms(&self, wpm: u32, image_pause_ms: u32) -> u32 {
        let pause = std::time::Duration::from_millis(image_pause_ms as u64);
        self.inner.chunk_duration(wpm, pause).as_millis() as u32
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
