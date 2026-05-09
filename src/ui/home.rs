use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui::widgets::{hint_line, loading_indicator, panel_block, Palette};
use crate::audio::models::DeviceStatus;

pub fn render(f: &mut Frame, app: &App, tick: u64) {
    let area = f.area();

    // Outer layout: header | body | footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(0),     // body
            Constraint::Length(3),  // footer hint bar
        ])
        .split(area);

    render_header(f, app, chunks[0], tick);
    render_body(f, app, chunks[1]);
    render_footer(f, chunks[2]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect, tick: u64) {
    // Split header: logo | default sink info | status
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22),
            Constraint::Min(0),
            Constraint::Length(22),
        ])
        .split(area);

    // Logo
    let logo = Paragraph::new(Line::from(vec![
        Span::styled("🔊 ", Style::default()),
        Span::styled("AudioCtl", Style::default().fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::BORDER)).bg(Palette::BG_PANEL));
    f.render_widget(logo, h[0]);

    // Default sink
    let default_text = if app.audio_state.default_sink_name.is_empty() {
        "No default output set".to_string()
    } else {
        // Try to get display name
        let display = app.audio_state
            .device_by_name(&app.audio_state.default_sink_name)
            .map(|d| d.display_name().to_string())
            .unwrap_or_else(|| app.audio_state.default_sink_name.clone());
        format!("★  Default: {}", display)
    };

    let center = Paragraph::new(Line::from(Span::styled(
        default_text,
        Style::default().fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::BORDER)).bg(Palette::BG_PANEL));
    f.render_widget(center, h[1]);

    // Status / loading
    let status_text = if app.loading {
        format!("{} Updating…", loading_indicator(tick))
    } else {
        let sinks = app.audio_state.devices.len();
        let streams = app.audio_state.streams.len();
        format!("{} sinks  {} streams", sinks, streams)
    };

    let status = Paragraph::new(Line::from(Span::styled(
        status_text,
        Style::default().fg(Palette::DIM),
    )))
    .alignment(Alignment::Right)
    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::BORDER)).bg(Palette::BG_PANEL));
    f.render_widget(status, h[2]);
}

fn render_body(f: &mut Frame, app: &App, area: Rect) {
    // Three column layout: real devices | combined groups | streams
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .split(area);

    render_real_devices_panel(f, app, cols[0]);
    render_combined_panel(f, app, cols[1]);
    render_streams_panel(f, app, cols[2]);
}

fn render_real_devices_panel(f: &mut Frame, app: &App, area: Rect) {
    let real_devs = app.audio_state.real_devices();
    let block = panel_block("📡 Output Devices", true);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if real_devs.is_empty() {
        let msg = Paragraph::new("No physical devices found.\n\nPress [F5] to refresh.\n\nIs PipeWire running?")
            .style(Style::default().fg(Palette::MUTED))
            .alignment(Alignment::Center);
        f.render_widget(msg, inner);
        return;
    }

    let items: Vec<Line> = real_devs
        .iter()
        .map(|d| {
            let status_icon = match &d.status {
                DeviceStatus::Running => Span::styled("▶ ", Style::default().fg(Palette::GREEN)),
                DeviceStatus::Idle => Span::styled("⏸ ", Style::default().fg(Palette::CYAN)),
                DeviceStatus::Suspended => Span::styled("💤 ", Style::default().fg(Palette::MUTED)),
                DeviceStatus::Error(_) => Span::styled("⚠ ", Style::default().fg(Palette::RED)),
            };
            let default_marker = if d.is_default {
                Span::styled(" ★", Style::default().fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("")
            };
            let name_style = if d.is_default {
                Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Palette::TEXT)
            };
            let combined_mark = if d.combined_group_id.is_some() {
                Span::styled(" ⊕", Style::default().fg(Palette::YELLOW))
            } else {
                Span::raw("")
            };
            Line::from(vec![
                status_icon,
                Span::styled(d.display_name(), name_style),
                combined_mark,
                default_marker,
            ])
        })
        .collect();

    let text = ratatui::text::Text::from(items);
    let p = Paragraph::new(text).style(Style::default().bg(Palette::BG_PANEL));
    f.render_widget(p, inner);
}

fn render_combined_panel(f: &mut Frame, app: &App, area: Rect) {
    let groups = &app.audio_state.combined_groups;
    let block = panel_block("🔀 Combined Outputs", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if groups.is_empty() {
        let msg = Paragraph::new("No combined outputs.\n\nPress [C] on the\nDevices screen to\ncreate one.")
            .style(Style::default().fg(Palette::MUTED))
            .alignment(Alignment::Center);
        f.render_widget(msg, inner);
        return;
    }

    let items: Vec<Line> = groups
        .iter()
        .map(|g| {
            let default_mark = if g.is_default {
                Span::styled(" ★", Style::default().fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("")
            };
            let members_str = g.members.join(", ");
            let label = Span::styled(
                format!("⊕ {}", g.display_name()),
                Style::default().fg(Palette::YELLOW).add_modifier(Modifier::BOLD),
            );
            Line::from(vec![label, default_mark, Span::styled(format!(" [{}]", members_str.chars().take(25).collect::<String>()), Style::default().fg(Palette::DIM))])
        })
        .collect();

    let text = ratatui::text::Text::from(items);
    let p = Paragraph::new(text).style(Style::default().bg(Palette::BG_PANEL));
    f.render_widget(p, inner);
}

fn render_streams_panel(f: &mut Frame, app: &App, area: Rect) {
    let streams = &app.audio_state.streams;
    let block = panel_block("🎵 Active Streams", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if streams.is_empty() {
        let msg = Paragraph::new("No audio playing.\n\nStart an application\nto see streams here.")
            .style(Style::default().fg(Palette::MUTED))
            .alignment(Alignment::Center);
        f.render_widget(msg, inner);
        return;
    }

    let items: Vec<Line> = streams
        .iter()
        .map(|s| {
            let vol_str = s.volume.map(|v| format!(" {}%", v)).unwrap_or_default();
            let mute_icon = if s.muted { "🔇" } else { "🔊" };
            Line::from(vec![
                Span::styled(format!("{} ", mute_icon), Style::default()),
                Span::styled(s.display_name(), Style::default().fg(Palette::TEXT)),
                Span::styled(vol_str, Style::default().fg(Palette::DIM)),
            ])
        })
        .collect();

    let text = ratatui::text::Text::from(items);
    let p = Paragraph::new(text).style(Style::default().bg(Palette::BG_PANEL));
    f.render_widget(p, inner);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let hints = hint_line(&[
        ("D", "Devices"),
        ("S", "Streams"),
        ("C", "Combine"),
        ("?", "Help"),
        ("F5", "Refresh"),
        ("Q", "Quit"),
    ]);
    let footer = Paragraph::new(hints)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::BORDER)).bg(Palette::BG_PANEL));
    f.render_widget(footer, area);
}
