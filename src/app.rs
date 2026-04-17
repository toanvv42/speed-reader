use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use ratatui_image::picker::Picker as ImagePicker;
use ratatui_image::protocol::StatefulProtocol;

use crate::doc::Block;
use crate::reader::Reader;
use crate::theme::{Theme, ThemeChoice};

pub enum Action {
    Quit,
    TogglePlay,
    NextWord,
    PrevWord,
    WpmUp,
    WpmDown,
    OpenPicker,
    ClosePicker,
    ToggleHelp,
    CycleTheme,
    PickerInput(char),
    PickerBackspace,
    PickerUp,
    PickerDown,
    PickerConfirm,
}

#[derive(PartialEq, Eq)]
pub enum Mode {
    Reading,
    Picker,
    Help,
}

pub struct App {
    pub should_quit: bool,
    pub mode: Mode,
    pub reader: Reader,
    pub wpm: u32,
    pub file_path: Option<PathBuf>,
    pub picker: FilePicker,
    last_tick: Instant,
    pub status_msg: Option<String>,
    image_picker: ImagePicker,
    pub image_cache: HashMap<String, StatefulProtocol>,
    pub theme_choice: ThemeChoice,
    pub theme: Theme,
}

pub struct FilePicker {
    pub query: String,
    pub entries: Vec<PathBuf>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub cwd: PathBuf,
}

impl App {
    pub fn new(image_picker: ImagePicker, theme_choice: ThemeChoice, theme: Theme) -> Self {
        Self {
            should_quit: false,
            mode: Mode::Reading,
            reader: Reader::empty(),
            wpm: 300,
            file_path: None,
            picker: FilePicker::new(),
            last_tick: Instant::now(),
            status_msg: None,
            image_picker,
            image_cache: HashMap::new(),
            theme_choice,
            theme,
        }
    }

    pub fn open_path(&mut self, path: &Path) -> Result<()> {
        let blocks = crate::doc::load(path)?;
        let base_dir = path.parent().map(|p| p.to_path_buf());

        self.image_cache.clear();
        let mut failed = 0usize;
        for block in &blocks {
            if let Block::Image(url) = block {
                if self.image_cache.contains_key(url) {
                    continue;
                }
                match load_image(&mut self.image_picker, url, base_dir.as_deref()) {
                    Ok(proto) => {
                        self.image_cache.insert(url.clone(), proto);
                    }
                    Err(_) => failed += 1,
                }
            }
        }

        self.reader = Reader::from_blocks(blocks);
        self.file_path = Some(path.to_path_buf());
        self.mode = Mode::Reading;
        self.last_tick = Instant::now();
        self.status_msg = if failed > 0 {
            Some(format!("{} image(s) failed to load", failed))
        } else {
            None
        };
        Ok(())
    }

    pub fn tick_timeout(&self) -> Duration {
        if self.mode != Mode::Reading || !self.reader.playing {
            return Duration::from_millis(250);
        }
        let per_chunk = self.reader.chunk_duration(self.wpm);
        let elapsed = self.last_tick.elapsed();
        per_chunk
            .saturating_sub(elapsed)
            .max(Duration::from_millis(5))
    }

    pub fn tick(&mut self) {
        if self.mode != Mode::Reading || !self.reader.playing {
            return;
        }
        let per_chunk = self.reader.chunk_duration(self.wpm);
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
            Action::Quit => self.should_quit = true,
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
            Action::ToggleHelp => {
                self.mode = match self.mode {
                    Mode::Help => Mode::Reading,
                    _ => Mode::Help,
                };
            }
            Action::CycleTheme => {
                self.theme_choice = self.theme_choice.cycle();
                self.theme = crate::theme::resolve(self.theme_choice);
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
        }
        Ok(())
    }
}

fn load_image(
    picker: &mut ImagePicker,
    src: &str,
    base_dir: Option<&Path>,
) -> Result<StatefulProtocol> {
    let bytes = fetch_bytes(src, base_dir)?;
    let img = image::load_from_memory(&bytes).context("failed to decode image")?;
    Ok(picker.new_resize_protocol(img))
}

fn fetch_bytes(src: &str, base_dir: Option<&Path>) -> Result<Vec<u8>> {
    if src.starts_with("http://") || src.starts_with("https://") {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(15))
            .build();
        let resp = agent.get(src).call().with_context(|| format!("GET {}", src))?;
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
