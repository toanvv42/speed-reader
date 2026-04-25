use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use ratatui_image::picker::Picker as ImagePicker;
use ratatui_image::protocol::StatefulProtocol;
use serde::{Deserialize, Serialize};

use speed_reader::doc::Block;
use speed_reader::preset::Preset;
use speed_reader::reader::{ChapterTarget, ChunkKind, Reader};
use speed_reader::theme::{Theme, ThemeChoice};

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct State {
    // Map of absolute file path to last read index
    locations: HashMap<String, usize>,
    recent_paths: Vec<String>,
}

struct ImageJob {
    url: String,
    bytes: Result<Vec<u8>>,
}

pub enum Action {
    Quit,
    TogglePlay,
    NextWord,
    PrevWord,
    WpmUp,
    WpmDown,
    OpenPicker,
    ClosePicker,
    OpenRecent(usize),
    CyclePreset,
    OpenChapterPicker,
    CloseChapterPicker,
    ToggleHelp,
    CycleTheme,
    PickerInput(char),
    PickerBackspace,
    PickerUp,
    PickerDown,
    PickerConfirm,
    ChapterInput(char),
    ChapterBackspace,
    ChapterUp,
    ChapterDown,
    ChapterConfirm,
}

#[derive(PartialEq, Eq)]
pub enum Mode {
    Reading,
    Picker,
    ChapterPicker,
    Help,
}

pub struct App {
    pub should_quit: bool,
    pub mode: Mode,
    pub reader: Reader,
    pub wpm: u32,
    pub file_path: Option<PathBuf>,
    pub picker: FilePicker,
    pub chapters: Vec<ChapterTarget>,
    pub chapter_picker: ChapterPicker,
    last_tick: Instant,
    pub status_msg: Option<String>,
    image_picker: ImagePicker,
    pub image_cache: HashMap<String, StatefulProtocol>,
    pub pending_image_urls: HashSet<String>,
    failed_count: usize,
    http_agent: ureq::Agent,
    image_tx: mpsc::Sender<ImageJob>,
    image_rx: mpsc::Receiver<ImageJob>,
    pub theme_choice: ThemeChoice,
    pub theme: Theme,
    pub preset: Preset,
    pub image_pause: Duration,
    state: State,
}

pub struct FilePicker {
    pub query: String,
    pub entries: Vec<PathBuf>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub cwd: PathBuf,
}

pub struct ChapterPicker {
    pub query: String,
    pub filtered: Vec<usize>,
    pub selected: usize,
}

pub struct RecentFile {
    pub path: PathBuf,
    pub index: usize,
}

impl App {
    pub fn new(
        image_picker: ImagePicker,
        theme_choice: ThemeChoice,
        theme: Theme,
        image_pause: Duration,
    ) -> Self {
        let http_agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(15))
            .build();
        let (image_tx, image_rx) = mpsc::channel();
        let state = Self::load_state().unwrap_or_default();
        Self {
            should_quit: false,
            mode: Mode::Reading,
            reader: Reader::empty(),
            wpm: 300,
            file_path: None,
            picker: FilePicker::new(),
            chapters: Vec::new(),
            chapter_picker: ChapterPicker::new(),
            last_tick: Instant::now(),
            status_msg: None,
            image_picker,
            image_cache: HashMap::new(),
            pending_image_urls: HashSet::new(),
            failed_count: 0,
            http_agent,
            image_tx,
            image_rx,
            theme_choice,
            theme,
            preset: Preset::Standard,
            image_pause,
            state,
        }
    }

    fn state_path() -> Option<PathBuf> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let home = std::env::var("HOME")
                .ok()
                .or_else(|| std::env::var("USERPROFILE").ok())?;
            Some(PathBuf::from(home).join(".speed-reader-state.json"))
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }

    fn load_state() -> Result<State> {
        let path = match Self::state_path() {
            Some(p) => p,
            None => return Ok(State::default()),
        };
        if !path.exists() {
            return Ok(State::default());
        }
        let f = std::fs::File::open(path)?;
        let state = serde_json::from_reader(f)?;
        Ok(state)
    }

    fn save_state(&self) -> Result<()> {
        let path = match Self::state_path() {
            Some(p) => p,
            None => return Ok(()),
        };
        let f = std::fs::File::create(path)?;
        serde_json::to_writer(f, &self.state)?;
        Ok(())
    }

    fn record_current_location(&mut self) {
        if let Some(path) = &self.file_path
            && let Ok(abs) = std::fs::canonicalize(path)
            && abs.to_str().is_some()
        {
            self.record_location_for(&abs, self.reader.index);
        }
    }

    fn record_location_for(&mut self, path: &Path, index: usize) {
        if let Some(s) = path.to_str() {
            let key = s.to_string();
            self.state.locations.insert(key.clone(), index);
            self.promote_recent_path(key);
        }
    }

    fn promote_recent_path(&mut self, key: String) {
        self.state.recent_paths.retain(|p| p != &key);
        self.state.recent_paths.insert(0, key);
        self.state.recent_paths.truncate(12);
    }

    pub fn recent_files(&self, limit: usize) -> Vec<RecentFile> {
        self.state
            .recent_paths
            .iter()
            .filter_map(|path| {
                let path_buf = PathBuf::from(path);
                if !path_buf.exists() || !path_buf.is_file() {
                    return None;
                }
                Some(RecentFile {
                    path: path_buf,
                    index: *self.state.locations.get(path).unwrap_or(&0),
                })
            })
            .take(limit)
            .collect()
    }

    pub fn current_chunk_duration(&self) -> Duration {
        let base = self.reader.chunk_duration(self.wpm, self.image_pause);
        let multiplier = self
            .reader
            .current()
            .map(|chunk| self.preset.chunk_multiplier(chunk.kind))
            .unwrap_or(1.0);
        Duration::from_secs_f32((base.as_secs_f32() * multiplier).max(0.02))
    }

    pub fn remaining_duration_from(&self, start: usize) -> Duration {
        if self.reader.chunks.is_empty() || start >= self.reader.chunks.len() {
            return Duration::ZERO;
        }

        self.reader.chunks[start..]
            .iter()
            .map(|chunk| {
                let base = match chunk.kind {
                    ChunkKind::Image => self.image_pause,
                    ChunkKind::Code => self.reader.chunk_duration(self.wpm, self.image_pause),
                    _ => {
                        let base_ms = 60_000.0 / (self.wpm.max(1) as f32);
                        Duration::from_millis((base_ms * chunk.multiplier).max(20.0) as u64)
                    }
                };
                Duration::from_secs_f32(
                    (base.as_secs_f32() * self.preset.chunk_multiplier(chunk.kind)).max(0.02),
                )
            })
            .fold(Duration::ZERO, |acc, d| acc.saturating_add(d))
    }

    pub fn current_chapter(&self) -> Option<&ChapterTarget> {
        let mut active = None;
        for chapter in &self.chapters {
            if chapter.chunk_index <= self.reader.index {
                active = Some(chapter);
            } else {
                break;
            }
        }
        active
    }

    pub fn chapter_progress(&self) -> Option<(usize, usize)> {
        let current = self.current_chapter()?;
        let start = current.chunk_index;
        let end = self
            .chapters
            .iter()
            .find(|chapter| chapter.chunk_index > self.reader.index)
            .map(|chapter| chapter.chunk_index)
            .unwrap_or(self.reader.chunks.len());
        let current_pos = self.reader.index.saturating_sub(start) + 1;
        let total = end.saturating_sub(start).max(1);
        Some((current_pos.min(total), total))
    }

    pub fn surrounding_words(&self) -> (Option<&str>, Option<&str>) {
        let prev = self
            .reader
            .chunks
            .get(self.reader.index.saturating_sub(1))
            .filter(|chunk| matches!(chunk.kind, ChunkKind::Word | ChunkKind::Heading))
            .map(|chunk| chunk.text.as_str());
        let next = self
            .reader
            .chunks
            .get(self.reader.index.saturating_add(1))
            .filter(|chunk| matches!(chunk.kind, ChunkKind::Word | ChunkKind::Heading))
            .map(|chunk| chunk.text.as_str());
        (prev, next)
    }

    pub fn open_path(&mut self, path: &Path) -> Result<()> {
        self.record_current_location();
        let _ = self.save_state();

        let doc = speed_reader::doc::load(path)?;
        let canonical = std::fs::canonicalize(path).ok();
        let base_dir = path.parent().map(|p| p.to_path_buf());

        self.image_cache.clear();
        self.pending_image_urls.clear();
        self.failed_count = 0;

        let mut seen: HashSet<String> = HashSet::new();
        for block in &doc.blocks {
            if let Block::Image(url) = block {
                if !seen.insert(url.clone()) {
                    continue;
                }
                self.pending_image_urls.insert(url.clone());
                let tx = self.image_tx.clone();
                let agent = self.http_agent.clone();
                let base = base_dir.clone();
                let url_s = url.clone();
                std::thread::spawn(move || {
                    let bytes = fetch_bytes(&agent, &url_s, base.as_deref());
                    let _ = tx.send(ImageJob { url: url_s, bytes });
                });
            }
        }

        self.chapters = speed_reader::reader::chapter_targets(&doc.blocks, &doc.sections);
        self.chapter_picker.reset(self.chapters.len());
        self.reader = Reader::from_blocks(doc.blocks);
        self.file_path = Some(path.to_path_buf());

        // Restore location
        if let Some(abs) = canonical.as_ref()
            && let Some(s) = abs.to_str()
            && let Some(&idx) = self.state.locations.get(s)
        {
            self.reader.index = idx.min(self.reader.chunks.len().saturating_sub(1));
        }

        if let Some(abs) = canonical.as_ref() {
            self.record_location_for(abs, self.reader.index);
            let _ = self.save_state();
        }

        self.mode = Mode::Reading;
        self.last_tick = Instant::now();
        self.update_image_status();
        Ok(())
    }

    pub fn pump_images(&mut self) {
        let mut updated = false;
        while let Ok(job) = self.image_rx.try_recv() {
            self.pending_image_urls.remove(&job.url);
            updated = true;
            match job
                .bytes
                .and_then(|b| image::load_from_memory(&b).context("failed to decode image"))
            {
                Ok(img) => {
                    let proto = self.image_picker.new_resize_protocol(img);
                    self.image_cache.insert(job.url, proto);
                }
                Err(_) => self.failed_count += 1,
            }
        }
        if updated {
            self.update_image_status();
        }
    }

    fn update_image_status(&mut self) {
        let pending = self.pending_image_urls.len();
        let failed = self.failed_count;
        self.status_msg = if pending > 0 {
            Some(format!("loading {} image(s)…", pending))
        } else if failed > 0 {
            Some(format!("{} image(s) failed to load", failed))
        } else {
            None
        };
    }

    pub fn tick_timeout(&self) -> Duration {
        let idle = if !self.pending_image_urls.is_empty() {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(250)
        };
        if self.mode != Mode::Reading || !self.reader.playing {
            return idle;
        }
        let per_chunk = self.current_chunk_duration();
        let elapsed = self.last_tick.elapsed();
        per_chunk
            .saturating_sub(elapsed)
            .max(Duration::from_millis(5))
            .min(idle)
    }

    pub fn reading_time_remaining(&self) -> Duration {
        if self.reader.chunks.is_empty() || self.reader.at_end() {
            return Duration::ZERO;
        }

        let total = self.remaining_duration_from(self.reader.index);

        if self.mode == Mode::Reading && self.reader.playing {
            let elapsed_on_chunk = self.last_tick.elapsed();
            total.saturating_sub(elapsed_on_chunk)
        } else {
            total
        }
    }

    pub fn tick(&mut self) {
        if self.mode != Mode::Reading || !self.reader.playing {
            return;
        }
        let per_chunk = self.current_chunk_duration();
        if self.last_tick.elapsed() >= per_chunk {
            if self.reader.at_end() {
                self.reader.playing = false;
            } else {
                self.reader.advance(1);
                self.last_tick = Instant::now();
            }
        }
    }

    pub fn handle(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Quit => {
                self.record_current_location();
                let _ = self.save_state();
                self.should_quit = true;
            }
            Action::TogglePlay => {
                if !self.reader.chunks.is_empty() {
                    if self.reader.at_end() && !self.reader.playing {
                        self.reader.index = 0;
                    }
                    self.reader.playing = !self.reader.playing;
                    self.last_tick = Instant::now();
                }
            }
            Action::NextWord => {
                self.reader.advance(1);
                self.last_tick = Instant::now();
            }
            Action::PrevWord => {
                self.reader.retreat(1);
                self.last_tick = Instant::now();
            }
            Action::WpmUp => self.wpm = (self.wpm + 25).min(1500),
            Action::WpmDown => self.wpm = self.wpm.saturating_sub(25).max(50),
            Action::OpenPicker => {
                self.reader.playing = false;
                if let Err(e) = self.picker.refresh() {
                    self.status_msg = Some(format!("picker: {}", e));
                    return Ok(());
                }
                self.mode = Mode::Picker;
            }
            Action::ClosePicker => {
                self.mode = Mode::Reading;
            }
            Action::OpenRecent(i) => {
                if let Some(recent) = self.recent_files(i + 1).into_iter().nth(i) {
                    match self.open_path(&recent.path) {
                        Ok(()) => {
                            self.status_msg = Some(format!("resumed: {}", recent.path.display()));
                        }
                        Err(e) => {
                            self.status_msg = Some(format!("open failed: {}", e));
                        }
                    }
                }
            }
            Action::CyclePreset => {
                self.preset = self.preset.cycle();
                self.wpm = self.preset.default_wpm();
                self.status_msg = Some(format!(
                    "preset: {} ({} wpm)",
                    self.preset.label(),
                    self.wpm
                ));
            }
            Action::OpenChapterPicker => {
                self.reader.playing = false;
                if self.chapters.is_empty() {
                    self.status_msg = Some("no chapters found".into());
                } else {
                    self.chapter_picker.reset(self.chapters.len());
                    self.mode = Mode::ChapterPicker;
                }
            }
            Action::CloseChapterPicker => {
                self.mode = Mode::Reading;
            }
            Action::ToggleHelp => {
                self.mode = match self.mode {
                    Mode::Help => Mode::Reading,
                    _ => Mode::Help,
                };
            }
            Action::CycleTheme => {
                self.theme_choice = self.theme_choice.cycle();
                self.theme = speed_reader::theme::resolve(self.theme_choice);
                self.status_msg = Some(format!("theme: {}", self.theme_choice.label()));
            }
            Action::PickerInput(c) => {
                self.picker.query.push(c);
                self.picker.refilter();
            }
            Action::PickerBackspace => {
                self.picker.query.pop();
                self.picker.refilter();
            }
            Action::PickerUp => self.picker.move_up(),
            Action::PickerDown => self.picker.move_down(),
            Action::PickerConfirm => {
                if let Some(path) = self.picker.selected_path() {
                    if path.is_dir() {
                        self.picker.cwd = path;
                        self.picker.query.clear();
                        self.picker.refresh()?;
                    } else {
                        match self.open_path(&path) {
                            Ok(()) => {}
                            Err(e) => {
                                self.status_msg = Some(format!("open failed: {}", e));
                            }
                        }
                    }
                }
            }
            Action::ChapterInput(c) => {
                self.chapter_picker.query.push(c);
                self.chapter_picker.refilter(&self.chapters);
            }
            Action::ChapterBackspace => {
                self.chapter_picker.query.pop();
                self.chapter_picker.refilter(&self.chapters);
            }
            Action::ChapterUp => self.chapter_picker.move_up(),
            Action::ChapterDown => self.chapter_picker.move_down(),
            Action::ChapterConfirm => {
                if let Some(target) = self.chapter_picker.selected_target(&self.chapters) {
                    self.reader.index = target
                        .chunk_index
                        .min(self.reader.chunks.len().saturating_sub(1));
                    self.last_tick = Instant::now();
                    self.mode = Mode::Reading;
                    self.status_msg = Some(format!("chapter: {}", target.title));
                }
            }
        }
        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.record_current_location();
        let _ = self.save_state();
    }
}

impl RecentFile {
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string()
    }

    pub fn parent_display(&self) -> String {
        self.path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "—".to_string())
    }
}

fn fetch_bytes(agent: &ureq::Agent, src: &str, base_dir: Option<&Path>) -> Result<Vec<u8>> {
    if src.starts_with("http://") || src.starts_with("https://") {
        let resp = agent
            .get(src)
            .call()
            .with_context(|| format!("GET {}", src))?;
        let mut bytes = Vec::with_capacity(64 * 1024);
        resp.into_reader()
            .take(16 * 1024 * 1024)
            .read_to_end(&mut bytes)
            .context("read image body")?;
        Ok(bytes)
    } else {
        let candidate = Path::new(src);
        let path = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else if let Some(dir) = base_dir {
            dir.join(candidate)
        } else {
            candidate.to_path_buf()
        };
        std::fs::read(&path).with_context(|| format!("read {}", path.display()))
    }
}

impl FilePicker {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            query: String::new(),
            entries: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            cwd,
        }
    }

    pub fn refresh(&mut self) -> Result<()> {
        self.entries.clear();
        if let Some(parent) = self.cwd.parent() {
            self.entries.push(parent.to_path_buf());
        }
        let mut read: Vec<PathBuf> = std::fs::read_dir(&self.cwd)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| !n.starts_with('.'))
                    .unwrap_or(false)
            })
            .collect();
        read.sort_by(|a, b| {
            let ad = a.is_dir();
            let bd = b.is_dir();
            bd.cmp(&ad).then_with(|| a.file_name().cmp(&b.file_name()))
        });
        self.entries.extend(read);
        self.refilter();
        Ok(())
    }

    pub fn refilter(&mut self) {
        let q = self.query.to_lowercase();
        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                if q.is_empty() {
                    return true;
                }
                if Some(p.as_path()) == self.cwd.parent() {
                    return true;
                }
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.to_lowercase().contains(&q))
                    .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.filtered
            .get(self.selected)
            .and_then(|&i| self.entries.get(i).cloned())
    }

    pub fn is_parent(&self, p: &Path) -> bool {
        self.cwd.parent() == Some(p)
    }
}

impl ChapterPicker {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            filtered: Vec::new(),
            selected: 0,
        }
    }

    pub fn reset(&mut self, len: usize) {
        self.query.clear();
        self.filtered = (0..len).collect();
        self.selected = 0;
    }

    pub fn refilter(&mut self, chapters: &[ChapterTarget]) {
        let query = self.query.to_lowercase();
        self.filtered = chapters
            .iter()
            .enumerate()
            .filter(|(_, chapter)| {
                query.is_empty() || chapter.title.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn selected_target<'a>(&self, chapters: &'a [ChapterTarget]) -> Option<&'a ChapterTarget> {
        self.filtered
            .get(self.selected)
            .and_then(|&index| chapters.get(index))
    }
}
