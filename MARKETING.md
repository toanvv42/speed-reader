# Speed Reader Marketing

This file turns the roadmap's marketing section into concrete launch assets and draft copy.

## Positioning

### Primary line

`speed-reader` is a quiet reading instrument for long-form text.

### Supporting lines

- Read with focus, one word at a time.
- Private by design. Local files stay local.
- Built for markdown, text, docx, and pdf.
- Calm enough for real reading, not productivity theater.

## Short Description

`speed-reader` is a local-first reading tool that presents text one word at a time so you can stay with the sentence instead of scanning the page. It supports real documents, keyboard-first workflows, chapter navigation, and focused reading presets.

## Feature Highlights

- Terminal-first reading workflow
- Resume where you left off
- Chapter and section jumps
- Gentle, standard, technical, and study presets
- Unicode-friendly reading, including Vietnamese
- Inline image pauses and code handling
- Local-first document support for `md`, `txt`, `docx`, and `pdf`

## Asset Checklist

- [ ] Terminal screenshot: empty state with recent files
- [ ] Terminal screenshot: active reading session with chapter info
- [ ] Terminal screenshot: chapter picker
- [ ] Terminal screenshot: code block mode
- [ ] Web screenshot: landing state
- [ ] Web screenshot: active RSVP state
- [ ] 20-30 second demo GIF
- [ ] 30-60 second demo video with captioning

## Feedback Channel

Recommended path:

- GitHub Issues for bugs and parsing failures
- GitHub Discussions for product feedback and reading workflow requests

## Launch Post: Long Form

### Title

I built a calm reading instrument for markdown, docx, and pdf

### Draft

Most reading software is built around scrolling, tabs, feeds, and interruption.

I wanted something quieter.

So I built `speed-reader`: a local-first reading tool that presents one word at a time and keeps the interface almost completely out of the way. It is not designed around fake "10x speed" claims. It is designed around focus.

The app already supports:

- markdown, text, docx, and pdf
- resume state per file
- chapter and section jumps
- code blocks and image pauses
- reading presets for gentle, standard, technical, and study modes
- Unicode-friendly text, including Vietnamese

It runs in the terminal today, and the web version is built from the same Rust reading engine.

If you try it, I especially want feedback on:

- dense technical documents
- messy PDFs
- multilingual text
- where pacing feels wrong
- where chapter detection breaks

Repo:

`https://github.com/toanvv42/speed-reader`

## Hacker News Draft

### Title options

- Show HN: speed-reader, a calm local-first reading instrument
- Show HN: speed-reader, one-word reading for markdown, docx, and pdf

### Body

Built this because most reading tools feel noisy.

`speed-reader` is a local-first reading tool that shows one word at a time for focused reading. It currently supports markdown, text, docx, and pdf, plus resume state, chapter jumps, reading presets, code blocks, and inline images.

Interested in feedback on:

- technical docs
- PDF parsing quality
- pacing presets
- multilingual text

## Reddit Draft

### r/rust

I built a Rust-based reading tool for focused reading of markdown, docx, and pdf.

It uses a shared Rust engine for both terminal and web reading, supports chapter jumps, resume state, presets, and Unicode-safe tokenization.

I’d love feedback on parser edge cases and pacing behavior.

### r/commandline

I wanted a reading tool that felt more like an instrument than an app.

This one runs in the terminal, opens local documents, remembers where you stopped, supports chapter jumps, and stays keyboard-first.

### r/productivity

This is not a "read 10x faster" tool.

It is a calmer way to read long-form documents with fewer visual distractions. If you read specs, essays, study material, or markdown notes, it may be useful.

## X / Twitter Thread Draft

1. I built `speed-reader` because most reading software is optimized for scrolling, not concentration.
2. It’s a local-first reading tool for markdown, text, docx, and pdf.
3. It remembers where you left off, supports chapter jumps, and now has reading presets for gentle, standard, technical, and study modes.
4. The terminal app is real and usable today.
5. The web version uses the same Rust reading engine.
6. Looking for feedback on messy PDFs, technical docs, and multilingual text.

## Demo Script

1. Start on the empty state and show recent files.
2. Open `sample.md`.
3. Start playback at standard preset.
4. Cycle to technical preset on the code section.
5. Open chapter picker and jump forward.
6. Pause on the image section.
7. Show resume by quitting and reopening.

## Release Notes Draft

### Version summary

This update reframes `speed-reader` as a calm, local-first reading instrument and improves the first-run experience.

### Highlights

- new product messaging in README and web metadata
- recent-files shelf with direct reopen shortcuts
- reading presets with `p` to cycle
- section-aware status line
- lightweight context preview around the current word
- improved sample document for first-run testing
