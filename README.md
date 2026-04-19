# speed-reader

A tiny RSVP (Rapid Serial Visual Presentation) speed reader for markdown,
running in the terminal. One word flashes at a time at a fixed cadence —
your eyes stop saccading and reading speed becomes a function of
recognition rather than scanning.

Built with [ratatui](https://ratatui.rs) + [crossterm](https://github.com/crossterm-rs/crossterm),
with inline images via [ratatui-image](https://github.com/benjajaja/ratatui-image).
Targeted at [Ghostty](https://ghostty.org/) (or any terminal that speaks
Kitty / Sixel graphics), but falls back to half-blocks on plainer terms.

## Features

- RSVP playback of markdown files (headings, paragraphs, code blocks)
- Inline images — local paths and `http(s)://` URLs, rendered via the
  terminal's native graphics protocol when available
- Chapter / section picker (`c`) to jump directly into a document
- Dark / light / **system** themes (auto-detects terminal background via OSC 11)
- Fuzzy-ish file picker (`o`) to open another file without leaving the TUI
- Adjustable WPM from 50 to 1500 (steps of 25, default 300)
- Unicode-safe word segmentation — works for English, Vietnamese, etc.

## Install

### macOS — one-liner (prebuilt binary)

```sh
curl -fsSL https://raw.githubusercontent.com/toanvv42/speed-reader/master/install.sh | sh
```

Installs the latest release to `~/.local/bin`. Options:

```sh
# pin a version
curl -fsSL .../install.sh | sh -s -- --version v0.1.0

# custom prefix (binary → $PREFIX/bin)
curl -fsSL .../install.sh | PREFIX=/usr/local sh
```

### From source (any platform)

Requires a recent Rust toolchain.

```sh
cargo install --git https://github.com/toanvv42/speed-reader
```

Or clone and build:

```sh
git clone https://github.com/toanvv42/speed-reader
cd speed-reader
make install         # builds release, installs to ~/.local/bin
make run             # builds and opens sample.md
```

Pre-built macOS binaries (x86_64 + aarch64) are attached to
[GitHub Releases](../../releases) for tagged versions.

## Usage

```sh
speed-reader [PATH] [--theme dark|light|system]
```

If no path is given, press `o` to open the file picker.

### Keys

| Key              | Action                             |
| ---------------- | ---------------------------------- |
| `Space`          | play / pause                       |
| `← →` / `h l`    | step one word                      |
| `↑ ↓` / `+ -`    | WPM ± 25                           |
| `o`              | open file picker                   |
| `c`              | open chapter picker                |
| `t`              | cycle theme (dark · light · system) |
| `?`              | toggle help                        |
| `q` / `Esc`      | quit                               |

In the pickers: `↑ ↓` move, `Enter` confirm, `Backspace` edit query,
`Esc` cancel. The file picker uses `Enter` to open/descend. When an image block
is reached, `Space` or `→` advances past it.

## Building

```sh
make                 # release build for host
make install         # install to $PREFIX/bin (default ~/.local)
make clean           # cargo clean + remove dist/
```

Cross-compiling (uses [`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild)):

```sh
make setup-cross     # one-time: installs zig + cargo-zigbuild
make mac             # aarch64 + x86_64 macOS
make linux           # aarch64 + x86_64 Linux (gnu)
make dist            # builds all four targets and tarballs into dist/
```

`make help` prints the full list.

## Web app configuration (Google Analytics + API keys)

The browser build lives in `web/index.html`.

### Enable Google Analytics (GA4)

1. Create a GA4 web data stream in Google Analytics and copy its **Measurement ID** (looks like `G-XXXXXXXXXX`).
2. Open `web/index.html`.
3. Set the `google-analytics-measurement-id` meta tag:

   ```html
   <meta name="google-analytics-measurement-id" content="G-XXXXXXXXXX">
   ```

4. Deploy as usual. If the tag is empty, analytics stays disabled.

The app records lightweight events such as:
- `reader_loaded`
- `document_loaded` (`markdown`, `plain_text`, `docx`, `pdf`)
- `reader_play` / `reader_pause`
- `wpm_changed`
- `theme_changed`

### Adding Google API keys safely

If you add features that call Google APIs (Drive, Gemini, etc.), use this rule:

- **Never put secret server keys in browser JavaScript.** Anything in `web/index.html` is public.
- For browser-only APIs, use a restricted browser key (HTTP referrer restrictions in Google Cloud Console).
- For sensitive APIs, route requests through your backend and keep secrets in server environment variables.

Recommended setup:

1. Store secrets on the server (`GOOGLE_API_KEY`, service account credentials, etc.).
2. Expose only a minimal endpoint from your backend to the web app.
3. In the frontend, call your own endpoint (not Google directly) for privileged operations.
4. Add rate limits and auth checks on that backend endpoint.

## Project layout

```
src/
  main.rs     entry point, CLI, terminal lifecycle
  app.rs      app state, actions, file picker, image loading
  doc.rs      markdown → blocks
  reader.rs   blocks → RSVP chunks, pacing
  input.rs    key → action mapping
  ui.rs       ratatui rendering
  theme.rs    dark / light / system palettes
```

## License

MIT. See `Cargo.toml` for crate metadata.
