use clipvault_domain::settings::Settings;

use super::PortError;

/// Persistence of user settings. Invalid stored values fall back to defaults (CFG-006).
///
/// Every implementation must pass `clipvault_test_support::settings_repository_contract!`.
pub trait SettingsRepository: Send + Sync {
    /// Current settings.
    ///
    /// # Errors
    /// [`PortError::Storage`] when the store cannot be read at all.
    fn load(&self) -> Result<Settings, PortError>;

    /// Persists every setting.
    ///
    /// # Errors
    /// [`PortError::Storage`] on failure.
    fn save(&self, settings: &Settings) -> Result<(), PortError>;
}
