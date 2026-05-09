use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::ui::widgets::{hint_line, panel_block, Palette};

pub fn render(f: &mut Frame) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled("🔊 AudioCtl", Style::default().fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD)),
        Span::styled(" — Help & Keyboard Reference", Style::default().fg(Palette::DIM)),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::BORDER)).bg(Palette::BG_PANEL));
    f.render_widget(title, chunks[0]);

    // Split help into columns
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    render_navigation_column(f, cols[0]);
    render_actions_column(f, cols[1]);

    // Footer
    let footer = Paragraph::new(hint_line(&[("Esc / ?", "close help")]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::BORDER)).bg(Palette::BG_PANEL));
    f.render_widget(footer, chunks[2]);
}

fn key_row<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:14}", key), Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(desc, Style::default().fg(Palette::TEXT)),
    ])
}

fn section<'a>(label: &'a str) -> Line<'a> {
    Line::from(Span::styled(
        format!("  ── {} ", label),
        Style::default().fg(Palette::MUTED).add_modifier(Modifier::BOLD),
    ))
}

fn render_navigation_column(f: &mut Frame, area: ratatui::layout::Rect) {
    let lines = vec![
        section("Navigation"),
        key_row("↑ / k", "Move up"),
        key_row("↓ / j", "Move down"),
        key_row("Enter", "Select / confirm"),
        key_row("Space", "Toggle selection"),
        key_row("Tab", "Toggle option"),
        key_row("Esc", "Go back / cancel"),
        Line::raw(""),
        section("Screens"),
        key_row("D", "Devices browser"),
        key_row("S", "Active streams"),
        key_row("C", "Create combined output"),
        key_row("?  or  F1", "This help screen"),
        key_row("F5", "Refresh audio state"),
        key_row("Q", "Quit"),
        Line::raw(""),
        section("Search & Filter"),
        key_row("/  or  F", "Open filter (Devices)"),
        key_row("Type…", "Filter by name/type"),
        key_row("Esc", "Clear filter"),
    ];

    let block = panel_block("⌨ Keyboard Shortcuts", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = ratatui::text::Text::from(lines);
    let p = Paragraph::new(text).style(Style::default().bg(Palette::BG_PANEL));
    f.render_widget(p, inner);
}

fn render_actions_column(f: &mut Frame, area: ratatui::layout::Rect) {
    let lines = vec![
        section("Device Actions"),
        key_row("d", "Set device as default output"),
        key_row("D", "Set default + move all streams"),
        key_row("m", "Move all streams to this device"),
        key_row("r", "Resume suspended device"),
        key_row("R", "Remove combined output"),
        key_row("C", "Go to 'Create combined' wizard"),
        Line::raw(""),
        section("Stream Actions"),
        key_row("Enter", "Move stream to default output"),
        key_row("M", "Move ALL streams to default"),
        Line::raw(""),
        section("Combine Wizard"),
        key_row("Space", "Toggle device selection"),
        key_row("Enter", "Proceed to next step"),
        key_row("Tab", "Toggle options"),
        key_row("Y / Enter", "Confirm & create"),
        key_row("N / Esc", "Cancel"),
        Line::raw(""),
        section("About"),
        Line::from(Span::styled("  AudioCtl — PipeWire/PulseAudio", Style::default().fg(Palette::MUTED))),
        Line::from(Span::styled("  TUI for Arch Linux. Uses pactl.", Style::default().fg(Palette::MUTED))),
    ];

    let block = panel_block("⚡ Actions Reference", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = ratatui::text::Text::from(lines);
    let p = Paragraph::new(text).style(Style::default().bg(Palette::BG_PANEL));
    f.render_widget(p, inner);
}
