use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use ratatui_image::{Resize, StatefulImage};
use unicode_segmentation::UnicodeSegmentation;

use crate::app::{App, Mode};
use speed_reader::reader::{Chunk, ChunkKind};
use speed_reader::text::truncate_start;
use speed_reader::theme::Palette;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let palette = Palette::for_theme(app.theme);

    // paint full background so light theme looks light even on a dark terminal
    let bg = Block::default().style(Style::default().bg(palette.bg).fg(palette.fg));
    f.render_widget(bg, size);

    draw_reader(f, app, size, &palette);

    match app.mode {
        Mode::Picker => draw_picker(f, app, size, &palette),
        Mode::Help => draw_help(f, size, &palette),
        Mode::Reading => {}
    }
}

fn draw_reader(f: &mut Frame, app: &mut App, area: Rect, p: &Palette) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let body = rows[0];
    let status = rows[1];

    let current_kind = app.reader.current().map(|c| c.kind);
    let current_url = app.reader.current().and_then(|c| c.image_url.clone());

    match current_kind {
        Some(ChunkKind::Image) => draw_image_body(f, app, body, current_url, p),
        Some(_) => draw_text_body(f, app, body, p),
        None => draw_empty_body(f, body, p),
    }

    f.render_widget(render_status(app, p), status);
}

fn draw_text_body(f: &mut Frame, app: &App, body: Rect, p: &Palette) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(body);

    if let Some(c) = app.reader.current() {
        let line = render_chunk(c, p);
        let word = Paragraph::new(line).alignment(Alignment::Center);
        f.render_widget(word, inner[1]);

        let hint = kind_hint(c.kind);
        if !hint.is_empty() {
            let hint_para =
                Paragraph::new(Line::from(Span::styled(hint, Style::default().fg(p.dim))))
                    .alignment(Alignment::Center);
            f.render_widget(hint_para, inner[2]);
        }
    }
}

fn draw_empty_body(f: &mut Frame, body: Rect, p: &Palette) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(body);

    let empty = Paragraph::new(Line::from(vec![
        Span::styled("press ", Style::default().fg(p.dim)),
        Span::styled(
            "Ctrl+O",
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" or ", Style::default().fg(p.dim)),
        Span::styled(
            "o",
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" to open a file", Style::default().fg(p.dim)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(empty, inner[1]);
}

fn draw_image_body(f: &mut Frame, app: &mut App, body: Rect, url: Option<String>, p: &Palette) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(body);

    let image_area = rows[0];
    let caption_area = rows[1];

    let url = match url {
        Some(u) => u,
        None => return,
    };

    match app.image_cache.get_mut(&url) {
        Some(proto) => {
            let resize = Resize::Fit(None);
            let rendered = proto.size_for(resize.clone(), image_area);
            let w = rendered.width.min(image_area.width);
            let h = rendered.height.min(image_area.height);
            let x = image_area.x + image_area.width.saturating_sub(w) / 2;
            let y = image_area.y + image_area.height.saturating_sub(h) / 2;
            let centered = Rect::new(x, y, w, h);
            let widget = StatefulImage::default().resize(resize);
            f.render_stateful_widget(widget, centered, proto);

            let caption = Line::from(vec![
                Span::styled("image: ", Style::default().fg(p.dim)),
                Span::styled(
                    truncate_start(&url, caption_area.width as usize).to_string(),
                    Style::default().fg(p.fg),
                ),
            ]);
            let hint = Line::from(Span::styled(
                "auto-advances · Space pause · ← → to step",
                Style::default().fg(p.dim),
            ));
            let para = Paragraph::new(vec![caption, hint]).alignment(Alignment::Center);
            f.render_widget(para, caption_area);
        }
        None => {
            let (label, label_style) = if app.pending_image_urls.contains(&url) {
                ("[loading image…] ", Style::default().fg(p.dim))
            } else {
                ("[image failed to load] ", Style::default().fg(p.accent))
            };
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(label, label_style),
                Span::styled(url.clone(), Style::default().fg(p.dim)),
            ]))
            .alignment(Alignment::Center);
            f.render_widget(msg, image_area);
        }
    }
}

fn render_chunk(c: &Chunk, p: &Palette) -> Line<'static> {
    if c.kind == ChunkKind::Paragraph {
        return Line::from(Span::styled("¶", Style::default().fg(p.dim)));
    }
    let graphemes: Vec<&str> = c.text.graphemes(true).collect();
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(graphemes.len());
    for (i, g) in graphemes.iter().enumerate() {
        let style = if i == c.orp {
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.fg)
        };
        spans.push(Span::styled((*g).to_string(), style));
    }
    Line::from(spans)
}

fn kind_hint(k: ChunkKind) -> &'static str {
    match k {
        ChunkKind::Heading => "— heading —",
        ChunkKind::Code => "— code —",
        _ => "",
    }
}

fn render_status(app: &App, p: &Palette) -> Paragraph<'static> {
    let total = app.reader.chunks.len();
    let idx = if total == 0 { 0 } else { app.reader.index + 1 };
    let play = if app.reader.playing { "▶" } else { "⏸" };
    let file = app
        .file_path
        .as_ref()
        .and_then(|pp| pp.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("—")
        .to_string();
    let msg = app.status_msg.clone().unwrap_or_default();
    let text = if msg.is_empty() {
        format!(
            " {}  {}  {} wpm  {}/{}   ?help  Ctrl+O open  t theme  q quit ",
            play, file, app.wpm, idx, total
        )
    } else {
        format!(" {}  {}   [{}] ", play, file, msg)
    };
    Paragraph::new(text).style(Style::default().fg(p.status_fg).bg(p.status_bg))
}

fn draw_picker(f: &mut Frame, app: &App, area: Rect, p: &Palette) {
    let w = (area.width.saturating_sub(6)).clamp(30, 72);
    let h = (area.height.saturating_sub(4)).clamp(10, 22);
    let x = area.width.saturating_sub(w) / 2;
    let y = area.height.saturating_sub(h) / 2;
    let rect = Rect::new(x, y, w, h);

    f.render_widget(Clear, rect);

    let title = format!(
        " Open file — {} ",
        truncate_start(
            &app.picker.cwd.display().to_string(),
            (w as usize).saturating_sub(12),
        )
    );
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent))
        .style(Style::default().bg(p.modal_bg).fg(p.fg));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    let query = Paragraph::new(format!("› {}_", app.picker.query)).style(Style::default().fg(p.fg));
    f.render_widget(query, rows[0]);

    let divider =
        Paragraph::new("─".repeat(inner.width as usize)).style(Style::default().fg(p.divider));
    f.render_widget(divider, rows[1]);

    let items: Vec<ListItem> = app
        .picker
        .filtered
        .iter()
        .map(|&i| {
            let path = &app.picker.entries[i];
            let name = if app.picker.is_parent(path) {
                "..".to_string()
            } else {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string()
            };
            let display = if path.is_dir() {
                format!("{}/", name)
            } else {
                name
            };
            ListItem::new(display)
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(p.list_hl_bg)
                .fg(p.accent)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");

    let mut state = ListState::default();
    if !app.picker.filtered.is_empty() {
        state.select(Some(app.picker.selected));
    }
    f.render_stateful_widget(list, rows[2], &mut state);
}

fn draw_help(f: &mut Frame, area: Rect, p: &Palette) {
    let w = 54u16.min(area.width.saturating_sub(4)).max(30);
    let h = 15u16.min(area.height.saturating_sub(2)).max(10);
    let x = area.width.saturating_sub(w) / 2;
    let y = area.height.saturating_sub(h) / 2;
    let rect = Rect::new(x, y, w, h);

    f.render_widget(Clear, rect);

    let lines = vec![
        Line::from("  Space      play / pause"),
        Line::from("  ← → h l    step word"),
        Line::from("  ↑ ↓ + -    WPM ±25"),
        Line::from("  Ctrl+O o   open file"),
        Line::from("  t          cycle theme (dark · light · system)"),
        Line::from("  ?          toggle this help"),
        Line::from("  q / Esc    quit"),
        Line::from(""),
        Line::from("  on image   auto-advance (--image-pause N sec)"),
        Line::from("  (picker)   ↑↓ move · Enter open · Esc cancel"),
    ];

    let block = Block::default()
        .title(" speed-reader — help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent))
        .style(Style::default().bg(p.modal_bg).fg(p.fg));
    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(p.fg))
        .wrap(Wrap { trim: false });
    f.render_widget(para, rect);
}
