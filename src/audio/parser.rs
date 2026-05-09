use anyhow::{Context, Result};
use serde::Deserialize;
use crate::audio::models::{CombinedGroup, Device, DeviceStatus, DeviceType, Stream};
use crate::dlog;

// ─── Raw JSON shapes from pactl ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct PactlSink {
    index: u32,
    name: String,
    description: String,
    driver: Option<String>,
    state: String,
    #[serde(rename = "sample_specification")]
    sample_spec: Option<String>,
    volume: Option<PactlVolume>,
    mute: Option<bool>,
    properties: Option<PactlSinkProps>,
    #[serde(rename = "owner_module")]
    owner_module: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct PactlVolume {
    #[serde(rename = "front-left")]
    front_left: Option<PactlChannel>,
}

#[derive(Debug, Deserialize)]
struct PactlChannel {
    value_percent: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PactlSinkProps {
    #[serde(rename = "bluetooth.connected")]
    bluetooth_connected: Option<String>,
    #[serde(rename = "device.class")]
    device_class: Option<String>,
    #[serde(rename = "combine.sink")]
    combine_sink: Option<String>,
    #[serde(rename = "module-combine-sink.sink_names")]
    combine_sink_names: Option<String>,
    #[serde(rename = "node.nick")]
    node_nick: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PactlSinkInput {
    index: u32,
    #[serde(rename = "sink")]
    sink_index: u32,
    properties: Option<PactlSinkInputProps>,
    volume: Option<PactlVolume>,
    mute: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PactlSinkInputProps {
    #[serde(rename = "application.name")]
    application_name: Option<String>,
    #[serde(rename = "media.name")]
    media_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PactlModule {
    index: Option<u32>,
    name: String,
    argument: Option<String>,
    properties: Option<serde_json::Value>,
}

// ─── Parsers ─────────────────────────────────────────────────────────────────

/// Parse JSON from `pactl --format=json list sinks`
pub fn parse_sinks(json: &str, default_sink_name: &str) -> Result<Vec<Device>> {
    dlog!("PARSER", "parse_sinks: json_len={} default={:?}", json.len(), default_sink_name);
    // Log the first object's keys to help diagnose schema mismatches
    if let Ok(arr) = serde_json::from_str::<serde_json::Value>(json) {
        if let Some(first) = arr.as_array().and_then(|a| a.first()) {
            if let Some(obj) = first.as_object() {
                let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
                dlog!("PARSER", "parse_sinks: first sink JSON keys = {:?}", keys);
            }
        } else {
            dlog!("PARSER", "parse_sinks: JSON is not an array or is empty — raw: {}",
                &json[..json.len().min(200)]);
        }
    } else {
        dlog!("PARSER", "parse_sinks: pre-parse as Value FAILED — raw start: {}",
            &json[..json.len().min(200)]);
    }
    let raw: Vec<PactlSink> = serde_json::from_str(json)
        .context("Failed to parse sink list from pactl")?;

    let devices = raw
        .into_iter()
        .map(|s| {
            let device_type = detect_device_type(&s);
            let status = DeviceStatus::from_str(&s.state);
            let is_default = s.name == default_sink_name;

            let mut sample_rate = None;
            let mut sample_format = None;
            let mut channels = None;

            if let Some(spec) = &s.sample_spec {
                let mut parts = spec.split_whitespace();
                sample_format = parts.next().map(String::from);
                channels = parts.next()
                    .and_then(|p| p.trim_end_matches("ch").parse::<u8>().ok());
                sample_rate = parts.next()
                    .map(|p| p.replace("Hz", " Hz"));
            }

            let volume = parse_volume(&s.volume);
            let muted = s.mute.unwrap_or(false);

            let module_index = s.owner_module.as_ref().and_then(|v| {
                v.as_u64().map(|n| n as u32)
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            });

            Device {
                index: s.index,
                name: s.name,
                description: s.description,
                device_type,
                status,
                is_default,
                combined_group_id: None,
                sample_rate,
                sample_format,
                channels,
                volume,
                muted,
                module_index,
            }
        })
        .collect();

    Ok(devices)
}

/// Parse JSON from `pactl --format=json list sink-inputs`
pub fn parse_sink_inputs(json: &str, devices: &[Device]) -> Result<Vec<Stream>> {
    dlog!("PARSER", "parse_sink_inputs: json_len={}", json.len());
    let raw: Vec<PactlSinkInput> = serde_json::from_str(json)
        .context("Failed to parse sink-input list from pactl")?;

    let streams = raw
        .into_iter()
        .map(|si| {
            let application_name = si.properties.as_ref()
                .and_then(|p| p.application_name.clone())
                .unwrap_or_else(|| "Unknown App".to_string());
            let media_name = si.properties.as_ref()
                .and_then(|p| p.media_name.clone());

            let sink_name = devices.iter()
                .find(|d| d.index == si.sink_index)
                .map(|d| d.name.clone())
                .unwrap_or_default();

            let volume = parse_volume(&si.volume);
            let muted = si.mute.unwrap_or(false);

            Stream {
                index: si.index,
                application_name,
                media_name,
                sink_index: si.sink_index,
                sink_name,
                volume,
                muted,
            }
        })
        .collect();

    Ok(streams)
}

/// Parse JSON from `pactl --format=json list modules` to find combined sinks
pub fn parse_combined_groups(json: &str) -> Result<Vec<CombinedGroup>> {
    dlog!("PARSER", "parse_combined_groups: json_len={}", json.len());
    let raw: Vec<PactlModule> = serde_json::from_str(json)
        .context("Failed to parse module list from pactl")?;
    dlog!("PARSER", "parse_combined_groups: {} total modules", raw.len());
    let combine_count = raw.iter().filter(|m| m.name == "module-combine-sink").count();
    dlog!("PARSER", "parse_combined_groups: {} module-combine-sink entries", combine_count);

    let groups = raw
        .into_iter()
        .filter(|m| m.name == "module-combine-sink")
        .filter_map(|m| {
            let args = m.argument.clone().unwrap_or_default();
            let module_index = m.index.or_else(|| {
                m.properties.as_ref().and_then(|p| {
                    p.get("object.id").and_then(|id| {
                        id.as_str().and_then(|s| s.parse().ok())
                            .or_else(|| id.as_u64().map(|n| n as u32))
                    })
                })
            }).unwrap_or(0);

            // Parse: sink_name=combined_output slaves=sink1,sink2 ...
            let sink_name = extract_arg(&args, "sink_name")
                .unwrap_or_else(|| format!("combined_{}", module_index));
            let slaves_str = extract_arg(&args, "slaves")
                .or_else(|| extract_arg(&args, "sink_properties"));
            let members: Vec<String> = slaves_str
                .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
                .unwrap_or_default();

            // Label: try to derive friendly name from members
            let label = extract_arg(&args, "sink_properties")
                .unwrap_or_else(|| sink_name.clone());

            Some(CombinedGroup {
                module_index,
                sink_name,
                label,
                members,
                is_default: false,
                is_active: true,
            })
        })
        .collect();

    Ok(groups)
}

/// Parse the default sink name from `pactl info` output
pub fn parse_default_sink(info_output: &str) -> String {
    dlog!("PARSER", "parse_default_sink from {} chars", info_output.len());
    for line in info_output.lines() {
        let line = line.trim();
        if line.starts_with("Default Sink:") {
            let sink = line
                .trim_start_matches("Default Sink:")
                .trim()
                .to_string();
            dlog!("PARSER", "parse_default_sink found: {:?}", sink);
            return sink;
        }
    }
    dlog!("PARSER", "parse_default_sink: NOT FOUND in pactl info output");
    String::new()
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn detect_device_type(s: &PactlSink) -> DeviceType {
    // Check name/driver for combined
    if s.name.contains("combined") || s.name.starts_with("combine") {
        return DeviceType::Combined;
    }
    if let Some(driver) = &s.driver {
        if driver.contains("combine") || driver.contains("module-combine") {
            return DeviceType::Combined;
        }
    }
    if let Some(props) = &s.properties {
        if props.combine_sink.is_some() || props.combine_sink_names.is_some() {
            return DeviceType::Combined;
        }
        if let Some(class) = &props.device_class {
            if class == "abstract" || class == "virtual" {
                return DeviceType::Virtual;
            }
        }
        if props.bluetooth_connected.as_deref() == Some("1")
            || props.bluetooth_connected.as_deref() == Some("true")
        {
            return DeviceType::Bluetooth;
        }
    }
    // Bluetooth heuristic from name
    if s.name.contains("bluez") || s.description.to_lowercase().contains("bluetooth") {
        return DeviceType::Bluetooth;
    }
    // Virtual/null sinks
    if s.name.contains("null") || s.name.contains("virtual") {
        return DeviceType::Virtual;
    }
    DeviceType::Physical
}

fn parse_volume(vol: &Option<PactlVolume>) -> Option<u8> {
    vol.as_ref()
        .and_then(|v| v.front_left.as_ref())
        .and_then(|ch| ch.value_percent.as_ref())
        .and_then(|vp| {
            vp.trim_end_matches('%').trim().parse::<f32>().ok()
        })
        .map(|f| f.round() as u8)
}

fn extract_arg<'a>(args: &'a str, key: &str) -> Option<String> {
    for part in args.split_whitespace() {
        if let Some(rest) = part.strip_prefix(&format!("{}=", key)) {
            return Some(rest.trim_matches('"').to_string());
        }
    }
    None
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_sink() {
        let info = "Server Name: PulseAudio (on PipeWire 1.0.7)\nDefault Sink: alsa_output.pci-0000_00_1f.3.analog-stereo\nDefault Source: alsa_input.foo\n";
        assert_eq!(
            parse_default_sink(info),
            "alsa_output.pci-0000_00_1f.3.analog-stereo"
        );
    }

    #[test]
    fn test_extract_arg() {
        let args = r#"sink_name=combined_output slaves=sink1,sink2 rate=48000"#;
        assert_eq!(extract_arg(args, "sink_name"), Some("combined_output".to_string()));
        assert_eq!(extract_arg(args, "slaves"), Some("sink1,sink2".to_string()));
        assert_eq!(extract_arg(args, "rate"), Some("48000".to_string()));
        assert_eq!(extract_arg(args, "missing"), None);
    }

    #[test]
    fn test_parse_volume_percent() {
        let vol = Some(PactlVolume {
            front_left: Some(PactlChannel {
                value_percent: Some("65%".to_string()),
            }),
        });
        assert_eq!(parse_volume(&vol), Some(65));
    }
}
