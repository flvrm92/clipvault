//! User settings aggregate with the defaults of specs/05 §3.

use std::time::Duration;

use crate::app_name::AppName;
use crate::hotkey::Hotkey;
use crate::retention::RetentionPolicy;

/// All user settings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Settings {
    /// Clipboard capture.
    pub capture: CaptureSettings,
    /// History retention.
    pub retention: RetentionSettings,
    /// Global shortcuts.
    pub hotkeys: HotkeySettings,
    /// Snippet behaviour.
    pub snippets: SnippetSettings,
    /// Appearance and language.
    pub ui: UiSettings,
    /// Miscellaneous.
    pub general: GeneralSettings,
}

/// Capture settings (CAP-006, CAP-008, CAP-009).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureSettings {
    /// Capture paused from the tray or settings.
    pub paused: bool,
    /// Texts above this UTF-8 size are not recorded.
    pub max_text_bytes: u64,
    /// Images above this uncompressed size are not recorded.
    pub max_image_bytes: u64,
    /// Applications whose copies are ignored.
    pub excluded_apps: Vec<AppName>,
}

impl CaptureSettings {
    /// Whether copies from `app` must be ignored.
    pub fn is_excluded(&self, app: &AppName) -> bool {
        self.excluded_apps.contains(app)
    }
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            paused: false,
            max_text_bytes: 1024 * 1024,
            max_image_bytes: 25 * 1024 * 1024,
            excluded_apps: Vec::new(),
        }
    }
}

/// Retention settings (RET-001..002, RET-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RetentionSettings {
    /// Age and count rules.
    pub policy: RetentionPolicy,
    /// Clear unpinned history on exit.
    pub clear_on_exit: bool,
}

/// Global shortcut settings (HK-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotkeySettings {
    /// Opens the history popup.
    pub open: Hotkey,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            open: Hotkey::default_open(),
        }
    }
}

/// Snippet settings (SNP-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnippetSettings {
    /// Restore the previous clipboard content after pasting a snippet.
    pub restore_clipboard: bool,
    /// Delay before restoring.
    pub restore_delay: Duration,
}

impl Default for SnippetSettings {
    fn default() -> Self {
        Self {
            restore_clipboard: true,
            restore_delay: Duration::from_millis(500),
        }
    }
}

/// Theme preference (THM-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    /// Follow Windows.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

/// Interface language preference (I18N-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LanguagePreference {
    /// Follow the Windows display language.
    #[default]
    System,
    /// Português (Brasil).
    PtBr,
    /// English.
    En,
}

/// Appearance settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiSettings {
    /// Theme.
    pub theme: ThemeMode,
    /// Language.
    pub language: LanguagePreference,
    /// Use the Windows accent color instead of the brand color (THM-004).
    pub use_accent_color: bool,
}

/// Miscellaneous settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GeneralSettings {
    /// The welcome screen was already shown (TRAY-006).
    pub first_run_done: bool,
}

#[cfg(test)]
mod tests;
