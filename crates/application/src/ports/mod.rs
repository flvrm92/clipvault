//! Ports: traits implemented by infrastructure adapters (specs/03 §3.2).
//!
//! Pattern: Ports & Adapters — specs/04 §P-01; Repository — §P-02.
//! Ports are added as use cases need them.

mod clipboard;
mod clock;
mod events;
mod focus;
mod history;
mod hotkeys;
mod images;
mod settings;

pub use clipboard::{
    ClipboardContent, ClipboardReader, ClipboardSequence, ClipboardSnapshot, ClipboardWriter,
    PrivacyFlags,
};
pub use clock::Clock;
pub use events::{AppEvent, EventPublisher};
pub use focus::{FocusTracker, KeystrokeSender, ScreenPoint, TargetWindow, WindowHandle};
pub use history::{ClearScope, HistoryQuery, HistoryRepository, KindFilter, Page, Removal};
pub use hotkeys::{Delay, HotkeyRegistrar, SnippetRepository, Suspension};
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
    /// The resource is taken (a shortcut held by another app, a duplicate name).
    #[error("already in use: {0}")]
    InUse(String),
}
