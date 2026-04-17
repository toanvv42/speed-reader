use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, prelude::Backend};
use ratatui_image::picker::Picker as ImagePicker;

mod app;
mod input;
mod ui;

use app::App;
use speed_reader::theme::{self, ThemeChoice};

#[derive(Parser)]
#[command(
    name = "speed-reader",
    about = "RSVP speed reader for markdown, text, docx, and pdf files"
)]
struct Cli {
    /// Path to the file to open (.md/.markdown/.mdx/.txt/.docx/.pdf)
    path: Option<PathBuf>,

    /// Color theme. `system` queries the terminal background (OSC 11).
    #[arg(long, value_enum, default_value_t = ThemeChoice::System)]
    theme: ThemeChoice,

    /// Seconds to dwell on each image before auto-advancing.
    #[arg(long, default_value_t = 3.0)]
    image_pause: f32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Query terminal capabilities BEFORE entering raw mode / alt screen:
    //   - graphics protocol + font cell size (ratatui-image)
    //   - background luma for the "system" theme (terminal-light via OSC 11)
    let image_picker =
        ImagePicker::from_query_stdio().unwrap_or_else(|_| ImagePicker::halfblocks());
    let theme_choice = cli.theme;
    let theme = theme::resolve(theme_choice);

    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let image_pause = Duration::from_millis((cli.image_pause.max(0.0) * 1000.0) as u64);
    let mut app = App::new(image_picker, theme_choice, theme, image_pause);
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
        app.pump_images();
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = app.tick_timeout();
        if let Some(action) = input::poll(timeout, app)? {
            app.handle(action)?;
        }
        app.tick();
    }
    Ok(())
}
