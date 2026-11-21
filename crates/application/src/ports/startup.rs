use super::PortError;

/// Whether ClipVault starts with Windows (CFG-003, ADR-0008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupState {
    /// Starts with Windows.
    Enabled,
    /// Does not start with Windows; the user may turn it on.
    Disabled,
    /// Turned off in the Windows startup settings; only the user can turn it on again there.
    DisabledByUser,
    /// Turned off by an organization policy.
    DisabledByPolicy,
    /// Turned on by an organization policy.
    EnabledByPolicy,
}

impl StartupState {
    /// Whether ClipVault starts with Windows in this state.
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled | Self::EnabledByPolicy)
    }

    /// Whether the app may change this state on the user's request.
    pub const fn can_change(self) -> bool {
        matches!(self, Self::Enabled | Self::Disabled)
    }
}

/// Start-with-Windows registration: MSIX `StartupTask` or the user's `Run` key.
pub trait StartupManager: Send + Sync {
    /// The current state.
    ///
    /// # Errors
    /// [`PortError::Unavailable`] when the system cannot be queried.
    fn state(&self) -> Result<StartupState, PortError>;

    /// Asks to start with Windows; returns the resulting state.
    ///
    /// # Errors
    /// [`PortError::Unavailable`] when the registration fails.
    fn enable(&self) -> Result<StartupState, PortError>;

    /// Stops starting with Windows; returns the resulting state.
    ///
    /// # Errors
    /// [`PortError::Unavailable`] when the registration fails.
    fn disable(&self) -> Result<StartupState, PortError>;
}
