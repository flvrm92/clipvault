use clipvault_domain::settings::Settings;

use super::PortError;

/// Persistence of user settings. Invalid stored values fall back to defaults (CFG-006).
pub trait SettingsRepository: Send + Sync {
    /// Current settings.
    ///
    /// # Errors
    /// [`PortError::Storage`] when the store cannot be read at all.
    fn load(&self) -> Result<Settings, PortError>;
}
