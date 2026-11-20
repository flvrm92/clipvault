//! Ports: traits implemented by infrastructure adapters (specs/03 §3.2).
//!
//! Pattern: Ports & Adapters — specs/04 §P-01; Repository — §P-02.
//! Ports are added as use cases need them.

mod clipboard;
mod clock;
mod events;
mod history;
mod images;
mod settings;

pub use clipboard::{ClipboardSequence, ClipboardSnapshot, PrivacyFlags};
pub use clock::Clock;
pub use events::{AppEvent, EventPublisher};
pub use history::HistoryRepository;
pub use images::{ImageStore, StoredImage};
pub use settings::SettingsRepository;

/// Failure of a port, without leaking adapter details.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PortError {
    /// Persistent storage (database, files) failed.
    #[error("storage error: {0}")]
    Storage(String),
    /// A system resource (clipboard, window) is temporarily unavailable.
    #[error("resource unavailable: {0}")]
    Unavailable(String),
    /// The requested entity does not exist.
    #[error("not found")]
    NotFound,
}
