//! Use cases (pattern: Command — specs/04 §P-04): one struct per action, `execute` method.

mod capture_clipboard;
mod history;
mod maintenance;

pub use capture_clipboard::{CaptureClipboard, CaptureDependencies, CaptureOutcome};
pub use history::{ClearHistory, DeleteItem, ListHistory, TogglePin};
pub use maintenance::{PurgeExpired, ReconcileImages, ReconcileReport, RetentionDependencies};
