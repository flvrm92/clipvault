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
    /// Text size limits offered to the user, in bytes (CAP-006).
    pub const TEXT_LIMIT_OPTIONS: [u64; 4] = [256 * KIB, MIB, 5 * MIB, 10 * MIB];
    /// Image size limits offered to the user, in uncompressed bytes (CAP-006).
    pub const IMAGE_LIMIT_OPTIONS: [u64; 4] = [16 * MIB, 64 * MIB, 128 * MIB, 256 * MIB];

    /// Whether copies from `app` must be ignored.
    pub fn is_excluded(&self, app: &AppName) -> bool {
        self.excluded_apps.contains(app)
    }

    /// Ignores copies from `app`; returns `false` when it was already excluded (CAP-009).
    pub fn exclude(&mut self, app: AppName) -> bool {
        if self.is_excluded(&app) {
            return false;
        }
        self.excluded_apps.push(app);
        true
    }

    /// Captures copies from `app` again; returns `false` when it was not excluded (CAP-009).
    pub fn include(&mut self, app: &AppName) -> bool {
        let before = self.excluded_apps.len();
        self.excluded_apps.retain(|a| a != app);
        self.excluded_apps.len() != before
    }

    /// Changes the text size limit to one of [`Self::TEXT_LIMIT_OPTIONS`].
    ///
    /// # Errors
    /// [`SettingsError::UnsupportedTextLimit`] for any other value.
    pub fn set_max_text_bytes(&mut self, bytes: u64) -> Result<(), SettingsError> {
        if !Self::TEXT_LIMIT_OPTIONS.contains(&bytes) {
            return Err(SettingsError::UnsupportedTextLimit(bytes));
        }
        self.max_text_bytes = bytes;
        Ok(())
    }

    /// Changes the image size limit to one of [`Self::IMAGE_LIMIT_OPTIONS`].
    ///
    /// # Errors
    /// [`SettingsError::UnsupportedImageLimit`] for any other value.
    pub fn set_max_image_bytes(&mut self, bytes: u64) -> Result<(), SettingsError> {
        if !Self::IMAGE_LIMIT_OPTIONS.contains(&bytes) {
            return Err(SettingsError::UnsupportedImageLimit(bytes));
        }
        self.max_image_bytes = bytes;
        Ok(())
    }
}

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;

/// Errors changing a setting to a value that is not offered.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    /// Not one of [`CaptureSettings::TEXT_LIMIT_OPTIONS`].
    #[error("unsupported text size limit: {0} bytes")]
    UnsupportedTextLimit(u64),
    /// Not one of [`CaptureSettings::IMAGE_LIMIT_OPTIONS`].
    #[error("unsupported image size limit: {0} bytes")]
    UnsupportedImageLimit(u64),
}

/// A section of the settings window with its own "Restore defaults" button (CFG-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    /// Language and capture state.
    General,
    /// Retention and size limits.
    History,
    /// Open shortcut and snippet behaviour (not the snippets themselves).
    Shortcuts,
    /// Excluded applications.
    Privacy,
    /// Theme and accent color.
    Appearance,
}

impl Settings {
    /// Resets the settings shown in `section` to their defaults (CFG-004); every other setting,
    /// the history and the snippets are kept.
    pub fn restore_defaults(&mut self, section: SettingsSection) {
        let defaults = Self::default();
        match section {
            SettingsSection::General => {
                self.ui.language = defaults.ui.language;
                self.capture.paused = defaults.capture.paused;
            }
            SettingsSection::History => {
                self.retention = defaults.retention;
                self.capture.max_text_bytes = defaults.capture.max_text_bytes;
                self.capture.max_image_bytes = defaults.capture.max_image_bytes;
            }
            SettingsSection::Shortcuts => {
                self.hotkeys = defaults.hotkeys;
                self.snippets = defaults.snippets;
            }
            SettingsSection::Privacy => self.capture.excluded_apps = defaults.capture.excluded_apps,
            SettingsSection::Appearance => {
                self.ui.theme = defaults.ui.theme;
                self.ui.use_accent_color = defaults.ui.use_accent_color;
            }
        }
    }
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            paused: false,
            max_text_bytes: MIB,
            max_image_bytes: 64 * MIB,
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
