use serde::{Deserialize, Serialize};

/// The type/category of an audio device (sink)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    Physical,
    Bluetooth,
    Combined,
    Virtual,
    Unknown,
}

impl DeviceType {
    pub fn display_label(&self) -> &str {
        match self {
            DeviceType::Physical => "🔊 Physical",
            DeviceType::Bluetooth => "🎧 Bluetooth",
            DeviceType::Combined => "🔀 Combined",
            DeviceType::Virtual => "💻 Virtual",
            DeviceType::Unknown => "❓ Unknown",
        }
    }
}

/// Status of a device
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Running,
    Idle,
    Suspended,
    Error(String),
}

impl DeviceStatus {
    pub fn display_label(&self) -> String {
        match self {
            DeviceStatus::Running => "▶ Running".to_string(),
            DeviceStatus::Idle => "⏸ Idle".to_string(),
            DeviceStatus::Suspended => "💤 Suspended".to_string(),
            DeviceStatus::Error(e) => format!("⚠ Error: {}", e),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "running" => DeviceStatus::Running,
            "idle" => DeviceStatus::Idle,
            "suspended" => DeviceStatus::Suspended,
            _ => DeviceStatus::Idle,
        }
    }
}

/// An audio sink (output device)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Numeric sink index from pactl
    pub index: u32,
    /// Internal sink name (e.g. "alsa_output.pci-0000_00_1f.3.analog-stereo")
    pub name: String,
    /// Human-friendly description
    pub description: String,
    /// Derived device type
    pub device_type: DeviceType,
    /// Current operational status
    pub status: DeviceStatus,
    /// Is this the system default sink?
    pub is_default: bool,
    /// If part of a combined group, which group?
    pub combined_group_id: Option<u32>,
    /// Sample rate (e.g. "48000 Hz")
    pub sample_rate: Option<String>,
    /// Sample format (e.g. "s16le")
    pub sample_format: Option<String>,
    /// Number of channels
    pub channels: Option<u8>,
    /// Volume 0–100
    pub volume: Option<u8>,
    /// Whether muted
    pub muted: bool,
    /// PulseAudio/PipeWire module index (for combined sinks, used to unload)
    pub module_index: Option<u32>,
}

impl Device {
    /// Returns a friendly display name, preferring description over raw name
    pub fn display_name(&self) -> &str {
        if self.description.is_empty() {
            &self.name
        } else {
            &self.description
        }
    }

    /// Returns a short type badge string
    pub fn type_badge(&self) -> &str {
        match self.device_type {
            DeviceType::Physical => "PHY",
            DeviceType::Bluetooth => "BT",
            DeviceType::Combined => "CMB",
            DeviceType::Virtual => "VRT",
            DeviceType::Unknown => "UNK",
        }
    }

    /// Whether device appears to be a combined sink
    pub fn is_combined(&self) -> bool {
        self.device_type == DeviceType::Combined
    }

    /// Whether device is available for audio (not suspended)
    pub fn is_available(&self) -> bool {
        !matches!(self.status, DeviceStatus::Suspended | DeviceStatus::Error(_))
    }
}

/// An active audio playback stream (sink input)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stream {
    /// Sink input index
    pub index: u32,
    /// Application name
    pub application_name: String,
    /// Media name/title if available
    pub media_name: Option<String>,
    /// Current sink index this stream is routed to
    pub sink_index: u32,
    /// Current sink name
    pub sink_name: String,
    /// Volume 0–100
    pub volume: Option<u8>,
    /// Whether muted
    pub muted: bool,
}

impl Stream {
    pub fn display_name(&self) -> String {
        if let Some(ref m) = self.media_name {
            if !m.is_empty() && m != "audio stream" {
                return format!("{} — {}", self.application_name, m);
            }
        }
        self.application_name.clone()
    }
}

/// A combined output group (module-combine-sink)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedGroup {
    /// Module index used to unload
    pub module_index: u32,
    /// The combined sink name
    pub sink_name: String,
    /// Human-friendly group name/label
    pub label: String,
    /// Member sink names
    pub members: Vec<String>,
    /// Is this the current default?
    pub is_default: bool,
    /// Is the combined sink currently active/visible?
    pub is_active: bool,
}

impl CombinedGroup {
    pub fn display_name(&self) -> &str {
        if self.label.is_empty() {
            &self.sink_name
        } else {
            &self.label
        }
    }
}

/// Full snapshot of audio state
#[derive(Debug, Clone, Default)]
pub struct AudioState {
    pub devices: Vec<Device>,
    pub streams: Vec<Stream>,
    pub combined_groups: Vec<CombinedGroup>,
    pub default_sink_name: String,
    /// Timestamp of last successful refresh
    pub last_refresh: Option<std::time::Instant>,
    /// Last error message (if any)
    pub last_error: Option<String>,
}

impl AudioState {
    pub fn default_device(&self) -> Option<&Device> {
        self.devices.iter().find(|d| d.is_default)
    }

    pub fn device_by_name(&self, name: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.name == name)
    }

    pub fn device_by_index(&self, index: u32) -> Option<&Device> {
        self.devices.iter().find(|d| d.index == index)
    }

    pub fn streams_on_sink(&self, sink_index: u32) -> Vec<&Stream> {
        self.streams.iter().filter(|s| s.sink_index == sink_index).collect()
    }

    /// Physical + bluetooth devices only (not combined/virtual)
    pub fn real_devices(&self) -> Vec<&Device> {
        self.devices
            .iter()
            .filter(|d| matches!(d.device_type, DeviceType::Physical | DeviceType::Bluetooth))
            .collect()
    }

    /// Combined-type devices
    pub fn combined_devices(&self) -> Vec<&Device> {
        self.devices
            .iter()
            .filter(|d| d.device_type == DeviceType::Combined)
            .collect()
    }
}
