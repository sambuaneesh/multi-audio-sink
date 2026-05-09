use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui::widgets::{hint_line, panel_block, Palette};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(area);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[0]);

    render_stream_list(f, app, body[0]);
    render_stream_detail(f, app, body[1]);
    render_hints(f, chunks[1]);
}

fn render_stream_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let streams = &app.audio_state.streams;
    let block = panel_block("🎵 Active Audio Streams", true);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if streams.is_empty() {
        let p = Paragraph::new(
            "No audio streams playing.\n\nStart music, a video, or any app\nthat produces sound to see it here.",
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Palette::MUTED));
        f.render_widget(p, inner);
        return;
    }

    let items: Vec<ListItem> = streams
        .iter()
        .map(|s| {
            let mute_icon = if s.muted { "🔇 " } else { "🔊 " };
            let vol_str = s.volume.map(|v| format!(" {}%", v)).unwrap_or_default();

            // Resolve sink display name
            let sink_disp = app
                .audio_state
                .device_by_index(s.sink_index)
                .map(|d| d.display_name().to_string())
                .unwrap_or_else(|| s.sink_name.clone());

            let on_default = s.sink_name == app.audio_state.default_sink_name;
            let sink_color = if on_default { Palette::GREEN } else { Palette::DIM };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(mute_icon, Style::default()),
                    Span::styled(s.display_name(), Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
                    Span::styled(vol_str, Style::default().fg(Palette::DIM)),
                ]),
                Line::from(vec![
                    Span::raw("     ↳ "),
                    Span::styled(sink_disp, Style::default().fg(sink_color)),
                    if on_default {
                        Span::styled(" ★", Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD))
                    } else {
                        Span::raw("")
                    },
                ]),
            ])
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.stream_cursor));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Palette::BG_SELECTED)
                .fg(Palette::ACCENT_BRIGHT)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, inner, &mut state);
}

fn render_stream_detail(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = panel_block("Stream Details", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let stream = match app.audio_state.streams.get(app.stream_cursor) {
        Some(s) => s,
        None => {
            let p = Paragraph::new("No stream selected.")
                .style(Style::default().fg(Palette::MUTED))
                .alignment(Alignment::Center);
            f.render_widget(p, inner);
            return;
        }
    };

    let sink_name = app
        .audio_state
        .device_by_index(stream.sink_index)
        .map(|d| d.display_name().to_string())
        .unwrap_or_else(|| stream.sink_name.clone());



    let on_default = stream.sink_name == app.audio_state.default_sink_name;

    let mut lines = vec![
        Line::from(Span::styled(
            stream.display_name(),
            Style::default().fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Playing on:  ", Style::default().fg(Palette::MUTED)),
            Span::styled(&sink_name, Style::default().fg(if on_default { Palette::GREEN } else { Palette::YELLOW })),
            if on_default {
                Span::styled(" ★ (default)", Style::default().fg(Palette::ACCENT))
            } else {
                Span::raw("")
            },
        ]),
    ];

    if let Some(vol) = stream.volume {
        let bar_len = (vol as usize * 20) / 100;
        let bar = format!("[{}{}] {}%", "█".repeat(bar_len), "░".repeat(20 - bar_len), vol);
        lines.push(Line::from(vec![
            Span::styled("Volume:      ", Style::default().fg(Palette::MUTED)),
            Span::styled(bar, Style::default().fg(if stream.muted { Palette::RED } else { Palette::GREEN })),
        ]));
    }

    if stream.muted {
        lines.push(Line::from(Span::styled("             🔇 Muted", Style::default().fg(Palette::RED))));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled("─── Quick Actions ────────────────────", Style::default().fg(Palette::BORDER))));

        lines.push(Line::from(Span::styled(
            "  [Enter] Move to specific output",
            Style::default().fg(Palette::DIM),
        )));

    lines.push(Line::from(Span::styled("  [M] Move ALL streams to default", Style::default().fg(Palette::DIM))));
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled("─── Technical ────────────────────────", Style::default().fg(Palette::BORDER))));
    lines.push(Line::from(Span::styled(
        format!("  Stream index: {}", stream.index),
        Style::default().fg(Palette::MUTED),
    )));

    let text = ratatui::text::Text::from(lines);
    let p = Paragraph::new(text).style(Style::default().bg(Palette::BG_PANEL));
    f.render_widget(p, inner);
}

fn render_hints(f: &mut Frame, area: ratatui::layout::Rect) {
    let hints = hint_line(&[
        ("↑↓/jk", "navigate"),
        ("Enter", "move to default"),
        ("M", "move all here"),
        ("D", "go to devices"),
        ("F5", "refresh"),
        ("Esc", "back"),
    ]);
    let footer = Paragraph::new(hints)
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::BORDER)).bg(Palette::BG_PANEL));
    f.render_widget(footer, area);
}
