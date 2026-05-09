mod app;
mod audio;
mod events;
mod logger;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::App;
use crate::audio::backend::AudioBackend;

// ─── CLI args ─────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "audio_tui",
    version = "0.1.0",
    about = "Human-friendly TUI for PipeWire/PulseAudio audio management",
    long_about = "AudioCtl — manage your Linux audio outputs without memorizing commands.\n\nRequires PipeWire (or PulseAudio) with pactl available.",
)]
struct Cli {
    /// Tick rate in milliseconds (controls notification expiry checks)
    #[arg(short, long, default_value = "250")]
    tick_rate: u64,

    /// Skip initial health check (useful if pactl is slow to start)
    #[arg(long)]
    no_health_check: bool,

    /// Enable debug logging to a file (default path: audio_tui_debug.log).
    /// All events — pactl calls, parse results, key presses, state transitions —
    /// are recorded step-by-step with millisecond timestamps.
    ///
    /// Examples:
    ///   audio_tui --debug
    ///   audio_tui --debug /tmp/mytrace.log
    #[arg(long, value_name = "LOG_FILE", num_args = 0..=1,
          default_missing_value = "audio_tui_debug.log")]
    debug: Option<String>,
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // ── Debug logger init ─────────────────────────────────────────────────
    if let Some(ref log_path) = cli.debug {
        match logger::init(log_path) {
            Ok(()) => {
                eprintln!("AudioCtl: debug logging enabled → {}", log_path);
            }
            Err(e) => {
                eprintln!("AudioCtl: WARNING — could not open log file {:?}: {}", log_path, e);
            }
        }
    }

    dlog!("INIT", "AudioCtl starting up");
    dlog!("INIT", "CLI args: tick_rate={}ms no_health_check={} debug={:?}",
        cli.tick_rate, cli.no_health_check, cli.debug);

    // ── Verify pactl is available early ──────────────────────────────────
    dlog!("INIT", "Checking if pactl is in PATH");
    match tokio::process::Command::new("which").arg("pactl").output().await {
        Ok(out) => {
            let path = String::from_utf8_lossy(&out.stdout);
            dlog!("INIT", "pactl found at: {}", path.trim());
        }
        Err(e) => {
            dlog!("INIT", "WARNING: 'which pactl' failed: {}", e);
        }
    }

    // ── Backend init ──────────────────────────────────────────────────────
    let backend = AudioBackend::new();

    if !cli.no_health_check {
        dlog!("INIT", "Running health check (pactl info)");
        match backend.check_health().await {
            Ok(server_name) => {
                eprintln!("AudioCtl: connected to {}", server_name);
                dlog!("INIT", "Health check passed: server={:?}", server_name);
            }
            Err(e) => {
                eprintln!("AudioCtl: Warning — {}", e);
                eprintln!("Launching anyway. Press F5 to retry after starting PipeWire.");
                dlog!("INIT", "Health check FAILED: {}", e);
            }
        }
    } else {
        dlog!("INIT", "Health check skipped (--no-health-check)");
    }

    // ── App init ──────────────────────────────────────────────────────────
    let mut app = App::new(backend);
    dlog!("INIT", "App created, fetching initial state");

    // Initial state fetch (non-fatal if it fails)
    match app.backend.fetch_state().await {
        Ok(state) => {
            dlog!("INIT", "Initial fetch OK: {} devices, {} streams",
                state.devices.len(), state.streams.len());
            app.server_name = app.backend.check_health().await.unwrap_or_default();
            app.audio_state = state;
        }
        Err(e) => {
            dlog!("INIT", "Initial fetch FAILED: {}", e);
            app.refresh_error = Some(e.to_string());
        }
    }

    dlog!("INIT", "Entering TUI — setting up terminal");

    // ── Terminal setup ────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend_term = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend_term)?;

    dlog!("INIT", "Terminal ready, starting event loop");

    // ── Run ───────────────────────────────────────────────────────────────
    let result = run_app(&mut terminal, &mut app, cli.tick_rate).await;

    // ── Terminal restore ──────────────────────────────────────────────────
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    dlog!("INIT", "Event loop ended, terminal restored");

    if let Err(e) = result {
        dlog!("INIT", "FATAL error from run_app: {}", e);
        eprintln!("AudioCtl exited with error: {}", e);
        std::process::exit(1);
    }

    dlog!("INIT", "Clean exit");
    println!("AudioCtl — goodbye!");
    Ok(())
}

// ─── Event loop ───────────────────────────────────────────────────────────────

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tick_rate_ms: u64,
) -> Result<()> {
    let tick_rate = Duration::from_millis(tick_rate_ms);
    let mut last_tick = Instant::now();
    let mut animation_tick: u64 = 0;

    dlog!("LOOP", "Event loop started, tick_rate={}ms", tick_rate_ms);

    loop {
        // Draw
        terminal.draw(|f| ui::render(f, app, animation_tick))?;

        // Poll for events with a timeout equal to tick rate
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            let ev = event::read()?;
            dlog!("LOOP", "Raw event: {:?}", ev);
            let needs_redraw = events::handle_event(app, ev).await;
            if needs_redraw {
                dlog!("LOOP", "Event handler requested redraw/refresh");
            }
        }

        // Tick
        if last_tick.elapsed() >= tick_rate {
            app.tick_notifications();
            animation_tick = animation_tick.wrapping_add(1);
            last_tick = Instant::now();
        }

        if app.should_quit {
            dlog!("LOOP", "should_quit=true, exiting loop");
            break;
        }
    }

    dlog!("LOOP", "Event loop exited normally");
    Ok(())
}
