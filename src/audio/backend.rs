use anyhow::{anyhow, Context, Result};
use tokio::process::Command;

use crate::audio::models::{AudioState, CombinedGroup, Device, DeviceType};
use crate::audio::parser;
use crate::{dlog, dlog_section};

// ─── Backend ─────────────────────────────────────────────────────────────────

pub struct AudioBackend;

impl AudioBackend {
    pub fn new() -> Self {
        Self
    }

    // ── Discovery ──────────────────────────────────────────────────────────

    /// Fetch full audio state snapshot
    pub async fn fetch_state(&self) -> Result<AudioState> {
        dlog_section!("fetch_state");
        dlog!("BACKEND", "Starting concurrent pactl queries (sinks, sink-inputs, modules, info)");

        // Run all queries concurrently
        let result = tokio::try_join!(
            run_pactl(&["--format=json", "list", "sinks"]),
            run_pactl(&["--format=json", "list", "sink-inputs"]),
            run_pactl(&["--format=json", "list", "modules"]),
            run_pactl(&["info"]),
        );

        let (sinks_json, inputs_json, modules_json, info_out) = match result {
            Ok(v) => {
                dlog!("BACKEND", "All pactl queries succeeded");
                v
            }
            Err(e) => {
                dlog!("BACKEND", "FATAL: concurrent pactl query failed: {}", e);
                return Err(e);
            }
        };

        // Log raw output lengths and first 400 chars for inspection
        dlog!("PARSE", "sinks JSON  : {} bytes", sinks_json.len());
        dlog!("PARSE", "sinks JSON preview: {}", truncate(&sinks_json, 400));
        dlog!("PARSE", "sink-inputs : {} bytes", inputs_json.len());
        dlog!("PARSE", "sink-inputs preview: {}", truncate(&inputs_json, 200));
        dlog!("PARSE", "modules JSON: {} bytes", modules_json.len());
        dlog!("PARSE", "info output : {}", truncate(&info_out, 300));

        let default_sink = parser::parse_default_sink(&info_out);
        dlog!("PARSE", "default_sink name = {:?}", default_sink);

        let mut devices = match parser::parse_sinks(&sinks_json, &default_sink) {
            Ok(d) => {
                dlog!("PARSE", "parse_sinks OK: {} device(s)", d.len());
                for dev in &d {
                    dlog!("DEVICE", "  idx={} name={:?} type={:?} status={:?} is_default={}",
                        dev.index, dev.name, dev.device_type, dev.status, dev.is_default);
                }
                d
            }
            Err(e) => {
                dlog!("PARSE", "parse_sinks ERROR: {}", e);
                return Err(e.context("Parsing sinks"));
            }
        };

        let combined_groups = match parser::parse_combined_groups(&modules_json) {
            Ok(g) => {
                dlog!("PARSE", "parse_combined_groups OK: {} group(s)", g.len());
                for grp in &g {
                    dlog!("COMBINED", "  module={} sink_name={:?} members={:?}",
                        grp.module_index, grp.sink_name, grp.members);
                }
                g
            }
            Err(e) => {
                dlog!("PARSE", "parse_combined_groups ERROR: {}", e);
                return Err(e.context("Parsing combined groups"));
            }
        };

        // Mark devices that are members of combined groups
        for group in &combined_groups {
            for member_name in &group.members {
                if let Some(d) = devices.iter_mut().find(|d| &d.name == member_name) {
                    d.combined_group_id = Some(group.module_index);
                    dlog!("COMBINED", "  Marked device {:?} as member of group {}", d.name, group.module_index);
                }
            }
            
            // Also explicitly mark the sink itself as a Combined type device.
            // This is necessary because PipeWire might not add "combined" to the sink's description
            // or provide PulseAudio-specific properties if the user chose a custom name.
            if let Some(d) = devices.iter_mut().find(|d| d.name == group.sink_name) {
                d.device_type = DeviceType::Combined;
                dlog!("COMBINED", "  Forced device {:?} type to Combined", d.name);
            }
        }

        // Detect stale combined devices
        let active_combined_names: std::collections::HashSet<&str> = combined_groups
            .iter()
            .map(|g| g.sink_name.as_str())
            .collect();

        for dev in devices.iter_mut() {
            if dev.device_type == DeviceType::Combined
                && !active_combined_names.contains(dev.name.as_str())
            {
                dlog!("DEVICE", "  Stale combined sink detected: {:?}", dev.name);
                dev.status = crate::audio::models::DeviceStatus::Error(
                    "Stale combined output (module unloaded)".to_string(),
                );
            }
        }

        let mut combined_groups = combined_groups;
        for g in combined_groups.iter_mut() {
            g.is_default = g.sink_name == default_sink;
            // In PipeWire, pactl list modules omits the index, causing it to default to 0.
            // We can recover it from the owner_module field of the corresponding Sink.
            if g.module_index == 0 {
                if let Some(dev) = devices.iter().find(|d| d.name == g.sink_name) {
                    if let Some(mi) = dev.module_index {
                        g.module_index = mi;
                        dlog!("COMBINED", "Recovered module_index {} for {}", mi, g.sink_name);
                    }
                }
            }
        }

        let streams = match parser::parse_sink_inputs(&inputs_json, &devices) {
            Ok(s) => {
                dlog!("PARSE", "parse_sink_inputs OK: {} stream(s)", s.len());
                for st in &s {
                    dlog!("STREAM", "  idx={} app={:?} sink_idx={}", st.index, st.application_name, st.sink_index);
                }
                s
            }
            Err(e) => {
                dlog!("PARSE", "parse_sink_inputs ERROR: {}", e);
                return Err(e.context("Parsing sink inputs"));
            }
        };

        dlog!("BACKEND", "fetch_state complete: {} devices, {} streams, {} combined groups",
            devices.len(), streams.len(), combined_groups.len());
        dlog_section!("fetch_state END");

        Ok(AudioState {
            devices,
            streams,
            combined_groups,
            default_sink_name: default_sink,
            last_refresh: Some(std::time::Instant::now()),
            last_error: None,
        })
    }

    // ── Default routing ────────────────────────────────────────────────────

    /// Set the system default sink
    pub async fn set_default_sink(&self, sink_name: &str) -> Result<()> {
        dlog!("ACTION", "set_default_sink({:?})", sink_name);
        let r = run_pactl(&["set-default-sink", sink_name])
            .await
            .map(|_| ())
            .with_context(|| format!("Failed to set default sink to '{}'", sink_name));
        if let Err(ref e) = r { dlog!("ACTION", "set_default_sink ERROR: {}", e); }
        else { dlog!("ACTION", "set_default_sink OK"); }
        r
    }

    /// Move all current streams to a given sink
    pub async fn move_all_streams_to(&self, sink_name: &str, state: &AudioState) -> Result<Vec<String>> {
        dlog!("ACTION", "move_all_streams_to({:?}) — {} stream(s)", sink_name, state.streams.len());
        let mut errors = Vec::new();
        for stream in &state.streams {
            if let Err(e) = self.move_stream_to(stream.index, sink_name).await {
                let msg = format!("Stream {} ({}): {}", stream.index, stream.application_name, e);
                dlog!("ACTION", "  move_stream ERROR: {}", msg);
                errors.push(msg);
            } else {
                dlog!("ACTION", "  moved stream {} OK", stream.index);
            }
        }
        Ok(errors)
    }

    /// Move a single stream to a given sink
    pub async fn move_stream_to(&self, stream_index: u32, sink_name: &str) -> Result<()> {
        dlog!("ACTION", "move_stream_to(stream={}, sink={:?})", stream_index, sink_name);
        run_pactl(&["move-sink-input", &stream_index.to_string(), sink_name])
            .await
            .map(|_| ())
            .with_context(|| format!("Failed to move stream {} to '{}'", stream_index, sink_name))
    }

    // ── Combined sinks ─────────────────────────────────────────────────────

    /// Create a combined sink from a list of sink names
    pub async fn create_combined_sink(
        &self,
        sink_name: &str,
        member_names: &[String],
    ) -> Result<u32> {
        dlog!("ACTION", "create_combined_sink name={:?} members={:?}", sink_name, member_names);
        if member_names.is_empty() {
            dlog!("ACTION", "ERROR: no members provided");
            return Err(anyhow!("Cannot create combined sink with no members"));
        }

        let slaves = member_names.join(",");
        let args_str = format!(
            "sink_name={} slaves={} sink_properties=device.description=\"{}\"",
            sink_name, slaves, sink_name
        );
        dlog!("PACTL", "load-module module-combine-sink {}", args_str);

        let output = run_pactl(&["load-module", "module-combine-sink", &args_str]).await?;
        dlog!("PACTL", "load-module stdout: {:?}", output.trim());

        let module_index = output
            .trim()
            .parse::<u32>()
            .with_context(|| format!("Unexpected output from load-module: '{}'", output.trim()))?;
        dlog!("ACTION", "create_combined_sink OK, module_index={}", module_index);
        Ok(module_index)
    }

    /// Remove a combined sink by unloading its module
    pub async fn remove_combined_sink(&self, group: &CombinedGroup) -> Result<()> {
        dlog!("ACTION", "remove_combined_sink module={} name={:?}", group.module_index, group.sink_name);
        let r = run_pactl(&["unload-module", &group.module_index.to_string()])
            .await
            .map(|_| ())
            .with_context(|| format!("Failed to unload module {}", group.module_index));
        if let Err(ref e) = r { dlog!("ACTION", "remove_combined_sink ERROR: {}", e); }
        else { dlog!("ACTION", "remove_combined_sink OK"); }
        r
    }

    /// Check if a sink name already exists
    pub async fn sink_name_exists(&self, name: &str, state: &AudioState) -> bool {
        state.devices.iter().any(|d| d.name == name)
    }

    // ── Device operations ──────────────────────────────────────────────────

    /// Attempt to resume/unsuspend a suspended device
    pub async fn resume_device(&self, device: &Device) -> Result<()> {
        dlog!("ACTION", "resume_device({:?})", device.name);
        run_pactl(&["suspend-sink", &device.name, "0"])
            .await
            .map(|_| ())
            .with_context(|| format!("Failed to resume device '{}'", device.name))
    }

    /// Set volume for a sink (0–100)
    pub async fn set_volume(&self, sink_name: &str, percent: u8) -> Result<()> {
        let vol_str = format!("{}%", percent.min(100));
        dlog!("ACTION", "set_volume({:?}, {})", sink_name, vol_str);
        run_pactl(&["set-sink-volume", sink_name, &vol_str])
            .await
            .map(|_| ())
    }

    /// Toggle mute for a sink
    pub async fn toggle_mute(&self, sink_name: &str) -> Result<()> {
        dlog!("ACTION", "toggle_mute({:?})", sink_name);
        run_pactl(&["set-sink-mute", sink_name, "toggle"])
            .await
            .map(|_| ())
    }

    // ── Health check ───────────────────────────────────────────────────────

    /// Verify pactl is available and PipeWire/PulseAudio is running
    pub async fn check_health(&self) -> Result<String> {
        dlog!("BACKEND", "check_health: running pactl info");
        let info = run_pactl(&["info"])
            .await
            .context("pactl not found or audio server not running.\n\nTry: systemctl --user start pipewire pipewire-pulse")?;

        dlog!("BACKEND", "check_health pactl info output:\n{}", info);

        for line in info.lines() {
            if line.trim().starts_with("Server Name:") {
                let name = line.trim_start_matches("Server Name:").trim().to_string();
                dlog!("BACKEND", "check_health server_name={:?}", name);
                return Ok(name);
            }
        }
        Ok("PipeWire/PulseAudio (details unavailable)".to_string())
    }
}

// ─── Helper ───────────────────────────────────────────────────────────────────

/// Run a pactl command, log invocation + result, return stdout or Err
async fn run_pactl(args: &[&str]) -> Result<String> {
    let cmd_str = format!("pactl {}", args.join(" "));
    dlog!("PACTL", ">> {}", cmd_str);

    let output = Command::new("pactl")
        .args(args)
        .output()
        .await
        .context("Failed to spawn pactl — is it installed? (pacman -S libpulse)")?;

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    dlog!("PACTL", "<< exit={} stdout_bytes={} stderr_bytes={}",
        exit_code, stdout.len(), stderr.len());

    if !stderr.trim().is_empty() {
        dlog!("PACTL", "   stderr: {}", stderr.trim());
    }

    if output.status.success() {
        Ok(stdout)
    } else {
        dlog!("PACTL", "   FAILED: {}", stderr.trim());
        Err(anyhow!(
            "pactl {} failed (exit {}): {}",
            args.join(" "),
            exit_code,
            stderr.trim()
        ))
    }
}

// ─── Utility ──────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.chars().map(|c| if c == '\n' { '↵' } else { c }).collect()
    } else {
        let mut r: String = s.chars().take(max).map(|c| if c == '\n' { '↵' } else { c }).collect();
        r.push_str(&format!("…[+{}]", s.len() - max));
        r
    }
}
