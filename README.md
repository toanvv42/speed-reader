# speed-reader

A calm, private reading instrument for long-form text.

`speed-reader` presents one word at a time so you can stay with the sentence instead of scanning the page. It is built for focused reading of real documents, not hypey "10x faster" claims.

The app runs in the terminal and in the browser through the same Rust reading engine. The browser UI is also packaged into the Tinywins Speed Reader extension, where page content is extracted locally and opened in the same reader interface without sending article text to a server.

Built with [ratatui](https://ratatui.rs) + [crossterm](https://github.com/crossterm-rs/crossterm), with inline images via [ratatui-image](https://github.com/benjajaja/ratatui-image). It is targeted at [Ghostty](https://ghostty.org/) and other terminals with graphics support, but falls back gracefully on simpler terminals.

## What It Does

- Focused RSVP reading for markdown, plain text, `docx`, and `pdf`
- Resume your place in each file automatically
- Render inline images from local paths or `http(s)://` URLs
- Jump by chapter or section when headings are available
- Handle headings, paragraphs, and code blocks with different pacing
- Support dark, light, and system themes
- Work well with Unicode text, including Vietnamese and other multilingual content

## Why It Exists

Most reading software is built for scrolling, tab-switching, and distraction. `speed-reader` is built for concentration:

- one word at a time
- keyboard-first controls
- local files
- private by default
- no account, no sync dependency, no feed-shaped UI

## Install

### macOS — one-liner (prebuilt binary)

```sh
curl -fsSL https://raw.githubusercontent.com/toanvv42/speed-reader/master/install.sh | sh
```

Installs the latest release to `~/.local/bin`. Options:

```sh
# pin a version
curl -fsSL .../install.sh | sh -s -- --version v0.1.0

# custom prefix (binary -> $PREFIX/bin)
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

Pre-built macOS binaries (x86_64 + aarch64) are attached to [GitHub Releases](../../releases) for tagged versions.

## Usage

```sh
speed-reader [PATH] [--theme dark|light|system] [--image-pause SECONDS]
```

Supported inputs include `.md`, `.markdown`, `.mdx`, `.txt`, `.docx`, and `.pdf`.

If no path is given, press `o` to open the file picker.

### Keys

| Key | Action |
| --- | --- |
| `Space` | play / pause |
| `← →` / `h l` | step one chunk |
| `↑ ↓` / `+ -` | WPM ± 25 |
| `o` | open file picker |
| `1`..`5` | reopen recent file from home screen |
| `c` | open chapter picker |
| `p` | cycle reading preset |
| `t` | cycle theme (dark · light · system) |
| `?` | toggle help |
| `q` / `Esc` | quit |

In the pickers: `↑ ↓` move, `Enter` confirm, `Backspace` edit query, `Esc` cancel. The file picker uses `Enter` to open or descend.

When an image block is reached, the reader waits for the configured image pause and then auto-advances.

## Current Product Shape

The terminal app already includes:

- saved reading position per file
- file picker for opening another document without leaving the UI
- chapter and section picker for navigating longer documents
- reading presets for gentle, standard, technical, and study pacing
- time remaining estimates
- section-aware status information while reading
- per-block pacing differences for text, headings, code, paragraph breaks, and images
- inline image loading with terminal graphics support when available

The web reader includes:

- paste, sample, drag-and-drop, and local file loading for markdown, text, `docx`, and `pdf`
- the same WASM-backed reader engine used by extension builds
- extension session boot via `?sessionId=...`, reading `speedReaderSession:<id>` from browser extension storage
- `returnUrl` support so the extension reader can replace the current tab and then return to the original page
- the canonical Tinywins UI used by both `tinywins.us` and the Chrome extension package

## Building

```sh
make                 # release build for host
make install         # install to $PREFIX/bin (default ~/.local)
make clean           # cargo clean + remove dist/
```

Build the canonical web reader:

```sh
./scripts/build-web.sh
```

This writes `dist/index.html`, a single-file build used for the public web app. The extension build uses the same `web/index.html` UI but packages its script and WASM as external extension assets to comply with browser extension CSP.

Cross-compiling (uses [`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild)):

```sh
make setup-cross     # one-time: installs zig + cargo-zigbuild
make mac             # aarch64 + x86_64 macOS
make linux           # aarch64 + x86_64 Linux (gnu)
make dist            # builds all four targets and tarballs into dist/
```

`make help` prints the full list.

## Project Layout

```text
src/
  main.rs     entry point, CLI, terminal lifecycle
  app.rs      app state, persistence, file picker, image loading
  doc.rs      markdown / text / docx / pdf -> blocks
  reader.rs   blocks -> RSVP chunks, pacing, time estimates
  input.rs    key -> action mapping
  ui.rs       ratatui rendering
  theme.rs    dark / light / system palettes
  wasm.rs     shared web reader bindings
```

## License

MIT. See `Cargo.toml` for crate metadata.
