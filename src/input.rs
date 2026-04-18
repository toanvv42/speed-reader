use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{Action, App, Mode};

pub fn poll(timeout: Duration, app: &App) -> Result<Option<Action>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    match event::read()? {
        Event::Key(k) if k.kind == KeyEventKind::Press => Ok(map_key(k, app)),
        _ => Ok(None),
    }
}

fn map_key(k: KeyEvent, app: &App) -> Option<Action> {
    match app.mode {
        Mode::Reading => map_reading(k),
        Mode::Picker => map_picker(k),
        Mode::ChapterPicker => map_chapter_picker(k),
        Mode::Help => map_help(k),
    }
}

fn map_reading(k: KeyEvent) -> Option<Action> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match (k.code, ctrl) {
        (KeyCode::Char('c'), true) => Some(Action::Quit),
        (KeyCode::Char('q'), false) => Some(Action::Quit),
        (KeyCode::Esc, _) => Some(Action::Quit),
        (KeyCode::Char(' '), _) => Some(Action::TogglePlay),
        (KeyCode::Right, _) | (KeyCode::Char('l'), false) => Some(Action::NextWord),
        (KeyCode::Left, _) | (KeyCode::Char('h'), false) => Some(Action::PrevWord),
        (KeyCode::Up, _) | (KeyCode::Char('+'), _) | (KeyCode::Char('='), _) => Some(Action::WpmUp),
        (KeyCode::Down, _) | (KeyCode::Char('-'), _) => Some(Action::WpmDown),
        (KeyCode::Char('o'), _) => Some(Action::OpenPicker),
        (KeyCode::Char('c'), false) => Some(Action::OpenChapterPicker),
        (KeyCode::Char('t'), false) => Some(Action::CycleTheme),
        (KeyCode::Char('?'), _) => Some(Action::ToggleHelp),
        _ => None,
    }
}

fn map_picker(k: KeyEvent) -> Option<Action> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match k.code {
        KeyCode::Esc => Some(Action::ClosePicker),
        KeyCode::Up => Some(Action::PickerUp),
        KeyCode::Down => Some(Action::PickerDown),
        KeyCode::Enter => Some(Action::PickerConfirm),
        KeyCode::Backspace => Some(Action::PickerBackspace),
        KeyCode::Char('c') if ctrl => Some(Action::Quit),
        KeyCode::Char(c) => Some(Action::PickerInput(c)),
        _ => None,
    }
}

fn map_chapter_picker(k: KeyEvent) -> Option<Action> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let alt = k.modifiers.contains(KeyModifiers::ALT);
    match k.code {
        KeyCode::Esc => Some(Action::CloseChapterPicker),
        KeyCode::Up => Some(Action::ChapterUp),
        KeyCode::Down => Some(Action::ChapterDown),
        KeyCode::Enter => Some(Action::ChapterConfirm),
        KeyCode::Backspace => Some(Action::ChapterBackspace),
        KeyCode::Char('c') if ctrl => Some(Action::Quit),
        KeyCode::Char(c) if !ctrl && !alt => Some(Action::ChapterInput(c)),
        _ => None,
    }
}

fn map_help(k: KeyEvent) -> Option<Action> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match (k.code, ctrl) {
        (KeyCode::Char('c'), true) => Some(Action::Quit),
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) | (KeyCode::Char('?'), _) => {
            Some(Action::ToggleHelp)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn chapter_picker_ignores_alt_modified_chars() {
        let key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::ALT);
        assert!(map_chapter_picker(key).is_none());
    }
}
