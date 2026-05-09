# 🔊 Multi Audio Sink — Human-friendly Audio Management TUI

A polished terminal user interface for managing audio outputs on Linux systems running
**PipeWire** (or PulseAudio-compatible setups). Designed for Arch Linux. No commands to memorize.

![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT-blue)
![Platform](https://img.shields.io/badge/platform-linux-green)

---

## What it does

| You want to…                          | Multi Audio Sink does it with…       |
|---------------------------------------|------------------------------|
| See all audio outputs                 | Home dashboard + Devices tab |
| Play sound on two devices at once     | Combine wizard (`C`)         |
| Set a device as default               | `d` on any device            |
| Move apps to a different output       | `m` or Streams tab           |
| Remove a combined output              | `R` on the combined device   |
| Wake up a suspended Bluetooth device  | `r` on the device            |
| Know what's going on right now        | Home screen, always visible  |

---

## Screenshots (key screens)

```
╭─ 🔊 Multi Audio Sink ─────────────────────╮╭── ★ Default: Speakers ────────────╮╭─ 2 sinks  1 stream ──╮
╰────────────────────────────────────╯╰───────────────────────────────────╯╰──────────────────────╯

╭─── 📡 Output Devices ─────────────────────╮╭── 🔀 Combined Outputs ────╮╭─ 🎵 Streams ───────────╮
│ ▸ ▶ [PHY] Speakers ★                      ││  No combined outputs.     ││ 🔊 Firefox — YouTube   │
│   💤 [BT]  AirPods Pro                    ││  Press [C] on devices to  ││      65%               │
│   ⏸ [PHY] HDMI Output                    ││  create one.              │╰────────────────────────╯
╰────────────────────────────────────────────╯╰───────────────────────────╯

  [D] Devices  [S] Streams  [C] Combine  [?] Help  [F5] Refresh  [Q] Quit
```

---

## Installation (Arch Linux)

### Prerequisites

```bash
# PipeWire with PulseAudio compatibility layer
sudo pacman -S pipewire pipewire-pulse

# pactl (comes with libpulse)
sudo pacman -S libpulse

# Verify PipeWire is running
systemctl --user status pipewire pipewire-pulse
```

### Build from source

```bash
# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone <repo-url> multi-audio-sink
cd multi-audio-sink
cargo build --release

# Run
./target/release/mas
```

### Install to PATH

```bash
cargo install --path .
# or
sudo install -m755 target/release/mas /usr/local/bin/mas
```

### AUR (if packaged)

```bash
yay -S multi-audio-sink  # once available
```

---

## Usage

```bash
mas                  # Launch normally
mas --debug          # Enable step-by-step debug logging to mas_debug.log
mas --tick-rate 100  # Faster animation (ms)
mas --no-health-check  # Skip pactl connectivity check
mas --help
```

---

## Keyboard Reference

### Global

| Key          | Action                  |
|--------------|-------------------------|
| `Q`          | Quit                    |
| `Ctrl+C`     | Force quit              |
| `?` / `F1`   | Help screen             |
| `F5`         | Refresh audio state     |
| `D`          | Go to Devices           |
| `S`          | Go to Streams           |
| `C`          | Create combined output  |
| `Esc`        | Go back / cancel        |

### Devices screen

| Key          | Action                                      |
|--------------|---------------------------------------------|
| `↑↓` / `jk` | Navigate devices                            |
| `d`          | Set as default output (streams stay)        |
| `D`          | Set as default AND move all streams here    |
| `m`          | Move all current streams to this device     |
| `r`          | Resume (unsuspend) this device              |
| `R`          | Remove this combined output (with confirm)  |
| `/` or `F`   | Filter / search devices                     |
| `Esc`        | Clear filter / go back                      |

### Streams screen

| Key       | Action                                       |
|-----------|----------------------------------------------|
| `↑↓` / `jk` | Navigate streams                          |
| `Enter`   | Move selected stream to current default sink |
| `M`       | Move ALL streams to the default output       |

### Combine wizard

| Key       | Action                      |
|-----------|-----------------------------|
| `Space`   | Select / deselect a device  |
| `Enter`   | Proceed to next step        |
| `Tab`     | Toggle options              |
| `Y`       | Confirm creation            |
| `N / Esc` | Cancel / go back            |

---

## Common Problems

### PipeWire running but no sinks visible

```
Symptom: Home screen shows "0 sinks"
```

1. Check PipeWire is fully started:
   ```bash
   systemctl --user start pipewire pipewire-pulse
   pactl info
   ```
2. Check `pactl list sinks` returns output.
3. If you see `Connection refused`, PulseAudio compat layer isn't running:
   ```bash
   systemctl --user enable --now pipewire-pulse.socket
   ```
4. Press `F5` to refresh after starting services.

---

### Bluetooth device shows as Suspended

```
Symptom: Device visible but shows 💤, can't use it
```

1. Make sure the Bluetooth device is powered on and connected:
   ```bash
   bluetoothctl connect <MAC>
   ```
2. Select the device in Multi Audio Sink and press `r` to resume it.
3. If still suspended after connecting, try:
   ```bash
   pactl set-sink-port bluez_sink.<...> headset-output
   ```

---

### Combined output created but no sound

```
Symptom: Combined sink exists but audio doesn't play on all devices
```

1. After creation, press `D` to set the combined output as default AND move streams.
2. Some apps cache the old sink — restart them or use `M` on the Streams screen.
3. Verify with `pactl list sink-inputs` that streams point to the combined sink.
4. If one member device is suspended, it will silently drop audio. Resume it with `r`.

---

### App streams not moving to new default

```
Symptom: Changed default but Firefox/Spotify still plays on old device
```

1. Use `D` (capital) instead of `d` — this also moves all active streams.
2. Go to Streams screen (`S`) and press `M` to move everything manually.
3. Some apps (notably browsers) may need a restart after sink changes.
4. PipeWire respects per-app routing overrides; check `pavucontrol` if available.

---

### Combined output becomes stale after reboot

```
Symptom: An old combined sink shows "⚠ Stale" status
```

Combined sinks are loaded as PipeWire/PA modules and don't survive reboot by default.
Multi Audio Sink will detect stale combined devices (module unloaded but sink still visible)
and mark them. Use `R` to clean them up.

To persist a combined sink across reboots, add to `/etc/pulse/default.pa` or
`~/.config/pipewire/pipewire-pulse.conf`:
```
load-module module-combine-sink sink_name=combined_main slaves=sink1,sink2
```

---

## Architecture

```
multi-audio-sink/
├── src/
│   ├── main.rs           # Entry point, terminal setup, async event loop
│   ├── app.rs            # App state machine, screens, wizard state, notifications
│   ├── events.rs         # Keyboard event dispatch (per-screen handlers)
│   ├── audio/
│   │   ├── mod.rs        # Module exports
│   │   ├── backend.rs    # All pactl calls (discovery, routing, combine, reset)
│   │   ├── models.rs     # Device, Stream, CombinedGroup, AudioState types
│   │   └── parser.rs     # JSON parsing from pactl output + unit tests
│   └── ui/
│       ├── mod.rs        # Render dispatch
│       ├── home.rs       # Dashboard (3-panel overview)
│       ├── devices.rs    # Device browser + detail panel
│       ├── streams.rs    # Active stream list + detail
│       ├── combine.rs    # 3-step combine wizard
│       ├── confirm.rs    # Confirmation dialog overlay
│       ├── help.rs       # Keyboard reference
│       └── widgets.rs    # Color palette, shared widgets, notification banner
```

### Backend strategy

All audio system interaction goes through `pactl` (ships with `libpulse`, works on both
PipeWire and PulseAudio). The backend:

- Uses `--format=json` for structured data parsing
- Runs multiple queries concurrently with `tokio::try_join!`
- Detects combined sinks by checking `module-combine-sink` entries
- Falls back gracefully with plain-English error messages

### Known limitations / assumptions

1. **pactl required** — must be installed (`pacman -S libpulse`)
2. **PulseAudio compat layer** — `pipewire-pulse` must be active
3. **Combined sinks are ephemeral** — don't survive reboot without manual config
4. **No native PipeWire API** — uses pactl for broad compatibility and stability
5. **Volume/mute control** — backend supports it, not yet exposed in the TUI as separate
   actions (uses device-level controls via pactl)
6. **Sources/microphones** — not currently managed; focus is output routing

---

## Dependencies

| Crate        | Purpose                          |
|--------------|----------------------------------|
| `ratatui`    | TUI rendering framework          |
| `crossterm`  | Terminal I/O and event handling  |
| `tokio`      | Async runtime for concurrent ops |
| `clap`       | CLI argument parsing             |
| `serde_json` | Parsing pactl JSON output        |
| `anyhow`     | Ergonomic error handling         |

---

## License

MIT — see LICENSE file.
