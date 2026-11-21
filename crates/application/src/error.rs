//! Application error type (specs/03 §6).

use clipvault_domain::app_name::AppNameError;
use clipvault_domain::hotkey::HotkeyError;
use clipvault_domain::settings::SettingsError;
use clipvault_domain::snippet::SnippetError;

use crate::ports::PortError;

/// Which ClipVault shortcut already uses a hotkey (HK-003), with the snippet name for the
/// message "Atalho já usado pelo snippet 'Assinatura'".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyConflict {
    /// The shortcut that opens the popup.
    OpenHistory,
    /// A snippet, by name.
    Snippet(String),
}

/// Errors returned by use cases.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// A port (storage, clipboard, …) failed.
    #[error(transparent)]
    Port(#[from] PortError),
    /// A snippet field is invalid (SNP-001).
    #[error(transparent)]
    InvalidSnippet(#[from] SnippetError),
    /// A shortcut is invalid or reserved (HK-003).
    #[error(transparent)]
    InvalidHotkey(#[from] HotkeyError),
    /// Another snippet already has this name (SNP-001).
    #[error("snippet name already in use")]
    NameTaken,
    /// Another ClipVault shortcut already uses this hotkey (HK-003).
    #[error("shortcut already used by {0:?}")]
    HotkeyTaken(HotkeyConflict),
    /// A setting value that is not offered (CAP-006).
    #[error(transparent)]
    InvalidSetting(#[from] SettingsError),
    /// An application name that cannot be excluded (CAP-009).
    #[error(transparent)]
    InvalidAppName(#[from] AppNameError),
}
