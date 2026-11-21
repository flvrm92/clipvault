use clipvault_domain::app_name::AppName;

use super::PortError;

/// Applications the user can pick to exclude from capture (CAP-009).
pub trait RunningApps: Send + Sync {
    /// Executables that own a visible top-level window, ClipVault itself excluded. Order and
    /// duplicates are unspecified.
    ///
    /// # Errors
    /// [`PortError::Unavailable`] when the windows cannot be enumerated.
    fn list(&self) -> Result<Vec<AppName>, PortError>;
}
