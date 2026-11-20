//! Rotating file logs (REL-007). Clipboard content is never logged (PRIV-003): the content
//! types redact themselves in `Debug` and a sentinel test guards the flows.

use std::path::Path;

use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};

/// Keeps the background log writer alive; logs are flushed when it is dropped.
pub struct LogGuard(#[allow(dead_code)] WorkerGuard);

/// Number of daily log files kept.
const MAX_LOG_FILES: usize = 5;

fn level_from_env() -> Level {
    std::env::var("CLIPVAULT_LOG")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(Level::INFO)
}

/// Starts logging to `<log_dir>/clipvault.<date>.log`. Returns `None` (and the app runs without
/// file logs) if the folder cannot be used.
pub fn init(log_dir: &Path) -> Option<LogGuard> {
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("clipvault")
        .filename_suffix("log")
        .max_log_files(MAX_LOG_FILES)
        .build(log_dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_max_level(level_from_env())
        .try_init()
        .ok()?;
    Some(LogGuard(guard))
}
