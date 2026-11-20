//! Use cases (pattern: Command — specs/04 §P-04): one struct per action, `execute` method.

mod capture_clipboard;

pub use capture_clipboard::{CaptureClipboard, CaptureDependencies, CaptureOutcome};
