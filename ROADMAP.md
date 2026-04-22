# Speed Reader Roadmap

This file is the working plan for product, engineering, and marketing. It is meant to be updated as work ships.

## Current Focus

As of `2026-04-22`, active execution is focused on the web app only.

That means:

- web UX and onboarding take priority over terminal UX
- browser MVP and web parity work move to the top of the queue
- terminal-only feature work is paused unless it directly supports the shared Rust engine

## Product Positioning

**Core idea:** a calm, private reading instrument for focused reading.

**Not the promise:** "read 10x faster."

**Primary value props**

- Private by default
- Local-first document reading
- Focused RSVP reading for long-form text
- Works with real files: `md`, `txt`, `docx`, `pdf`
- Minimal UI for keyboard-first readers

**Primary audiences**

- Knowledge workers reading long docs
- Students reading dense material
- Programmers and writers reading markdown and technical text
- Productivity users who prefer calm tools
- Multilingual readers, including Vietnamese users

## Success Criteria

### Product

- [ ] New users understand what the app does within 10 seconds
- [ ] README and landing page match actual capabilities
- [ ] Opening a file and starting a reading session feels obvious
- [x] Resume flow is visible and useful, not hidden state
- [ ] Core reading experience feels better on real documents, not just demos

### Distribution

- [ ] Public landing page clearly communicates the product
- [ ] Demo assets exist for terminal and web
- [ ] First launch posts are prepared before release
- [ ] Feedback channel is easy to find

## Phase 1: Tighten The Core

Goal: make the current product feel complete and trustworthy.

Note: terminal items below may remain complete, but new execution should favor equivalent web work first.

### Positioning and Messaging

- [x] Rewrite README intro around focused reading, privacy, and calm design
- [x] Update README feature list to reflect actual support for `txt`, `docx`, `pdf`, resume state, image pause, and themes
- [x] Align CLI description and landing page copy with the same product language
- [x] Replace vague "speed reading" language with more credible "focus reading" language where appropriate

### Onboarding

- [x] Ensure first-run experience is clear even with no file provided
- [x] Add a short in-app explanation of RSVP and recommended starting WPM
- [x] Ship a polished sample document for first-time use
- [x] Make help overlay more instructional for new users

### Core UX

- [x] Expose recent files and resume progress in the UI
- [x] Add reading presets such as `gentle`, `standard`, `technical`, and `study`
- [x] Improve progress visibility with section-aware cues where possible
- [x] Add optional small context preview around current word
- [x] Improve status bar wording and consistency

### Robustness

- [ ] Test parsing quality on real `pdf`, `docx`, and markdown files
- [ ] Improve handling of awkward punctuation, quotes, and technical prose
- [ ] Review behavior on image-heavy markdown documents
- [ ] Verify persistence behavior for moved or renamed files

## Phase 2: Make It Distinctly Useful

Goal: move from "nice demo" to "tool people keep."

### Reading Intelligence

- [ ] Implement adaptive pacing for long words, headings, code, and sentence boundaries
- [ ] Tune pause logic for comprehension, not just raw speed
- [ ] Add better pacing around paragraph boundaries and dense sections

### Session Design

- [ ] Add reading goals: time-based, word-based, or finish-this-file
- [ ] Show progress toward the active goal
- [ ] Add lightweight session completion feedback

### Reader Utilities

- [ ] Add bookmarks or "mark this spot" support
- [ ] Add simple note markers or "confusing section" flags
- [ ] Improve section navigation if heading structure is available

### Document Quality

- [ ] Improve extraction quality for messy PDFs
- [ ] Handle code blocks in a way that remains readable in RSVP
- [ ] Review docx heading detection against real-world files

## Phase 3: Differentiate

Goal: create features that are memorable and worth sharing.

### Reading Modes

- [ ] Add mode presets like `deep work`, `study`, `skim`, and `code review`
- [ ] Tune UI chrome and pacing behavior per mode

### Language Support

- [ ] Improve product messaging around multilingual support
- [ ] Test segmentation and pacing with Vietnamese and mixed-language documents
- [ ] Consider language-aware defaults later if needed

### Comprehension Features

- [ ] Add optional recap pauses at useful boundaries
- [ ] Add lightweight prompts for reflection or marking key sections
- [ ] Avoid quiz-heavy or gamified UX

### Analytics

- [ ] Track useful local-only stats such as time spent, files completed, and average pace
- [ ] Surface stats only if they help the reading habit

## Web Strategy

Goal: use the existing WASM direction to widen distribution.

- [ ] Ship a browser MVP for local-only reading
- [ ] Support drag-and-drop for `md`, `txt`, and `docx`
- [ ] Make mobile controls acceptable for simple reading sessions
- [ ] Preserve the privacy-first message: no account required
- [ ] Keep parity with core pacing behavior from the Rust engine

## Web-Only Execution Order

This is the active queue now.

### Now

- [x] Audit the current web onboarding flow in `web/index.html`
- [x] Make the landing state explain the product in one screen
- [x] Ensure the web app can open a sample document quickly
- [x] Verify drag-and-drop and local file opening on the browser path

### Next

- [x] Improve mobile controls and touch guidance
- [x] Expose presets clearly in the web UI
- [x] Show chapter navigation and progress more clearly in the web UI
- [x] Confirm parity between terminal and web pacing behavior where shared logic exists

### After That

- [ ] Prepare web screenshots and demo assets
- [ ] Draft a web-first launch flow and demo script
- [ ] Polish the public browser MVP for sharing

## Preset Spec

These are the first reading presets to implement in the app. They should start simple and tune `wpm` plus chunk pacing behavior before adding more advanced logic.

### `gentle`

- Default WPM: `250`
- Use when: first-time users, dense prose, fatigue
- Behavior intent: slower baseline, stronger punctuation and paragraph pauses

### `standard`

- Default WPM: `300`
- Use when: general reading
- Behavior intent: current default experience, balanced pace

### `technical`

- Default WPM: `240`
- Use when: docs, specs, code-heavy material
- Behavior intent: slower code and heading pacing, more caution around symbols and long tokens

### `study`

- Default WPM: `220`
- Use when: memorization, exam prep, deliberate reading
- Behavior intent: strongest sentence and paragraph pauses, optimized for comprehension over throughput

### First implementation rule

- Presets may start by setting baseline `wpm` and a label in the UI
- A later pass should let presets influence chunk multipliers for code, headings, punctuation, and paragraph boundaries
- Preset switching should not remove manual WPM adjustment; users can start from a preset and fine-tune

## Marketing Strategy

## Positioning

**Best positioning line:** a quiet reading instrument for long-form text.

**Messaging pillars**

- Focus: one word at a time reduces scanning noise
- Privacy: documents stay local
- Calmness: minimal, non-addictive, no feed-like interface
- Practicality: supports real files and real reading workflows
- Taste: keyboard-friendly, minimal, intentional design

## Go-To-Market

### Launch Assets

- [ ] Homepage with crisp product copy
- [ ] Terminal screenshots
- [ ] Web screenshots
- [ ] One short demo GIF
- [ ] One short demo video
- [x] "Why this exists" launch post

### Launch Channels

- [x] GitHub README and release notes
- [ ] Hacker News launch post
- [ ] Reddit posts for relevant communities
- [ ] Personal site or blog post
- [ ] X/Twitter thread if useful

### Target Communities

- [ ] `r/rust`
- [ ] `r/commandline`
- [ ] `r/productivity`
- [ ] `r/plaintext`
- [ ] Obsidian or markdown-focused communities
- [ ] Indie hacker / builder communities

### Content Angles

- [x] "Why I built a calm reading tool instead of another productivity app"
- [x] "Reading markdown, PDFs, and docs with one-word focus"
- [x] "A local-first reading tool for technical readers"
- [ ] "What RSVP gets right and wrong"

## 90-Day Execution Plan

## Month 1

- [x] Rewrite README and landing page copy
- [x] Add recent files / resume UI
- [x] Add onboarding improvements
- [x] Add reading presets
- [ ] Prepare screenshots and demo assets

## Web Sprint

Use this instead of the broader month buckets while the web app is the only active focus.

- [x] Web onboarding audit
- [x] Web landing-state cleanup
- [x] Web sample/open flow polish
- [x] Web mobile-control pass
- [ ] Web screenshots/demo prep

## Month 2

- [ ] Ship adaptive pacing
- [ ] Add reading goals
- [ ] Improve progress and context visibility
- [ ] Test on real-world PDFs and docx files
- [x] Draft launch posts

## Month 3

- [ ] Ship browser MVP
- [ ] Add bookmarks or note markers
- [ ] Launch publicly
- [ ] Collect and triage feedback
- [ ] Reprioritize based on usage and friction

## Immediate Next Actions

- [x] Rewrite `README.md` to match the current app and new positioning
- [x] Audit landing page copy in `web/index.html`
- [x] Define the first set of reading presets
- [x] Design recent-files / resume UI in the terminal app
- [x] Choose which Month 1 item ships first

## Progress Log

Use this section as an execution journal. Keep entries short and dated.

- 2026-04-22: Initial roadmap and marketing strategy written into `ROADMAP.md`.
- 2026-04-22: Rewrote `README.md` around focused reading, updated feature coverage, and aligned CLI and web metadata copy with the new positioning.
- 2026-04-22: Added first-run onboarding copy in the empty state and help overlay, including a short RSVP explanation and recommended starting WPM.
- 2026-04-22: Defined the first preset set in `ROADMAP.md`: `gentle`, `standard`, `technical`, and `study`, with concrete default WPM targets and behavior intent.
- 2026-04-22: Added a visible recent-files shelf to the empty state, persisted true recent-file ordering, and made the top five entries directly reopenable with `1`–`5`.
- 2026-04-22: Implemented terminal reading presets with a `p` shortcut, added section-aware status cues, refreshed status wording, and rewrote `sample.md` into a stronger first-run document.
- 2026-04-22: Added a lightweight context strip around the current word so the RSVP view keeps a bit more semantic orientation.
- 2026-04-22: Added `MARKETING.md` with launch positioning, asset checklist, release-notes copy, and draft posts for GitHub, Hacker News, Reddit, and X.
- 2026-04-22: Updated the roadmap to make the web app the only active execution focus until further notice.
- 2026-04-22: Shipped a web-first onboarding pass in `web/index.html`: clearer landing copy, built-in sample loading, visible preset controls, improved in-reader chapter/progress context, and a mobile status-bar layout that accommodates the preset flow.
- 2026-04-22: Moved preset definitions into shared Rust code and switched the web app to wasm-backed preset timing so browser pacing stays aligned with the shared reader engine.
