use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;
use crate::audio::models::{DeviceStatus, DeviceType};
use crate::ui::widgets::{hint_line, panel_block, status_style, device_type_color, Palette};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // main list + detail
            Constraint::Length(3),  // hints
        ])
        .split(area);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        ])
        .split(chunks[0]);

    render_device_list(f, app, body[0]);
    render_device_detail(f, app, body[1]);
    render_hints(f, app, chunks[1]);
}

fn render_device_list(f: &mut Frame, app: &App, area: Rect) {
    let filtered = app.filtered_devices();

    // Title with filter hint
    let title = if app.filter_active {
        format!("🔍 Filter: {}█", app.filter_text)
    } else if !app.filter_text.is_empty() {
        format!("📡 Devices [filter: {}]", app.filter_text)
    } else {
        "📡 Output Devices".to_string()
    };

    let block = panel_block(&title, true);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if filtered.is_empty() {
        let msg = if app.filter_text.is_empty() {
            "No devices found.\n\nPress [F5] to refresh.\nIs PipeWire running?"
        } else {
            "No devices match your filter.\nPress [Esc] to clear."
        };
        let p = Paragraph::new(msg)
            .style(Style::default().fg(Palette::MUTED))
            .alignment(Alignment::Center);
        f.render_widget(p, inner);
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(_, d)| {
            let status_icon = match &d.status {
                DeviceStatus::Running => Span::styled("▶ ", Style::default().fg(Palette::GREEN)),
                DeviceStatus::Idle => Span::styled("⏸ ", Style::default().fg(Palette::CYAN)),
                DeviceStatus::Suspended => Span::styled("💤 ", Style::default().fg(Palette::MUTED)),
                DeviceStatus::Error(_) => Span::styled("⚠ ", Style::default().fg(Palette::RED)),
            };

            let type_color = device_type_color(&d.device_type);
            let type_badge = Span::styled(
                format!("[{}] ", d.type_badge()),
                Style::default().fg(type_color),
            );

            let name_style = if d.is_default {
                Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Palette::TEXT)
            };

            let default_star = if d.is_default {
                Span::styled(" ★", Style::default().fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("")
            };

            let combined_mark = if d.combined_group_id.is_some() {
                Span::styled(" ⊕", Style::default().fg(Palette::YELLOW))
            } else {
                Span::raw("")
            };

            let mute_mark = if d.muted {
                Span::styled(" 🔇", Style::default())
            } else {
                Span::raw("")
            };

            ListItem::new(Line::from(vec![
                status_icon,
                type_badge,
                Span::styled(d.display_name(), name_style),
                default_star,
                combined_mark,
                mute_mark,
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.device_cursor));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Palette::BG_SELECTED)
                .fg(Palette::ACCENT_BRIGHT)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, inner, &mut list_state);
}

fn render_device_detail(f: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Device Details", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let device = match app.selected_device() {
        Some(d) => d,
        None => {
            let p = Paragraph::new("Select a device to view details.")
                .style(Style::default().fg(Palette::MUTED))
                .alignment(Alignment::Center);
            f.render_widget(p, inner);
            return;
        }
    };

    let mut lines = vec![
        // Name
        Line::from(vec![
            Span::styled(device.display_name(), Style::default().fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD)),
            if device.is_default {
                Span::styled("  ★ Default", Style::default().fg(Palette::ACCENT))
            } else {
                Span::raw("")
            },
        ]),
        Line::raw(""),
        // Type
        Line::from(vec![
            Span::styled("Type:      ", Style::default().fg(Palette::MUTED)),
            Span::styled(device.device_type.display_label(), Style::default().fg(device_type_color(&device.device_type))),
        ]),
        // Status
        Line::from(vec![
            Span::styled("Status:    ", Style::default().fg(Palette::MUTED)),
            Span::styled(device.status.display_label(), status_style(&device.status)),
        ]),
    ];

    // Volume
    if let Some(vol) = device.volume {
        let bar_len = (vol as usize * 20) / 100;
        let bar = format!("[{}{}] {}%",
            "█".repeat(bar_len),
            "░".repeat(20 - bar_len),
            vol
        );
        lines.push(Line::from(vec![
            Span::styled("Volume:    ", Style::default().fg(Palette::MUTED)),
            Span::styled(bar, Style::default().fg(if device.muted { Palette::RED } else { Palette::GREEN })),
        ]));
        if device.muted {
            lines.push(Line::from(Span::styled("           🔇 Muted", Style::default().fg(Palette::RED))));
        }
    }

    // Sample rate
    if let Some(ref rate) = device.sample_rate {
        lines.push(Line::from(vec![
            Span::styled("Rate:      ", Style::default().fg(Palette::MUTED)),
            Span::styled(rate, Style::default().fg(Palette::DIM)),
        ]));
    }

    // Channels
    if let Some(ch) = device.channels {
        let ch_label = match ch {
            1 => "Mono".to_string(),
            2 => "Stereo".to_string(),
            n => format!("{} channels", n),
        };
        lines.push(Line::from(vec![
            Span::styled("Channels:  ", Style::default().fg(Palette::MUTED)),
            Span::styled(ch_label, Style::default().fg(Palette::DIM)),
        ]));
    }

    // Combined membership
    if let Some(group_id) = device.combined_group_id {
        let group_name = app.audio_state.combined_groups
            .iter()
            .find(|g| g.module_index == group_id)
            .map(|g| g.display_name().to_string())
            .unwrap_or_else(|| format!("Group #{}", group_id));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("Part of:   ", Style::default().fg(Palette::MUTED)),
            Span::styled(group_name, Style::default().fg(Palette::YELLOW)),
        ]));
    }

    // Active streams on this device
    let streams_here = app.audio_state.streams_on_sink(device.index);
    if !streams_here.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled("Streams:", Style::default().fg(Palette::MUTED))));
        for s in streams_here.iter().take(5) {
            lines.push(Line::from(Span::styled(
                format!("  🎵 {}", s.display_name()),
                Style::default().fg(Palette::DIM),
            )));
        }
        if streams_here.len() > 5 {
            lines.push(Line::from(Span::styled(
                format!("  … and {} more", streams_here.len() - 5),
                Style::default().fg(Palette::MUTED),
            )));
        }
    }

    // Actions hint
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled("─── Actions ─────────────────────", Style::default().fg(Palette::BORDER))));

    if device.is_combined() {
        lines.push(Line::from(Span::styled("  [R] Remove this combined output", Style::default().fg(Palette::DIM))));
    } else if matches!(device.status, DeviceStatus::Suspended) {
        lines.push(Line::from(Span::styled("  [r] Resume (wake up) this device", Style::default().fg(Palette::DIM))));
    }
    if !device.is_default {
        lines.push(Line::from(Span::styled("  [d] Use as main output (default only)", Style::default().fg(Palette::DIM))));
        lines.push(Line::from(Span::styled("  [D] Use as main + move all streams", Style::default().fg(Palette::DIM))));
    }
    lines.push(Line::from(Span::styled("  [m] Move all streams here", Style::default().fg(Palette::DIM))));

    // Internal name (for advanced users)
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled("─── Technical ──────────────────────", Style::default().fg(Palette::BORDER))));
    lines.push(Line::from(Span::styled(
        format!("  ID: {}", device.name),
        Style::default().fg(Palette::MUTED),
    )));

    let text = ratatui::text::Text::from(lines);
    let p = Paragraph::new(text)
        .style(Style::default().bg(Palette::BG_PANEL));
    f.render_widget(p, inner);
}

fn render_hints(f: &mut Frame, app: &App, area: Rect) {
    let hints = if app.filter_active {
        hint_line(&[
            ("Type", "filter"),
            ("Esc", "clear filter"),
            ("↑↓", "move"),
        ])
    } else {
        let device = app.selected_device();
        let is_combined = device.map(|d| d.is_combined()).unwrap_or(false);
        let is_suspended = device.map(|d| matches!(d.status, DeviceStatus::Suspended)).unwrap_or(false);

        let mut hints_vec: Vec<(&str, &str)> = vec![
            ("↑↓/jk", "navigate"),
            ("/", "filter"),
            ("d", "set default"),
            ("D", "default+streams"),
            ("m", "move all streams"),
            ("C", "combine"),
        ];
        if is_combined {
            hints_vec.push(("R", "remove combined"));
        }
        if is_suspended {
            hints_vec.push(("r", "resume device"));
        }
        hints_vec.push(("Esc", "back"));
        hint_line(&hints_vec)
    };

    let footer = Paragraph::new(hints)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::BORDER)).bg(Palette::BG_PANEL));
    f.render_widget(footer, area);
}
