//! Pausing and resuming capture (CAP-008, TRAY-002, TRAY-003).

use std::sync::Arc;

use crate::AppError;
use crate::ports::{AppEvent, EventPublisher, SettingsRepository};

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
