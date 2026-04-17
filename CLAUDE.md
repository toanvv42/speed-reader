# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
cargo build --release        # release build
cargo build                  # debug build
cargo run -- sample.md       # run against sample file
cargo test                   # run tests
cargo clippy                 # lint
cargo fmt                    # format

make run                     # build release + open sample.md
make install                 # install to ~/.local/bin
make clean                   # cargo clean + remove dist/

# Cross-compile (requires `make setup-cross` once)
make mac                     # aarch64 + x86_64 macOS
make linux                   # aarch64 + x86_64 Linux
make dist                    # all four targets → dist/*.tar.gz
```

## Architecture

The app is a single-binary Rust TUI with a straightforward pipeline:

**`doc.rs`** — Parses a markdown (or plain text) file into `Block` variants: `Text`, `Heading`, `Code`, `Image`. Uses `pulldown-cmark`.

**`reader.rs`** — Converts `Block`s into `Chunk`s for RSVP display. Each chunk has a `multiplier` that slows pacing at punctuation or block boundaries. The `orp` field marks the Optimal Recognition Point index (the letter to highlight). Playback state (index, playing flag) lives here.

**`app.rs`** — Top-level state: holds `Reader`, `FilePicker`, WPM, theme, image cache, and current `Mode` (`Reading | Picker | Help`). `tick()` advances playback each frame. `handle(Action)` is the single dispatch point for all user actions. Images are loaded eagerly on `open_path()` and cached in a `HashMap<url, StatefulProtocol>`.

**`input.rs`** — Maps crossterm key events → `Action` enum variants.

**`ui.rs`** — ratatui rendering; called every frame via `terminal.draw()`.

**`theme.rs`** — `ThemeChoice` (Dark/Light/System) and palette structs. "System" queries the terminal background luminance via OSC 11 (`terminal-light` crate) and chooses dark or light accordingly.

**`main.rs`** — CLI parsing (clap), terminal lifecycle (raw mode, alternate screen), image picker initialization (must happen before raw mode to query terminal capabilities), and the main event loop.

## Key design points

- Terminal graphics are detected via `ImagePicker::from_query_stdio()` **before** entering raw mode; afterwards the query would be suppressed by the alternate screen.
- Image pauses playback automatically when the reader advances to an `Image` chunk (`ChunkKind::Image` sets `playing = false` in `advance()`).
- `chunk_duration()` multiplies the base ms-per-word by the chunk's `multiplier`, so sentence ends and headings pause longer.
- The file picker is a simple substring filter (`refilter()`), not a fuzzy matcher despite the README's "fuzzy-ish" label.
