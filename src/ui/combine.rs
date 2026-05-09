use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, CombineStepState};
use crate::ui::widgets::{hint_line, panel_block, device_type_color, Palette};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // wizard step header
            Constraint::Min(0),    // main content
            Constraint::Length(4), // hints
        ])
        .split(area);

    render_wizard_header(f, app, chunks[0]);

    match app.combine_wizard.step {
        CombineStepState::SelectDevices => render_select_step(f, app, chunks[1]),
        CombineStepState::NameGroup => render_name_step(f, app, chunks[1]),
        CombineStepState::Confirm => render_confirm_step(f, app, chunks[1]),
    }

    render_hints(f, app, chunks[2]);
}

fn render_wizard_header(f: &mut Frame, app: &App, area: Rect) {
    let steps = [
        (CombineStepState::SelectDevices, "1 · Choose Devices"),
        (CombineStepState::NameGroup, "2 · Name Group"),
        (CombineStepState::Confirm, "3 · Confirm & Apply"),
    ];

    let constraints: Vec<Constraint> = steps.iter().map(|_| Constraint::Percentage(33)).collect();
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (i, (step, label)) in steps.iter().enumerate() {
        let is_current = &app.combine_wizard.step == step;
        let is_done = match (&app.combine_wizard.step, step) {
            (CombineStepState::NameGroup, CombineStepState::SelectDevices) => true,
            (CombineStepState::Confirm, CombineStepState::SelectDevices) => true,
            (CombineStepState::Confirm, CombineStepState::NameGroup) => true,
            _ => false,
        };

        let (fg, bg, prefix) = if is_current {
            (Palette::BG, Palette::ACCENT, "▶ ")
        } else if is_done {
            (Palette::BG, Palette::GREEN, "✓ ")
        } else {
            (Palette::MUTED, Palette::BG_PANEL, "  ")
        };

        let p = Paragraph::new(format!("{}{}", prefix, label))
            .alignment(Alignment::Center)
            .style(Style::default().fg(fg).bg(bg).add_modifier(if is_current { Modifier::BOLD } else { Modifier::empty() }))
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(if is_current { Palette::ACCENT } else { Palette::BORDER })));
        f.render_widget(p, cols[i]);
    }
}

fn render_select_step(f: &mut Frame, app: &App, area: Rect) {
    let real_devs = app.audio_state.real_devices();
    let selected_count = app.combine_wizard.selected_device_indices.len();

    let title = format!("🔊 Select devices to combine ({} selected)", selected_count);
    let block = panel_block(&title, true);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if real_devs.is_empty() {
        let p = Paragraph::new("No physical output devices found.\nConnect a device and press F5 to refresh.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Palette::MUTED));
        f.render_widget(p, inner);
        return;
    }

    let items: Vec<ListItem> = real_devs
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let selected = app.combine_wizard.selected_device_indices.contains(&i);
            let check = if selected {
                Span::styled("[✓] ", Style::default().fg(Palette::GREEN).add_modifier(Modifier::BOLD))
            } else {
                Span::styled("[ ] ", Style::default().fg(Palette::MUTED))
            };
            let name_style = if selected {
                Style::default().fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Palette::TEXT)
            };
            let type_color = device_type_color(&d.device_type);
            ListItem::new(vec![
                Line::from(vec![
                    check,
                    Span::styled(format!("[{}] ", d.type_badge()), Style::default().fg(type_color)),
                    Span::styled(d.display_name(), name_style),
                ]),
                Line::from(Span::styled(
                    format!("     ↳ {}", d.status.display_label()),
                    Style::default().fg(Palette::MUTED),
                )),
            ])
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.device_cursor));

    let list = List::new(items)
        .highlight_style(Style::default().bg(Palette::BG_SELECTED).fg(Palette::ACCENT_BRIGHT).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, inner, &mut list_state);
}

fn render_name_step(f: &mut Frame, app: &App, area: Rect) {
    // Split: name input + options + preview
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // name input
            Constraint::Length(5),  // options
            Constraint::Min(0),     // preview
        ])
        .split(area);

    // Name input box
    {
        let cursor = app.combine_wizard.name_cursor;
        let text = &app.combine_wizard.group_name;
        let mut display = text.clone();
        // Insert cursor marker
        if cursor <= display.len() {
            display.insert(cursor, '│');
        }
        let valid = app.combine_name_is_valid();
        let border_color = if valid { Palette::GREEN } else { Palette::RED };
        let hint = if valid {
            "✓ Valid name"
        } else {
            "Letters, numbers, _ and - only"
        };

        let p = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  Name: ", Style::default().fg(Palette::MUTED)),
                Span::styled(&display, Style::default().fg(Palette::TEXT).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("         ", Style::default()),
                Span::styled(hint, Style::default().fg(if valid { Palette::GREEN } else { Palette::YELLOW })),
            ]),
        ])
        .block(
            Block::default()
                .title(" 📝 Group Name ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .bg(Palette::BG_PANEL),
        );
        f.render_widget(p, rows[0]);
    }

    // Options
    {
        let default_check = if app.combine_wizard.set_as_default { "✓" } else { " " };
        let p = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!("  [{}] ", default_check), Style::default().fg(Palette::ACCENT)),
                Span::styled("Set as main output after creation", Style::default().fg(Palette::TEXT)),
            ]),
            Line::from(Span::styled("       (Tab to toggle)", Style::default().fg(Palette::MUTED))),
        ])
        .block(
            Block::default()
                .title(" ⚙ Options ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Palette::BORDER))
                .bg(Palette::BG_PANEL),
        );
        f.render_widget(p, rows[1]);
    }

    // Preview
    {
        let devices = app.combine_selected_devices();
        let sink_name = app.combine_sink_name();
        let mut preview_lines = vec![
            Line::from(Span::styled("  Will create:", Style::default().fg(Palette::MUTED))),
            Line::from(Span::styled(
                format!("  Combined output: \"{}\"", app.combine_wizard.group_name.trim()),
                Style::default().fg(Palette::YELLOW).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("  Sink name: {}", sink_name),
                Style::default().fg(Palette::MUTED),
            )),
            Line::from(Span::styled("  Members:", Style::default().fg(Palette::MUTED))),
        ];
        for d in &devices {
            preview_lines.push(Line::from(Span::styled(
                format!("    • {}", d.display_name()),
                Style::default().fg(Palette::TEXT),
            )));
        }
        if app.combine_wizard.set_as_default {
            preview_lines.push(Line::from(Span::styled(
                "  → Will be set as the default output",
                Style::default().fg(Palette::GREEN),
            )));
        }

        let p = Paragraph::new(preview_lines)
            .block(
                Block::default()
                    .title(" 👁 Preview ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Palette::BORDER))
                    .bg(Palette::BG_PANEL),
            );
        f.render_widget(p, rows[2]);
    }
}

fn render_confirm_step(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(area);

    // Summary
    {
        let devices = app.combine_selected_devices();
        let sink_name = app.combine_sink_name();
        let default_check = if app.combine_wizard.set_as_default { "✓" } else { " " };
        let move_check = if app.combine_wizard.move_streams { "✓" } else { " " };

        let mut lines = vec![
            Line::from(Span::styled("  You are about to:", Style::default().fg(Palette::MUTED))),
            Line::raw(""),
            Line::from(Span::styled(
                format!("  Create combined output: \"{}\"", app.combine_wizard.group_name.trim()),
                Style::default().fg(Palette::YELLOW).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("  Sink name:              {}", sink_name),
                Style::default().fg(Palette::MUTED),
            )),
            Line::raw(""),
            Line::from(Span::styled("  Combining these outputs:", Style::default().fg(Palette::MUTED))),
        ];
        for d in &devices {
            lines.push(Line::from(Span::styled(
                format!("    • {}", d.display_name()),
                Style::default().fg(Palette::TEXT),
            )));
        }
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled(format!("  [{}] ", default_check), Style::default().fg(Palette::ACCENT)),
            Span::styled("Set as default output", Style::default().fg(Palette::TEXT)),
            Span::styled("  (d to toggle)", Style::default().fg(Palette::MUTED)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(format!("  [{}] ", move_check), Style::default().fg(Palette::ACCENT)),
            Span::styled("Move current streams here too", Style::default().fg(Palette::TEXT)),
            Span::styled("  (Tab to toggle)", Style::default().fg(Palette::MUTED)),
        ]));

        let p = Paragraph::new(lines)
            .block(panel_block("📋 Summary", true));
        f.render_widget(p, rows[0]);
    }

    // Confirm buttons
    {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        let yes = Paragraph::new(Line::from(vec![
            Span::styled("[Enter / Y] ", Style::default().fg(Palette::BG).add_modifier(Modifier::BOLD)),
            Span::styled("Yes, create it!", Style::default().fg(Palette::BG).add_modifier(Modifier::BOLD)),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::GREEN)).bg(Palette::GREEN).style(Style::default().bg(Palette::GREEN)));
        f.render_widget(yes, cols[0]);

        let no = Paragraph::new(Line::from(Span::styled(
            "[N / Esc]  Cancel",
            Style::default().fg(Palette::TEXT),
        )))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::BORDER)).bg(Palette::BG_PANEL));
        f.render_widget(no, cols[1]);
    }
}

fn render_hints(f: &mut Frame, app: &App, area: Rect) {
    let hints = match app.combine_wizard.step {
        CombineStepState::SelectDevices => hint_line(&[
            ("↑↓/jk", "navigate"),
            ("Space", "select/deselect"),
            ("Enter", "next (need ≥2)"),
            ("Esc", "cancel"),
        ]),
        CombineStepState::NameGroup => hint_line(&[
            ("Type", "group name"),
            ("Tab", "toggle default"),
            ("Enter", "next"),
            ("Esc", "back"),
        ]),
        CombineStepState::Confirm => hint_line(&[
            ("Enter/Y", "create"),
            ("Tab", "toggle move streams"),
            ("D", "toggle default"),
            ("N/Esc", "cancel"),
        ]),
    };

    let footer = Paragraph::new(hints)
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::BORDER)).bg(Palette::BG_PANEL));
    f.render_widget(footer, area);
}
