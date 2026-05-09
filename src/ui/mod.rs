pub mod combine;
pub mod confirm;
pub mod devices;
pub mod help;
pub mod home;
pub mod streams;
pub mod widgets;

pub mod select_device;

use ratatui::Frame;
use crate::app::{App, Screen};
use crate::ui::widgets::render_notification;

/// Top-level render dispatch
pub fn render(f: &mut Frame, app: &App, tick: u64) {
    // Fill background
    let area = f.area();
    f.render_widget(
        ratatui::widgets::Block::default()
            .style(ratatui::style::Style::default().bg(widgets::Palette::BG)),
        area,
    );

    match &app.screen {
        Screen::Home => home::render(f, app, tick),
        Screen::Devices => devices::render(f, app),
        Screen::Streams => streams::render(f, app),
        Screen::Combine => combine::render(f, app),
        Screen::Help => help::render(f),
        Screen::Confirm(action) => {
            // Render underlying screen then overlay dialog
            devices::render(f, app);
            confirm::render(f, app, action);
        }
        Screen::SelectDevice { stream_index, cursor } => {
            // Render underlying screen then overlay dialog
            streams::render(f, app);
            select_device::render(f, app, *stream_index, *cursor);
        }
    }

    // Notification banner always on top
    render_notification(f, app);
}
