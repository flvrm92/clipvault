//! Use cases (pattern: Command — specs/04 §P-04): one struct per action, `execute` method.

mod capture_clipboard;
mod capture_state;
mod history;
mod maintenance;
mod paste;
mod paste_snippet;
mod settings;
mod snippets;

pub use capture_clipboard::{CaptureClipboard, CaptureDependencies, CaptureOutcome};
pub use capture_state::{CaptureState, SetCaptureState, UpdateSettings};
pub use history::{ClearHistory, DeleteItem, GetItem, ListHistory, TogglePin};
pub use maintenance::{PurgeExpired, ReconcileImages, ReconcileReport, RetentionDependencies};
pub use paste::{CopyItem, FallbackReason, PasteDependencies, PasteItem, PasteMode, PasteOutcome};
pub use paste_snippet::{PasteSnippet, PasteSnippetDependencies, SnippetTarget};
pub use settings::{
    ChangeRetention, ChangeSizeLimit, ClearOnExit, ExcludeApp, IncludeApp, ListExclusionCandidates,
    RestoreDefaults, SizeLimit,
};
pub use snippets::{
    ApplyHotkeyBindings, ChangeOpenHotkey, DeleteSnippet, ListSnippets, SaveSnippet,
    SnippetDependencies, SnippetDraft, SuspendHotkeys,
};

pub use crate::error::HotkeyConflict;
pub use crate::ports::Suspension;
