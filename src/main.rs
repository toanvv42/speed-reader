use std::io::stdout;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, prelude::Backend};
use ratatui_image::picker::Picker as ImagePicker;

mod app;
mod doc;
mod input;
mod reader;
mod theme;
mod ui;

use app::App;
use theme::ThemeChoice;

#[derive(Parser)]
#[command(name = "speed-reader", about = "RSVP speed reader for markdown files")]
struct Cli {
    /// Path to a markdown or text file to open
    path: Option<PathBuf>,

    /// Color theme. `system` queries the terminal background (OSC 11).
    #[arg(long, value_enum, default_value_t = CliTheme::System)]
    theme: CliTheme,
}

#[derive(Copy, Clone, ValueEnum)]
enum CliTheme {
    Dark,
    Light,
    System,
}

impl From<CliTheme> for ThemeChoice {
    fn from(t: CliTheme) -> Self {
        match t {
            CliTheme::Dark => ThemeChoice::Dark,
            CliTheme::Light => ThemeChoice::Light,
            CliTheme::System => ThemeChoice::System,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Query terminal capabilities BEFORE entering raw mode / alt screen:
    //   - graphics protocol + font cell size (ratatui-image)
    //   - background luma for the "system" theme (terminal-light via OSC 11)
    let image_picker =
        ImagePicker::from_query_stdio().unwrap_or_else(|_| ImagePicker::halfblocks());
    let theme_choice: ThemeChoice = cli.theme.into();
    let theme = theme::resolve(theme_choice);

    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(image_picker, theme_choice, theme);
    let run_res = (|| -> Result<()> {
        if let Some(path) = cli.path.as_ref() {
            app.open_path(path)?;
        }
        run(&mut terminal, &mut app)
    })();

    disable_raw_mode().ok();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .ok();
    terminal.show_cursor().ok();

    run_res
}

fn run<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    <B as Backend>::Error: Send + Sync + 'static,
{
    while !app.should_quit {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = app.tick_timeout();
        if let Some(action) = input::poll(timeout, app)? {
            app.handle(action)?;
        }
        app.tick();
    }
    Ok(())
}
