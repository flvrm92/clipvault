//! Start with Windows (CFG-003) and the first-run welcome (TRAY-006).

use std::sync::Arc;

use super::UpdateSettings;
use crate::AppError;
use crate::ports::{EventPublisher, SettingsRepository, StartupManager, StartupState};

/// Reads whether ClipVault starts with Windows (CFG-003).
pub struct GetStartup {
    manager: Arc<dyn StartupManager>,
}

impl GetStartup {
    /// Builds the use case.
    pub fn new(manager: Arc<dyn StartupManager>) -> Self {
        Self { manager }
    }

    /// The current state.
    ///
    /// # Errors
    /// The system cannot be queried.
    pub fn execute(&self) -> Result<StartupState, AppError> {
        Ok(self.manager.state()?)
    }
}

/// Turns starting with Windows on or off (CFG-003).
pub struct SetStartup {
    manager: Arc<dyn StartupManager>,
}

impl SetStartup {
    /// Builds the use case.
    pub fn new(manager: Arc<dyn StartupManager>) -> Self {
        Self { manager }
    }

    /// Applies `enabled` unless the state is locked by the user or a policy, which is never
    /// overridden; returns the resulting state.
    ///
    /// # Errors
    /// The registration fails.
    pub fn execute(&self, enabled: bool) -> Result<StartupState, AppError> {
        let current = self.manager.state()?;
        if !current.can_change() || current.is_enabled() == enabled {
            return Ok(current);
        }
        Ok(if enabled {
            self.manager.enable()?
        } else {
            self.manager.disable()?
        })
    }
}

/// How the user left the welcome screen (TRAY-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeChoice {
    /// "Começar" or "Abrir configurações", with the state of "Iniciar com o Windows".
    Confirmed {
        /// The checkbox was checked.
        start_with_windows: bool,
    },
    /// Closed without confirming: no consent to start with Windows.
    Dismissed,
}

/// Finishes the first run (TRAY-006, CFG-003).
pub struct CompleteWelcome {
    settings: Arc<dyn SettingsRepository>,
    events: Arc<dyn EventPublisher>,
    manager: Arc<dyn StartupManager>,
}

impl CompleteWelcome {
    /// Builds the use case.
    pub fn new(
        settings: Arc<dyn SettingsRepository>,
        events: Arc<dyn EventPublisher>,
        manager: Arc<dyn StartupManager>,
    ) -> Self {
        Self {
            settings,
            events,
            manager,
        }
    }

    /// Whether the welcome screen must still be shown.
    ///
    /// # Errors
    /// Settings storage failures.
    pub fn is_pending(&self) -> Result<bool, AppError> {
        Ok(!self.settings.load()?.general.first_run_done)
    }

    /// Records the first run and, with consent, starts with Windows; returns the startup state.
    ///
    /// # Errors
    /// Storage or registration failures (the first run is recorded before registering).
    pub fn execute(&self, choice: WelcomeChoice) -> Result<StartupState, AppError> {
        UpdateSettings::new(self.settings.clone(), self.events.clone())
            .execute(|s| s.general.first_run_done = true)?;
        match choice {
            WelcomeChoice::Confirmed {
                start_with_windows: true,
            } => SetStartup::new(self.manager.clone()).execute(true),
            _ => GetStartup::new(self.manager.clone()).execute(),
        }
    }
}
