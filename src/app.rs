use std::collections::HashSet;
use anyhow::Result;

use crate::audio::backend::AudioBackend;
use crate::audio::models::{AudioState, CombinedGroup, Device};
use crate::{dlog, dlog_section};

// ─── Screen enum ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Home,
    Devices,
    Streams,
    Combine,
    Help,
    Confirm(ConfirmAction),
}

/// Actions that require confirmation before executing
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    RemoveCombined { group_index: usize },
    MoveAllStreams { sink_name: String },
    SetDefault { sink_name: String, move_streams: bool },
    ResumeDevice { device_index: usize },
}

// ─── Wizard state for combine flow ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum CombineStep {
    SelectDevices,
    NameGroup,
    Confirm,
}

#[derive(Debug, Default, Clone)]
pub struct CombineWizard {
    pub step: CombineStepState,
    pub selected_device_indices: HashSet<usize>,
    pub group_name: String,
    pub name_cursor: usize,
    pub set_as_default: bool,
    pub move_streams: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CombineStepState {
    SelectDevices,
    NameGroup,
    Confirm,
}

impl Default for CombineStepState {
    fn default() -> Self {
        CombineStepState::SelectDevices
    }
}

// ─── Notification ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    pub created_at: std::time::Instant,
}

impl Notification {
    pub fn info(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), level: NotificationLevel::Info, created_at: std::time::Instant::now() }
    }
    pub fn success(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), level: NotificationLevel::Success, created_at: std::time::Instant::now() }
    }
    pub fn warning(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), level: NotificationLevel::Warning, created_at: std::time::Instant::now() }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), level: NotificationLevel::Error, created_at: std::time::Instant::now() }
    }
}

// ─── App ──────────────────────────────────────────────────────────────────────

pub struct App {
    /// Current screen
    pub screen: Screen,
    /// Audio state snapshot
    pub audio_state: AudioState,
    /// Backend for commands
    pub backend: AudioBackend,
    /// Loading flag (during async operations)
    pub loading: bool,
    /// Current notification to show
    pub notification: Option<Notification>,
    /// Cursor position for device list
    pub device_cursor: usize,
    /// Cursor position for stream list
    pub stream_cursor: usize,
    /// Cursor position for combined groups list
    pub combined_cursor: usize,
    /// Current search/filter string
    pub filter_text: String,
    /// Whether filter input is active
    pub filter_active: bool,
    /// Combine wizard state
    pub combine_wizard: CombineWizard,
    /// Whether app should quit
    pub should_quit: bool,
    /// Backend server name (e.g. "PulseAudio (on PipeWire 1.2.0)")
    pub server_name: String,
    /// Error from last state refresh
    pub refresh_error: Option<String>,
    /// Whether to scroll device list
    pub device_scroll_offset: usize,
    /// Whether to scroll stream list
    pub stream_scroll_offset: usize,
}

impl App {
    pub fn new(backend: AudioBackend) -> Self {
        Self {
            screen: Screen::Home,
            audio_state: AudioState::default(),
            backend,
            loading: false,
            notification: None,
            device_cursor: 0,
            stream_cursor: 0,
            combined_cursor: 0,
            filter_text: String::new(),
            filter_active: false,
            combine_wizard: CombineWizard::default(),
            should_quit: false,
            server_name: String::new(),
            refresh_error: None,
            device_scroll_offset: 0,
            stream_scroll_offset: 0,
        }
    }

    /// Navigate to a screen, resetting relevant state
    pub fn navigate_to(&mut self, screen: Screen) {
        dlog!("NAV", "navigate_to {:?} (from {:?})", screen, self.screen);
        match &screen {
            Screen::Combine => {
                self.combine_wizard = CombineWizard::default();
            }
            Screen::Devices => {
                self.filter_text.clear();
                self.filter_active = false;
                self.device_scroll_offset = 0;
            }
            Screen::Streams => {
                self.stream_scroll_offset = 0;
            }
            _ => {}
        }
        self.screen = screen;
    }

    /// Go back to the home screen
    pub fn go_home(&mut self) {
        dlog!("NAV", "go_home (from {:?})", self.screen);
        self.screen = Screen::Home;
        self.filter_text.clear();
        self.filter_active = false;
    }

    /// Show a notification (auto-expires via render)
    pub fn notify(&mut self, n: Notification) {
        dlog!("NOTIFY", "[{:?}] {}", n.level, n.message);
        self.notification = Some(n);
    }

    /// Clear expired notifications (older than 4 seconds)
    pub fn tick_notifications(&mut self) {
        if let Some(ref n) = self.notification {
            if n.created_at.elapsed().as_secs() > 4 {
                self.notification = None;
            }
        }
    }

    /// Get filtered device list
    pub fn filtered_devices(&self) -> Vec<(usize, &Device)> {
        let query = self.filter_text.to_lowercase();
        self.audio_state
            .devices
            .iter()
            .enumerate()
            .filter(|(_, d)| {
                if query.is_empty() {
                    return true;
                }
                d.display_name().to_lowercase().contains(&query)
                    || d.name.to_lowercase().contains(&query)
                    || d.device_type.display_label().to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Clamp device cursor to valid range
    pub fn clamp_device_cursor(&mut self) {
        let len = self.filtered_devices().len();
        if len == 0 {
            self.device_cursor = 0;
        } else if self.device_cursor >= len {
            self.device_cursor = len - 1;
        }
    }

    /// Clamp stream cursor
    pub fn clamp_stream_cursor(&mut self) {
        let len = self.audio_state.streams.len();
        if len == 0 {
            self.stream_cursor = 0;
        } else if self.stream_cursor >= len {
            self.stream_cursor = len - 1;
        }
    }

    /// Clamp combined cursor
    pub fn clamp_combined_cursor(&mut self) {
        let len = self.audio_state.combined_groups.len();
        if len == 0 {
            self.combined_cursor = 0;
        } else if self.combined_cursor >= len {
            self.combined_cursor = len - 1;
        }
    }

    /// Get the currently selected device (from filtered list)
    pub fn selected_device(&self) -> Option<&Device> {
        self.filtered_devices()
            .get(self.device_cursor)
            .map(|(_, d)| *d)
    }

    /// Get the currently selected stream
    pub fn selected_stream(&self) -> Option<&crate::audio::models::Stream> {
        self.audio_state.streams.get(self.stream_cursor)
    }

    /// Get the currently selected combined group
    pub fn selected_combined_group(&self) -> Option<&CombinedGroup> {
        self.audio_state.combined_groups.get(self.combined_cursor)
    }

    // ── Combine wizard helpers ────────────────────────────────────────────

    pub fn combine_toggle_device(&mut self, idx: usize) {
        if self.combine_wizard.selected_device_indices.contains(&idx) {
            self.combine_wizard.selected_device_indices.remove(&idx);
        } else {
            self.combine_wizard.selected_device_indices.insert(idx);
        }
    }

    pub fn combine_selected_devices(&self) -> Vec<&Device> {
        let real_devices = self.audio_state.real_devices();
        self.combine_wizard
            .selected_device_indices
            .iter()
            .filter_map(|&i| real_devices.get(i).copied())
            .collect()
    }

    pub fn combine_can_proceed(&self) -> bool {
        self.combine_wizard.selected_device_indices.len() >= 2
    }

    pub fn combine_name_is_valid(&self) -> bool {
        let name = self.combine_wizard.group_name.trim();
        !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Derive a safe sink name from the wizard's group name
    pub fn combine_sink_name(&self) -> String {
        let label = self.combine_wizard.group_name.trim().to_lowercase();
        let safe: String = label
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
            .collect();
        format!("combined_{}", safe)
    }

    // ── Async actions (called from event handler, state refresh follows) ──

    pub async fn action_refresh(&mut self) {
        dlog_section!("action_refresh");
        dlog!("APP", "action_refresh called");
        self.loading = true;
        match self.backend.fetch_state().await {
            Ok(state) => {
                dlog!("APP", "action_refresh OK: {} devices, {} streams, {} combined, default={:?}",
                    state.devices.len(), state.streams.len(),
                    state.combined_groups.len(), state.default_sink_name);
                self.audio_state = state;
                self.refresh_error = None;
                self.clamp_device_cursor();
                self.clamp_stream_cursor();
                self.clamp_combined_cursor();
            }
            Err(e) => {
                dlog!("APP", "action_refresh ERROR: {}", e);
                self.refresh_error = Some(e.to_string());
                self.audio_state.last_error = Some(e.to_string());
                self.notify(Notification::error(format!("Refresh failed: {}", e)));
            }
        }
        self.loading = false;
        dlog!("APP", "action_refresh done, loading=false");
    }

    pub async fn action_set_default(&mut self, sink_name: String, move_streams: bool) {
        self.loading = true;
        match self.backend.set_default_sink(&sink_name).await {
            Ok(()) => {
                if move_streams {
                    // Quick state clone to pass to move_all_streams_to
                    let state_snapshot = self.audio_state.clone();
                    let errors = self.backend.move_all_streams_to(&sink_name, &state_snapshot).await.unwrap_or_default();
                    if errors.is_empty() {
                        self.notify(Notification::success(format!("'{}' is now your default output and all streams moved.", sink_name)));
                    } else {
                        self.notify(Notification::warning(format!(
                            "Default set, but {} stream(s) could not be moved.",
                            errors.len()
                        )));
                    }
                } else {
                    self.notify(Notification::success(format!("'{}' is now the default output.", sink_name)));
                }
            }
            Err(e) => {
                self.notify(Notification::error(format!("Could not set default: {}", e)));
            }
        }
        self.loading = false;
        self.action_refresh().await;
    }

    pub async fn action_create_combined(&mut self) {
        let sink_name = self.combine_sink_name();
        let members: Vec<String> = self.combine_selected_devices()
            .iter()
            .map(|d| d.name.clone())
            .collect();
        let set_default = self.combine_wizard.set_as_default;
        let move_streams = self.combine_wizard.move_streams;

        // Check for duplicate name
        if self.audio_state.devices.iter().any(|d| d.name == sink_name) {
            self.notify(Notification::warning(
                "A combined output with that name already exists. Please choose a different name.",
            ));
            return;
        }

        self.loading = true;
        match self.backend.create_combined_sink(&sink_name, &members).await {
            Ok(_module_idx) => {
                // Small delay to let PipeWire register the new sink
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                if set_default {
                    let _ = self.backend.set_default_sink(&sink_name).await;
                    if move_streams {
                        let state_snapshot = self.audio_state.clone();
                        let _ = self.backend.move_all_streams_to(&sink_name, &state_snapshot).await;
                    }
                }
                self.notify(Notification::success(format!(
                    "Combined output '{}' created with {} devices.",
                    self.combine_wizard.group_name.trim(),
                    members.len()
                )));
                self.screen = Screen::Devices;
            }
            Err(e) => {
                self.notify(Notification::error(format!("Failed to create combined output: {}", e)));
            }
        }
        self.loading = false;
        self.action_refresh().await;
    }

    pub async fn action_remove_combined(&mut self, group_idx: usize) {
        let group = match self.audio_state.combined_groups.get(group_idx) {
            Some(g) => g.clone(),
            None => return,
        };

        self.loading = true;
        match self.backend.remove_combined_sink(&group).await {
            Ok(()) => {
                self.notify(Notification::success(format!(
                    "Combined output '{}' removed.",
                    group.display_name()
                )));
            }
            Err(e) => {
                self.notify(Notification::error(format!("Could not remove: {}", e)));
            }
        }
        self.loading = false;
        self.action_refresh().await;
    }

    pub async fn action_move_stream(&mut self, stream_idx: u32, sink_name: String) {
        self.loading = true;
        match self.backend.move_stream_to(stream_idx, &sink_name).await {
            Ok(()) => {
                self.notify(Notification::success(format!("Stream moved to '{}'.", sink_name)));
            }
            Err(e) => {
                self.notify(Notification::error(format!("Could not move stream: {}", e)));
            }
        }
        self.loading = false;
        self.action_refresh().await;
    }

    pub async fn action_move_all_streams_to(&mut self, sink_name: String) {
        let state_snapshot = self.audio_state.clone();
        self.loading = true;
        let errors = self.backend.move_all_streams_to(&sink_name, &state_snapshot).await.unwrap_or_default();
        if errors.is_empty() {
            self.notify(Notification::success(format!("All streams moved to '{}'.", sink_name)));
        } else {
            self.notify(Notification::warning(format!(
                "Moved streams, but {} failed: {}",
                errors.len(),
                errors.first().unwrap_or(&String::new())
            )));
        }
        self.loading = false;
        self.action_refresh().await;
    }

    pub async fn action_resume_device(&mut self, device_idx: usize) {
        let device = match self.audio_state.devices.get(device_idx) {
            Some(d) => d.clone(),
            None => return,
        };
        self.loading = true;
        match self.backend.resume_device(&device).await {
            Ok(()) => {
                self.notify(Notification::success(format!("'{}' resumed.", device.display_name())));
            }
            Err(e) => {
                self.notify(Notification::error(format!("Could not resume device: {}", e)));
            }
        }
        self.loading = false;
        self.action_refresh().await;
    }
}
