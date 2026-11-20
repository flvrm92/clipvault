//! Retention purge and image reconciliation (RET-001..005).

use std::collections::BTreeSet;
use std::sync::Arc;

use clipvault_domain::image::FileKey;
use clipvault_domain::retention::RetentionPolicy;
use clipvault_domain::time::Timestamp;

use super::history::remove_image_files;
use crate::AppError;
use crate::ports::{
    AppEvent, Clock, EventPublisher, HistoryRepository, ImageStore, SettingsRepository,
};

/// Deletes the items `policy` selects and their files; returns how many were deleted.
pub(crate) fn apply_retention(
    history: &dyn HistoryRepository,
    images: &dyn ImageStore,
    policy: RetentionPolicy,
    now: Timestamp,
) -> Result<usize, AppError> {
    let expired = policy.select_expired(&history.retention_candidates()?, now);
    if expired.is_empty() {
        return Ok(0);
    }
    let removal = history.delete_many(&expired)?;
    remove_image_files(images, &removal.image_keys);
    Ok(removal.count)
}

/// Ports required by [`PurgeExpired`].
#[derive(Clone)]
pub struct RetentionDependencies {
    /// History persistence.
    pub history: Arc<dyn HistoryRepository>,
    /// Image files.
    pub images: Arc<dyn ImageStore>,
    /// User settings (source of the policy).
    pub settings: Arc<dyn SettingsRepository>,
    /// Current time.
    pub clock: Arc<dyn Clock>,
    /// UI notifications.
    pub events: Arc<dyn EventPublisher>,
}

/// Applies the configured retention policy (RET-004: at startup, hourly and after a change).
pub struct PurgeExpired {
    deps: RetentionDependencies,
}

impl PurgeExpired {
    /// Builds the use case.
    pub const fn new(deps: RetentionDependencies) -> Self {
        Self { deps }
    }

    /// Removes the expired items and returns how many were removed.
    ///
    /// # Errors
    /// Storage failures of the database.
    pub fn execute(&self) -> Result<usize, AppError> {
        let policy = self.deps.settings.load()?.retention.policy;
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

    /// How many items `policy` would remove now — shown before a policy change is confirmed.
    ///
    /// # Errors
    /// Storage failures of the database.
    pub fn preview(&self, policy: RetentionPolicy) -> Result<usize, AppError> {
        let candidates = self.deps.history.retention_candidates()?;
        Ok(policy
            .select_expired(&candidates, self.deps.clock.now())
            .len())
    }
}

/// Outcome of [`ReconcileImages`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReconcileReport {
    /// Files on disk without a history item.
    pub orphan_files_removed: usize,
    /// Image items whose file is missing.
    pub broken_items_removed: usize,
}

/// Makes the database and the image folder consistent at startup (RET-005).
pub struct ReconcileImages {
    history: Arc<dyn HistoryRepository>,
    images: Arc<dyn ImageStore>,
}

impl ReconcileImages {
    /// Builds the use case.
    pub fn new(history: Arc<dyn HistoryRepository>, images: Arc<dyn ImageStore>) -> Self {
        Self { history, images }
    }

    /// Removes orphan files and items without files.
    ///
    /// # Errors
    /// Storage failures.
    pub fn execute(&self) -> Result<ReconcileReport, AppError> {
        let files: BTreeSet<FileKey> = self.images.keys()?.into_iter().collect();
        let entries = self.history.image_entries()?;
        let referenced: BTreeSet<&FileKey> = entries.iter().map(|(_, key)| key).collect();

        let orphans: Vec<FileKey> = files
            .iter()
            .filter(|key| !referenced.contains(key))
            .cloned()
            .collect();
        remove_image_files(self.images.as_ref(), &orphans);

        let broken: Vec<_> = entries
            .iter()
            .filter(|(_, key)| !files.contains(key))
            .map(|(id, _)| *id)
            .collect();
        let broken_items_removed = if broken.is_empty() {
            0
        } else {
            self.history.delete_many(&broken)?.count
        };

        Ok(ReconcileReport {
            orphan_files_removed: orphans.len(),
            broken_items_removed,
        })
    }
}
