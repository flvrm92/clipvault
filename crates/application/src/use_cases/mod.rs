//! Use cases (pattern: Command — specs/04 §P-04): one struct per action, `execute` method.

mod capture_clipboard;
mod capture_state;
mod history;
mod maintenance;
mod paste;

pub use capture_clipboard::{CaptureClipboard, CaptureDependencies, CaptureOutcome};
pub use capture_state::{CaptureState, SetCaptureState};
pub use history::{ClearHistory, DeleteItem, GetItem, ListHistory, TogglePin};
pub use maintenance::{PurgeExpired, ReconcileImages, ReconcileReport, RetentionDependencies};
pub use paste::{CopyItem, FallbackReason, PasteDependencies, PasteItem, PasteMode, PasteOutcome};
