use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, CombineStepState, ConfirmAction, Notification, Screen};
use crate::dlog;

/// Handle a crossterm event, modifying app state as needed.
/// Returns true if a refresh is needed after the event.
pub async fn handle_event(app: &mut App, event: Event) -> bool {
    if let Event::Key(key) = event {
        // Log every key event with screen context
        dlog!("KEY", "screen={:?} code={:?} modifiers={:?}", app.screen, key.code, key.modifiers);
        // Global quit
        if matches!(key, KeyEvent { code: KeyCode::Char('q'), modifiers: KeyModifiers::NONE, .. })
            || matches!(key, KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. })
        {
            // Don't quit if filter is active or in combine name step
            if !app.filter_active && app.combine_wizard.step != CombineStepState::NameGroup {
                app.should_quit = true;
                return false;
            }
        }

        // Delegate to screen-specific handler
        let needs_refresh = match app.screen.clone() {
            Screen::Home => handle_home(app, key).await,
            Screen::Devices => handle_devices(app, key).await,
            Screen::Streams => handle_streams(app, key).await,
            Screen::Combine => handle_combine(app, key).await,
            Screen::Help => handle_help(app, key),
            Screen::Confirm(action) => handle_confirm(app, key, action).await,
            Screen::SelectDevice { stream_index, cursor } => handle_select_device(app, key, stream_index, cursor).await,
        };

        needs_refresh
    } else {
        false
    }
}

// ─── Home ─────────────────────────────────────────────────────────────────────

async fn handle_home(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('d') | KeyCode::Char('D') => {
            app.navigate_to(Screen::Devices);
            false
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            app.navigate_to(Screen::Streams);
            false
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.navigate_to(Screen::Combine);
            false
        }
        KeyCode::Char('?') | KeyCode::F(1) => {
            app.navigate_to(Screen::Help);
            false
        }
        KeyCode::Char('r') | KeyCode::F(5) => {
            app.action_refresh().await;
            true
        }
        _ => false,
    }
}

// ─── Devices ──────────────────────────────────────────────────────────────────

async fn handle_devices(app: &mut App, key: KeyEvent) -> bool {
    // Filter input mode
    if app.filter_active {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                app.filter_active = false;
            }
            KeyCode::Backspace => {
                app.filter_text.pop();
                app.device_cursor = 0;
                app.device_scroll_offset = 0;
            }
            KeyCode::Char(c) => {
                app.filter_text.push(c);
                app.device_cursor = 0;
                app.device_scroll_offset = 0;
            }
            _ => {}
        }
        return false;
    }

    match key.code {
        KeyCode::Esc => {
            app.go_home();
            false
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.device_cursor > 0 {
                app.device_cursor -= 1;
            }
            false
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = app.filtered_devices().len().saturating_sub(1);
            if app.device_cursor < max {
                app.device_cursor += 1;
            }
            false
        }
        KeyCode::Char('f') | KeyCode::Char('/') => {
            app.filter_active = true;
            false
        }
        KeyCode::Char('d') => {
            // Set selected device as default (just default, ask about streams)
            if let Some(device) = app.selected_device() {
                let sink_name = device.name.clone();
                let action = ConfirmAction::SetDefault { sink_name, move_streams: false };
                app.screen = Screen::Confirm(action);
            }
            false
        }
        KeyCode::Char('D') => {
            // Set default AND move streams
            if let Some(device) = app.selected_device() {
                let sink_name = device.name.clone();
                let action = ConfirmAction::SetDefault { sink_name, move_streams: true };
                app.screen = Screen::Confirm(action);
            }
            false
        }
        KeyCode::Char('r') => {
            // Resume suspended device
            if let Some(device) = app.selected_device() {
                if matches!(device.status, crate::audio::models::DeviceStatus::Suspended) {
                    let device_index = app.filtered_devices()
                        .get(app.device_cursor)
                        .map(|(i, _)| *i)
                        .unwrap_or(0);
                    app.action_resume_device(device_index).await;
                    return true;
                }
            }
            false
        }
        KeyCode::Char('R') => {
            // Remove selected combined group
            if let Some(device) = app.selected_device() {
                if device.is_combined() {
                    // find group index by sink name
                    let device_name = device.name.clone();
                    if let Some(group_idx) = app.audio_state.combined_groups
                        .iter()
                        .position(|g| g.sink_name == device_name)
                    {
                        let action = ConfirmAction::RemoveCombined { group_index: group_idx };
                        app.screen = Screen::Confirm(action);
                    }
                }
            }
            false
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            // Move all streams to selected device
            if let Some(device) = app.selected_device() {
                let sink_name = device.name.clone();
                let action = ConfirmAction::MoveAllStreams { sink_name };
                app.screen = Screen::Confirm(action);
            }
            false
        }
        KeyCode::Char('c') => {
            app.navigate_to(Screen::Combine);
            false
        }
        KeyCode::Char('s') => {
            app.navigate_to(Screen::Streams);
            false
        }
        KeyCode::F(5) => {
            app.action_refresh().await;
            true
        }
        KeyCode::Char('?') => {
            app.navigate_to(Screen::Help);
            false
        }
        _ => false,
    }
}

// ─── Streams ──────────────────────────────────────────────────────────────────

async fn handle_streams(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.go_home();
            false
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.stream_cursor > 0 {
                app.stream_cursor -= 1;
            }
            false
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = app.audio_state.streams.len().saturating_sub(1);
            if app.stream_cursor < max {
                app.stream_cursor += 1;
            }
            false
        }
        KeyCode::Enter => {
            if let Some(stream) = app.selected_stream() {
                // Find index of current sink to set initial cursor
                let current_sink_name = &stream.sink_name;
                let cursor = app.audio_state.devices.iter()
                    .position(|d| &d.name == current_sink_name)
                    .unwrap_or(0);
                    
                app.screen = Screen::SelectDevice { stream_index: stream.index, cursor };
            }
            false
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            // Move all streams to default
            let sink_name = app.audio_state.default_sink_name.clone();
            if !sink_name.is_empty() {
                let action = ConfirmAction::MoveAllStreams { sink_name };
                app.screen = Screen::Confirm(action);
            } else {
                app.notify(Notification::warning("No default sink set. Go to Devices to set one."));
            }
            false
        }
        KeyCode::Char('d') => {
            app.navigate_to(Screen::Devices);
            false
        }
        KeyCode::F(5) => {
            app.action_refresh().await;
            true
        }
        KeyCode::Char('?') => {
            app.navigate_to(Screen::Help);
            false
        }
        _ => false,
    }
}

// ─── Combine wizard ───────────────────────────────────────────────────────────

async fn handle_combine(app: &mut App, key: KeyEvent) -> bool {
    match app.combine_wizard.step.clone() {
        CombineStepState::SelectDevices => handle_combine_select(app, key).await,
        CombineStepState::NameGroup => handle_combine_name(app, key),
        CombineStepState::Confirm => handle_combine_confirm(app, key).await,
    }
}

async fn handle_combine_select(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.go_home();
            false
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.device_cursor > 0 {
                app.device_cursor -= 1;
            }
            false
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = app.audio_state.real_devices().len().saturating_sub(1);
            if app.device_cursor < max {
                app.device_cursor += 1;
            }
            false
        }
        KeyCode::Char(' ') => {
            let idx = app.device_cursor;
            app.combine_toggle_device(idx);
            false
        }
        KeyCode::Enter => {
            if app.combine_can_proceed() {
                app.combine_wizard.step = CombineStepState::NameGroup;
                // Suggest a default name
                if app.combine_wizard.group_name.is_empty() {
                    app.combine_wizard.group_name = "my_combined".to_string();
                    app.combine_wizard.name_cursor = app.combine_wizard.group_name.len();
                }
            } else {
                app.notify(Notification::warning("Select at least 2 devices with Space, then press Enter."));
            }
            false
        }
        KeyCode::Char('?') => {
            app.navigate_to(Screen::Help);
            false
        }
        _ => false,
    }
}

fn handle_combine_name(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.combine_wizard.step = CombineStepState::SelectDevices;
            false
        }
        KeyCode::Enter => {
            if app.combine_name_is_valid() {
                app.combine_wizard.step = CombineStepState::Confirm;
            } else {
                app.notify(Notification::warning(
                    "Name can only contain letters, numbers, hyphens, and underscores.",
                ));
            }
            false
        }
        KeyCode::Backspace => {
            let cursor = app.combine_wizard.name_cursor;
            if cursor > 0 {
                app.combine_wizard.group_name.remove(cursor - 1);
                app.combine_wizard.name_cursor -= 1;
            }
            false
        }
        KeyCode::Left => {
            if app.combine_wizard.name_cursor > 0 {
                app.combine_wizard.name_cursor -= 1;
            }
            false
        }
        KeyCode::Right => {
            let max = app.combine_wizard.group_name.len();
            if app.combine_wizard.name_cursor < max {
                app.combine_wizard.name_cursor += 1;
            }
            false
        }
        KeyCode::Tab => {
            // Toggle "set as default"
            app.combine_wizard.set_as_default = !app.combine_wizard.set_as_default;
            false
        }
        KeyCode::Char(c) => {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                let cursor = app.combine_wizard.name_cursor;
                app.combine_wizard.group_name.insert(cursor, c);
                app.combine_wizard.name_cursor += 1;
            }
            false
        }
        _ => false,
    }
}

async fn handle_combine_confirm(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.combine_wizard.step = CombineStepState::NameGroup;
            false
        }
        KeyCode::Tab => {
            // Toggle move streams option
            app.combine_wizard.move_streams = !app.combine_wizard.move_streams;
            false
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            app.combine_wizard.set_as_default = !app.combine_wizard.set_as_default;
            false
        }
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.action_create_combined().await;
            true
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.go_home();
            false
        }
        _ => false,
    }
}

// ─── Confirm dialog ───────────────────────────────────────────────────────────

async fn handle_confirm(app: &mut App, key: KeyEvent, action: ConfirmAction) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            // Cancel: go back to previous screen
            match app.screen.clone() {
                Screen::Confirm(_) => {
                    app.screen = Screen::Devices;
                }
                _ => {}
            }
            false
        }
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            match action {
                ConfirmAction::RemoveCombined { group_index } => {
                    app.screen = Screen::Devices;
                    app.action_remove_combined(group_index).await;
                    true
                }
                ConfirmAction::MoveAllStreams { sink_name } => {
                    app.screen = Screen::Streams;
                    app.action_move_all_streams_to(sink_name).await;
                    true
                }
                ConfirmAction::SetDefault { sink_name, move_streams } => {
                    app.screen = Screen::Devices;
                    app.action_set_default(sink_name, move_streams).await;
                    true
                }
                ConfirmAction::ResumeDevice { device_index } => {
                    app.screen = Screen::Devices;
                    app.action_resume_device(device_index).await;
                    true
                }
            }
        }
        _ => false,
    }
}

// ─── Help ─────────────────────────────────────────────────────────────────────

fn handle_help(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.go_home();
        }
        _ => {}
    }
    false
}

// ─── Select Device ────────────────────────────────────────────────────────────

async fn handle_select_device(app: &mut App, key: KeyEvent, stream_index: u32, cursor: usize) -> bool {
    let devices_len = app.audio_state.devices.len();
    if devices_len == 0 {
        app.screen = Screen::Streams;
        return false;
    }

    let max = devices_len.saturating_sub(1);

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('c') => {
            app.screen = Screen::Streams;
            false
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let new_cursor = cursor.saturating_sub(1);
            app.screen = Screen::SelectDevice { stream_index, cursor: new_cursor };
            false
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let new_cursor = if cursor < max { cursor + 1 } else { cursor };
            app.screen = Screen::SelectDevice { stream_index, cursor: new_cursor };
            false
        }
        KeyCode::Enter => {
            if let Some(device) = app.audio_state.devices.get(cursor) {
                let sink_name = device.name.clone();
                app.action_move_stream(stream_index, sink_name).await;
            }
            app.screen = Screen::Streams;
            true
        }
        _ => false,
    }
}

