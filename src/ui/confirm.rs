use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{App, ConfirmAction};
use crate::ui::widgets::{hint_line, Palette};

/// Render the confirmation dialog overlay
pub fn render(f: &mut Frame, app: &App, action: &ConfirmAction) {
    let area = f.area();

    // Center a dialog box
    let dialog = centered_rect(60, 14, area);

    let (title, lines) = describe_action(app, action);

    let mut all_lines = lines;
    all_lines.push(Line::raw(""));
    all_lines.push(Line::from(Span::styled("─".repeat(40), Style::default().fg(Palette::BORDER))));
    all_lines.push(Line::raw(""));

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(Rect {
            x: dialog.x + 2,
            y: dialog.y + dialog.height.saturating_sub(4),
            width: dialog.width.saturating_sub(4),
            height: 3,
        });

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Palette::YELLOW))
        .style(Style::default().bg(Palette::BG_PANEL));

    let content = Paragraph::new(all_lines)
        .block(block)
        .style(Style::default().bg(Palette::BG_PANEL));

    f.render_widget(ratatui::widgets::Clear, dialog);
    f.render_widget(content, dialog);

    // Confirm button
    let yes = Paragraph::new("[Enter / Y]  Confirm")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Palette::BG).bg(Palette::GREEN).add_modifier(Modifier::BOLD));
    f.render_widget(yes, cols[0]);

    // Cancel button
    let no = Paragraph::new("[N / Esc]  Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Palette::TEXT).bg(Palette::BG_PANEL));
    f.render_widget(no, cols[1]);
}

fn describe_action<'a>(app: &App, action: &ConfirmAction) -> (&'static str, Vec<Line<'a>>) {
    match action {
        ConfirmAction::RemoveCombined { group_index } => {
            let group = app.audio_state.combined_groups.get(*group_index);
            let name = group.map(|g| g.display_name().to_string()).unwrap_or_default();
            let members = group.map(|g| g.members.join(", ")).unwrap_or_default();

            let lines = vec![
                Line::from(Span::styled(
                    "  ⚠  Remove combined output?",
                    Style::default().fg(Palette::YELLOW).add_modifier(Modifier::BOLD),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    format!("  \"{}\"", name),
                    Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("  Members: {}", members),
                    Style::default().fg(Palette::MUTED),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    "  Any streams playing through it will\n  need to be moved to another output.",
                    Style::default().fg(Palette::DIM),
                )),
            ];
            ("⚠ Confirm Removal", lines)
        }

        ConfirmAction::MoveAllStreams { sink_name } => {
            let display = app
                .audio_state
                .device_by_name(sink_name)
                .map(|d| d.display_name().to_string())
                .unwrap_or_else(|| sink_name.clone());
            let count = app.audio_state.streams.len();

            let lines = vec![
                Line::from(Span::styled(
                    format!("  Move all {} stream(s) to:", count),
                    Style::default().fg(Palette::TEXT),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    format!("  \"{}\"", display),
                    Style::default().fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    "  All currently playing audio will be\n  sent to this output.",
                    Style::default().fg(Palette::DIM),
                )),
            ];
            ("Move Streams", lines)
        }

        ConfirmAction::SetDefault { sink_name, move_streams } => {
            let display = app
                .audio_state
                .device_by_name(sink_name)
                .map(|d| d.display_name().to_string())
                .unwrap_or_else(|| sink_name.clone());

            let mut lines = vec![
                Line::from(Span::styled(
                    "  Set as the default output:",
                    Style::default().fg(Palette::TEXT),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    format!("  ★  \"{}\"", display),
                    Style::default().fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD),
                )),
                Line::raw(""),
            ];

            if *move_streams {
                lines.push(Line::from(Span::styled(
                    "  All current streams will also be moved.",
                    Style::default().fg(Palette::GREEN),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "  Current streams will stay where they are.",
                    Style::default().fg(Palette::DIM),
                )));
            }

            ("Set Default Output", lines)
        }

        ConfirmAction::ResumeDevice { device_index } => {
            let device = app.audio_state.devices.get(*device_index);
            let name = device.map(|d| d.display_name().to_string()).unwrap_or_default();

            let lines = vec![
                Line::from(Span::styled("  Resume suspended device?", Style::default().fg(Palette::TEXT))),
                Line::raw(""),
                Line::from(Span::styled(
                    format!("  \"{}\"", name),
                    Style::default().fg(Palette::CYAN).add_modifier(Modifier::BOLD),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    "  This will try to wake up the device.\n  If it's Bluetooth, ensure it is on.",
                    Style::default().fg(Palette::DIM),
                )),
            ];
            ("Resume Device", lines)
        }
    }
}

/// Return a centered Rect with given percentage dimensions
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let width = (area.width * percent_x / 100).min(area.width);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    Rect { x: area.x + x, y: area.y + y, width, height }
}
