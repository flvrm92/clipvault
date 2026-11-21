//! Settings window actions (CFG-004, CAP-006, CAP-009, RET-004, RET-006).

use std::collections::BTreeSet;
use std::sync::Arc;

use clipvault_domain::app_name::AppName;
use clipvault_domain::hotkey::Hotkey;
use clipvault_domain::retention::RetentionPolicy;
use clipvault_domain::settings::{Settings, SettingsSection};

use super::maintenance::{RetentionDependencies, apply_retention};
use super::snippets::{ChangeOpenHotkey, SnippetDependencies};
use super::{ClearHistory, PurgeExpired, UpdateSettings};
use crate::AppError;
use crate::ports::{
    AppEvent, ClearScope, EventPublisher, HistoryRepository, ImageStore, RunningApps,
    SettingsRepository,
};

/// Changes the retention policy and applies it right away (RET-004).
pub struct ChangeRetention {
    deps: RetentionDependencies,
}

impl ChangeRetention {
    /// Builds the use case.
    pub const fn new(deps: RetentionDependencies) -> Self {
        Self { deps }
    }

    /// How many items `policy` would remove now, for the confirmation shown before applying it.
    ///
    /// # Errors
    /// Storage failures.
    pub fn preview(&self, policy: RetentionPolicy) -> Result<usize, AppError> {
        PurgeExpired::new(self.deps.clone()).preview(policy)
    }

    /// Saves `policy` and removes what it selects; returns how many items were removed.
    ///
    /// # Errors
    /// Storage failures.
    pub fn execute(&self, policy: RetentionPolicy) -> Result<usize, AppError> {
        UpdateSettings::new(self.deps.settings.clone(), self.deps.events.clone())
            .execute(|s| s.retention.policy = policy)?;
        self.purge(policy)
    }

    fn purge(&self, policy: RetentionPolicy) -> Result<usize, AppError> {
        let removed = apply_retention(
            self.deps.history.as_ref(),
            self.deps.images.as_ref(),
            policy,
            self.deps.clock.now(),
        )?;
        if removed > 0 {
            self.deps.events.publish(AppEvent::HistoryChanged);
        }
        Ok(removed)
    }
}

/// Resets one section of the settings window to its defaults (CFG-004).
pub struct RestoreDefaults {
    retention: RetentionDependencies,
    snippets: SnippetDependencies,
}

impl RestoreDefaults {
    /// Builds the use case.
    pub const fn new(retention: RetentionDependencies, snippets: SnippetDependencies) -> Self {
        Self {
            retention,
            snippets,
        }
    }

    /// Restores `section`; returns how many history items the restored retention removed.
    ///
    /// The default open shortcut is registered first: when it is taken, nothing changes.
    ///
    /// # Errors
    /// Shortcut conflicts ([`AppError::HotkeyTaken`], [`crate::ports::PortError::InUse`]) and
    /// storage failures.
    pub fn execute(&self, section: SettingsSection) -> Result<usize, AppError> {
        if section == SettingsSection::Shortcuts {
            ChangeOpenHotkey::new(self.snippets.clone())
                .execute(&Hotkey::default_open().to_string())?;
        }
        let before = self.retention.settings.load()?;
        let after = UpdateSettings::new(
            self.retention.settings.clone(),
            self.retention.events.clone(),
        )
        .execute(|s| s.restore_defaults(section))?;
        if before.capture.paused != after.capture.paused {
            self.retention.events.publish(AppEvent::CaptureStateChanged);
        }
        if section == SettingsSection::History {
            return ChangeRetention::new(self.retention.clone()).purge(after.retention.policy);
        }
        Ok(0)
    }
}

/// Which size limit to change (CAP-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeLimit {
    /// Largest text recorded.
    Text,
    /// Largest image recorded.
    Image,
}

/// Changes a size limit to one of the offered options (CAP-006).
pub struct ChangeSizeLimit {
    settings: Arc<dyn SettingsRepository>,
    events: Arc<dyn EventPublisher>,
}

impl ChangeSizeLimit {
    /// Builds the use case.
    pub fn new(settings: Arc<dyn SettingsRepository>, events: Arc<dyn EventPublisher>) -> Self {
        Self { settings, events }
    }

    /// Validates and saves `bytes` as the `limit`.
    ///
    /// # Errors
    /// [`AppError::InvalidSetting`] for values that are not offered; storage failures.
    pub fn execute(&self, limit: SizeLimit, bytes: u64) -> Result<(), AppError> {
        let mut settings = self.settings.load()?;
        match limit {
            SizeLimit::Text => settings.capture.set_max_text_bytes(bytes)?,
            SizeLimit::Image => settings.capture.set_max_image_bytes(bytes)?,
        }
        self.save(&settings)
    }

    fn save(&self, settings: &Settings) -> Result<(), AppError> {
        self.settings.save(settings)?;
        self.events.publish(AppEvent::SettingsChanged);
        Ok(())
    }
}

/// Ignores copies made in an application (CAP-009).
pub struct ExcludeApp {
    settings: Arc<dyn SettingsRepository>,
    events: Arc<dyn EventPublisher>,
}

impl ExcludeApp {
    /// Builds the use case.
    pub fn new(settings: Arc<dyn SettingsRepository>, events: Arc<dyn EventPublisher>) -> Self {
        Self { settings, events }
    }

    /// Adds the executable named `name` (bare name or path); returns its normalized name.
    ///
    /// # Errors
    /// [`AppError::InvalidAppName`] for blank names; storage failures.
    pub fn execute(&self, name: &str) -> Result<AppName, AppError> {
        let app = AppName::parse(name)?;
        let added = app.clone();
        UpdateSettings::new(self.settings.clone(), self.events.clone())
            .execute(|s| _ = s.capture.exclude(added))?;
        Ok(app)
    }
}

/// Captures copies made in an excluded application again (CAP-009).
pub struct IncludeApp {
    settings: Arc<dyn SettingsRepository>,
    events: Arc<dyn EventPublisher>,
}

impl IncludeApp {
    /// Builds the use case.
    pub fn new(settings: Arc<dyn SettingsRepository>, events: Arc<dyn EventPublisher>) -> Self {
        Self { settings, events }
    }

    /// Removes the executable named `name` from the excluded applications.
    ///
    /// # Errors
    /// [`AppError::InvalidAppName`] for blank names; storage failures.
    pub fn execute(&self, name: &str) -> Result<(), AppError> {
        let app = AppName::parse(name)?;
        UpdateSettings::new(self.settings.clone(), self.events.clone())
            .execute(|s| _ = s.capture.include(&app))?;
        Ok(())
    }
}

/// Running applications the user can still exclude (CAP-009).
pub struct ListExclusionCandidates {
    running: Arc<dyn RunningApps>,
    settings: Arc<dyn SettingsRepository>,
}

impl ListExclusionCandidates {
    /// Builds the use case.
    pub fn new(running: Arc<dyn RunningApps>, settings: Arc<dyn SettingsRepository>) -> Self {
        Self { running, settings }
    }

    /// Sorted, unique applications with a visible window that are not excluded yet.
    ///
    /// # Errors
    /// Window enumeration or storage failures.
    pub fn execute(&self) -> Result<Vec<AppName>, AppError> {
        let capture = self.settings.load()?.capture;
        let candidates: BTreeSet<AppName> = self
            .running
            .list()?
            .into_iter()
            .filter(|app| !capture.is_excluded(app))
            .collect();
        Ok(candidates.into_iter().collect())
    }
}

/// Clears the unpinned history when the app exits, if the user asked for it (RET-006).
pub struct ClearOnExit {
    history: Arc<dyn HistoryRepository>,
    images: Arc<dyn ImageStore>,
    settings: Arc<dyn SettingsRepository>,
}

impl ClearOnExit {
    /// Builds the use case.
    pub fn new(
        history: Arc<dyn HistoryRepository>,
        images: Arc<dyn ImageStore>,
        settings: Arc<dyn SettingsRepository>,
    ) -> Self {
        Self {
            history,
            images,
            settings,
        }
    }

    /// Removes the unpinned items when enabled; returns how many were removed.
    ///
    /// # Errors
    /// Storage failures.
    pub fn execute(&self) -> Result<usize, AppError> {
        if !self.settings.load()?.retention.clear_on_exit {
            return Ok(0);
        }
        ClearHistory::new(
            self.history.clone(),
            self.images.clone(),
            Arc::new(NoEvents),
        )
        .execute(ClearScope::UnpinnedOnly)
    }
}

/// Nobody listens while the app is exiting.
struct NoEvents;

impl EventPublisher for NoEvents {
    fn publish(&self, _event: AppEvent) {}
}
