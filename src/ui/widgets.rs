use ratatui::prelude::Stylize;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, Notification, NotificationLevel};

// ─── Color palette ────────────────────────────────────────────────────────────

pub struct Palette;

impl Palette {
    pub const BG: Color = Color::Rgb(15, 17, 26);
    pub const BG_PANEL: Color = Color::Rgb(22, 25, 37);
    pub const BG_SELECTED: Color = Color::Rgb(35, 40, 60);
    pub const BG_HIGHLIGHT: Color = Color::Rgb(50, 60, 90);

    pub const ACCENT: Color = Color::Rgb(120, 160, 255);
    pub const ACCENT_BRIGHT: Color = Color::Rgb(160, 200, 255);
    pub const GREEN: Color = Color::Rgb(100, 220, 130);
    pub const YELLOW: Color = Color::Rgb(255, 210, 100);
    pub const RED: Color = Color::Rgb(255, 100, 100);
    pub const ORANGE: Color = Color::Rgb(255, 160, 80);
    pub const CYAN: Color = Color::Rgb(100, 220, 220);
    pub const PURPLE: Color = Color::Rgb(180, 130, 255);
    pub const MUTED: Color = Color::Rgb(100, 110, 140);
    pub const TEXT: Color = Color::Rgb(210, 215, 230);
    pub const DIM: Color = Color::Rgb(130, 140, 165);
    pub const BORDER: Color = Color::Rgb(55, 65, 95);
    pub const BORDER_ACTIVE: Color = Color::Rgb(90, 120, 200);
}

// ─── Styled block builder ────────────────────────────────────────────────────

pub fn panel_block(title: &str, active: bool) -> Block<'_> {
    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if active { Palette::BORDER_ACTIVE } else { Palette::BORDER }))
        .style(Style::default().bg(Palette::BG_PANEL))
}

pub fn plain_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Palette::BORDER))
        .style(Style::default().bg(Palette::BG_PANEL))
}

// ─── Status indicator ─────────────────────────────────────────────────────────

pub fn status_style(status: &crate::audio::models::DeviceStatus) -> Style {
    use crate::audio::models::DeviceStatus::*;
    match status {
        Running => Style::default().fg(Palette::GREEN),
        Idle => Style::default().fg(Palette::CYAN),
        Suspended => Style::default().fg(Palette::MUTED),
        Error(_) => Style::default().fg(Palette::RED),
    }
}

pub fn device_type_color(dt: &crate::audio::models::DeviceType) -> Color {
    use crate::audio::models::DeviceType::*;
    match dt {
        Physical => Palette::ACCENT,
        Bluetooth => Palette::PURPLE,
        Combined => Palette::YELLOW,
        Virtual => Palette::MUTED,
        Unknown => Palette::DIM,
    }
}

// ─── Notification banner ──────────────────────────────────────────────────────

pub fn render_notification(f: &mut Frame, app: &App) {
    if let Some(ref n) = app.notification {
        let (icon, fg, bg) = match n.level {
            NotificationLevel::Info => ("ℹ ", Palette::TEXT, Color::Rgb(30, 40, 70)),
            NotificationLevel::Success => ("✓ ", Palette::GREEN, Color::Rgb(20, 50, 30)),
            NotificationLevel::Warning => ("⚠ ", Palette::YELLOW, Color::Rgb(50, 40, 10)),
            NotificationLevel::Error => ("✗ ", Palette::RED, Color::Rgb(50, 15, 15)),
        };

        let msg = format!("{}{}", icon, n.message);
        let area = notification_area(f.area());

        let widget = Paragraph::new(msg)
            .style(Style::default().fg(fg).bg(bg))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(fg).bg(bg)),
            );

        f.render_widget(Clear, area);
        f.render_widget(widget, area);
    }
}

fn notification_area(screen: Rect) -> Rect {
    let width = screen.width.saturating_sub(4).min(80);
    let height = 3;
    Rect {
        x: screen.width.saturating_sub(width + 2),
        y: screen.height.saturating_sub(height + 1),
        width,
        height,
    }
}

// ─── Key hint bar ─────────────────────────────────────────────────────────────

pub fn hint_line<'a>(hints: &[(&'a str, &'a str)]) -> Line<'a> {
    let mut spans = Vec::new();
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(
            format!("[{}]", key),
            Style::default().fg(Palette::ACCENT).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(Palette::DIM),
        ));
    }
    Line::from(spans)
}

// ─── Loading spinner ─────────────────────────────────────────────────────────

pub fn loading_indicator(tick: u64) -> &'static str {
    const FRAMES: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
    FRAMES[(tick as usize) % FRAMES.len()]
}

// ─── Badge ────────────────────────────────────────────────────────────────────

pub fn default_badge<'a>() -> Span<'a> {
    Span::styled(
        " ★ DEFAULT ",
        Style::default()
            .fg(Palette::BG)
            .bg(Palette::ACCENT_BRIGHT)
            .add_modifier(Modifier::BOLD),
    )
}

pub fn combined_badge<'a>() -> Span<'a> {
    Span::styled(
        " ⊕ COMBINED ",
        Style::default()
            .fg(Palette::BG)
            .bg(Palette::YELLOW)
            .add_modifier(Modifier::BOLD),
    )
}

pub fn bt_badge<'a>() -> Span<'a> {
    Span::styled(
        " ⌘ BT ",
        Style::default()
            .fg(Palette::BG)
            .bg(Palette::PURPLE)
            .add_modifier(Modifier::BOLD),
    )
}
