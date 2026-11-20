//! Pausing and resuming capture (CAP-008, TRAY-002, TRAY-003).

use std::sync::Arc;

use clipvault_domain::settings::Settings;

use crate::AppError;
use crate::ports::{AppEvent, EventPublisher, SettingsRepository};

/// Applies a change to the settings and saves it immediately (CFG-002).
pub struct UpdateSettings {
    settings: Arc<dyn SettingsRepository>,
    events: Arc<dyn EventPublisher>,
}

impl UpdateSettings {
    /// Builds the use case.
    pub fn new(settings: Arc<dyn SettingsRepository>, events: Arc<dyn EventPublisher>) -> Self {
        Self { settings, events }
    }

    /// Loads, changes, saves and notifies.
    ///
    /// # Errors
    /// Settings storage failures.
    pub fn execute(&self, change: impl FnOnce(&mut Settings)) -> Result<Settings, AppError> {
        let mut settings = self.settings.load()?;
        change(&mut settings);
        self.settings.save(&settings)?;
        self.events.publish(AppEvent::SettingsChanged);
        Ok(settings)
    }
}

/// Whether clipboard changes are recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    /// Changes are recorded.
    Active,
    /// Changes are ignored.
    Paused,
}

/// Reads and changes the capture state, persisted in the settings.
pub struct SetCaptureState {
    settings: Arc<dyn SettingsRepository>,
    events: Arc<dyn EventPublisher>,
}

impl SetCaptureState {
    /// Builds the use case.
    pub fn new(settings: Arc<dyn SettingsRepository>, events: Arc<dyn EventPublisher>) -> Self {
        Self { settings, events }
    }

    /// The current state.
    ///
    /// # Errors
    /// Settings storage failures.
    pub fn current(&self) -> Result<CaptureState, AppError> {
        Ok(if self.settings.load()?.capture.paused {
            CaptureState::Paused
        } else {
            CaptureState::Active
        })
    }

    /// Persists `state` and notifies the UI and the tray.
    ///
    /// # Errors
    /// Settings storage failures.
    pub fn execute(&self, state: CaptureState) -> Result<(), AppError> {
        let mut settings = self.settings.load()?;
        settings.capture.paused = state == CaptureState::Paused;
        self.settings.save(&settings)?;
        self.events.publish(AppEvent::CaptureStateChanged);
        Ok(())
    }
}
