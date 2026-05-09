use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui::widgets::{panel_block, Palette};

pub fn render(f: &mut Frame, app: &App, stream_index: u32, cursor: usize) {
    let area = f.area();

    // Overlay is 60 chars wide, 15 chars high, centered
    let popup_width = 60;
    let popup_height = 18;

    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width.min(area.width), popup_height.min(area.height));

    // Clear background
    f.render_widget(Clear, popup_area);

    let stream = app.audio_state.streams.iter().find(|s| s.index == stream_index);
    let stream_name = stream.map(|s| s.display_name().to_string()).unwrap_or_else(|| "Unknown Stream".to_string());

    let title_str = format!(" Move '{}' To... ", stream_name);
    let block = panel_block(&title_str, true)
        .border_style(Style::default().fg(Palette::ACCENT_BRIGHT));
    
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);

    let devices = &app.audio_state.devices;
    
    if devices.is_empty() {
        let p = Paragraph::new("No available outputs.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Palette::MUTED));
        f.render_widget(p, chunks[0]);
    } else {
        let items: Vec<ListItem> = devices.iter().map(|d| {
            let on_default = app.audio_state.default_sink_name == d.name;
            let current = stream.map(|s| s.sink_index == d.index).unwrap_or(false);

            let prefix = if current { " (Current) " } else { "" };
            let suffix = if on_default { " ★" } else { "" };

            let name_style = if current {
                Style::default().fg(Palette::GREEN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Palette::TEXT)
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Palette::YELLOW)),
                Span::styled(d.display_name(), name_style),
                Span::styled(suffix, Style::default().fg(Palette::ACCENT)),
            ]))
        }).collect();

        let mut state = ListState::default();
        state.select(Some(cursor));

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Palette::BG_SELECTED)
                    .fg(Palette::ACCENT_BRIGHT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");

        f.render_stateful_widget(list, chunks[0], &mut state);
    }

    let help_text = Paragraph::new("↑↓: Select   Enter: Move   Esc: Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Palette::DIM));
    f.render_widget(help_text, chunks[1]);
}
