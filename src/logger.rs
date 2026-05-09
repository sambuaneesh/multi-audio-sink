/// Global debug logger — writes timestamped events to a file.
///
/// Activated via `--debug [path]` (default: `mas_debug.log`).
/// All subsystems call `log!()` / `log_section!()` macros; when debug mode
/// is off those calls compile to nothing (zero-cost in production).
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

// ─── Global logger state ─────────────────────────────────────────────────────

static LOGGER: OnceLock<Mutex<DebugLogger>> = OnceLock::new();

pub struct DebugLogger {
    file: Option<File>,
    enabled: bool,
    start_us: u128,
}

impl DebugLogger {
    fn new() -> Self {
        Self {
            file: None,
            enabled: false,
            start_us: now_us(),
        }
    }

    /// Open the log file and enable logging
    pub fn enable(&mut self, path: &str) -> std::io::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        self.file = Some(file);
        self.enabled = true;
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn write_entry(&mut self, category: &str, msg: &str) {
        if !self.enabled {
            return;
        }
        let elapsed_ms = (now_us() - self.start_us) / 1000;
        let entry = format!(
            "[{:>8}ms] [{:<14}] {}\n",
            elapsed_ms, category, msg
        );
        if let Some(ref mut f) = self.file {
            let _ = f.write_all(entry.as_bytes());
            let _ = f.flush();
        }
    }
}

fn now_us() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros())
        .unwrap_or(0)
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Initialize the global logger. Call once from main before any logging.
pub fn init(path: &str) -> std::io::Result<()> {
    let logger = LOGGER.get_or_init(|| Mutex::new(DebugLogger::new()));
    let mut guard = logger.lock().unwrap();
    guard.enable(path)?;
    // Write session header
    guard.write_entry("INIT", &format!(
        "=== Multi Audio Sink debug session started === log: {}",
        path
    ));
    guard.write_entry("INIT", &format!(
        "Timestamp format: elapsed ms since app start | category | message"
    ));
    Ok(())
}

/// Returns whether debug logging is active
pub fn is_debug() -> bool {
    LOGGER.get()
        .and_then(|l| l.lock().ok())
        .map(|g| g.is_enabled())
        .unwrap_or(false)
}

/// Write a log entry. Category is left-padded to 14 chars.
pub fn log(category: &str, msg: &str) {
    if let Some(logger) = LOGGER.get() {
        if let Ok(mut guard) = logger.lock() {
            guard.write_entry(category, msg);
        }
    }
}

/// Write a visible section divider
pub fn log_section(title: &str) {
    log("────────────", &format!("──── {} ────", title));
}

// ─── Convenience macros ───────────────────────────────────────────────────────

/// Log a message if debug mode is active. Zero-cost if disabled.
#[macro_export]
macro_rules! dlog {
    ($cat:expr, $($arg:tt)*) => {
        if $crate::logger::is_debug() {
            $crate::logger::log($cat, &format!($($arg)*));
        }
    };
}

/// Log a section divider
#[macro_export]
macro_rules! dlog_section {
    ($title:expr) => {
        if $crate::logger::is_debug() {
            $crate::logger::log_section($title);
        }
    };
}
