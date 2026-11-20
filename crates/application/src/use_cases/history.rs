//! History browsing and management (HIST-001..007).

use std::sync::Arc;

use clipvault_domain::clip::PinState;
use clipvault_domain::ids::ClipId;
use clipvault_domain::image::FileKey;

use crate::AppError;
use crate::ports::{
    AppEvent, ClearScope, EventPublisher, HistoryQuery, HistoryRepository, ImageStore, Page,
};
use clipvault_domain::clip::ClipItem;

/// Removes image files after their items were deleted. A failure leaves an orphan file that
/// the startup reconciliation removes (RET-005), so it is logged instead of failing the action.
pub(crate) fn remove_image_files(images: &dyn ImageStore, keys: &[FileKey]) {
    for key in keys {
        if let Err(error) = images.remove(key) {
            tracing::warn!(%error, "failed to remove image file; reconciliation will retry");
        }
    }
}

/// Lists a page of the history (HIST-001..004).
pub struct ListHistory {
    history: Arc<dyn HistoryRepository>,
}

impl ListHistory {
    /// Builds the use case.
    pub fn new(history: Arc<dyn HistoryRepository>) -> Self {
        Self { history }
    }

    /// Returns a page. The limit is clamped to `1..=MAX_PAGE_SIZE`; a blank search means no search.
    ///
    /// # Errors
    /// Storage failures.
    pub fn execute(&self, mut query: HistoryQuery) -> Result<Page<ClipItem>, AppError> {
        query.limit = query.limit.clamp(1, HistoryQuery::MAX_PAGE_SIZE);
        query.search = query
            .search
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        Ok(self.history.list(&query)?)
    }
}

/// Reads one item in full, e.g. for the preview (HIST-008).
pub struct GetItem {
    history: Arc<dyn HistoryRepository>,
}

impl GetItem {
    /// Builds the use case.
    pub fn new(history: Arc<dyn HistoryRepository>) -> Self {
        Self { history }
    }

    /// The item with `id`.
    ///
    /// # Errors
    /// [`crate::ports::PortError::NotFound`] or storage failures.
    pub fn execute(&self, id: ClipId) -> Result<ClipItem, AppError> {
        Ok(self
            .history
            .get(id)?
            .ok_or(crate::ports::PortError::NotFound)?)
    }
}

/// Pins an unpinned item or unpins a pinned one (HIST-005).
pub struct TogglePin {
    history: Arc<dyn HistoryRepository>,
    events: Arc<dyn EventPublisher>,
}

impl TogglePin {
    /// Builds the use case.
    pub fn new(history: Arc<dyn HistoryRepository>, events: Arc<dyn EventPublisher>) -> Self {
        Self { history, events }
    }

    /// Flips the pin state and returns the new state.
    ///
    /// # Errors
    /// [`crate::ports::PortError::NotFound`] or storage failures.
    pub fn execute(&self, id: ClipId) -> Result<PinState, AppError> {
        let item = self
            .history
            .get(id)?
            .ok_or(crate::ports::PortError::NotFound)?;
        let state = PinState::from_flag(item.pinned).toggled();
        self.history.set_pin_state(id, state)?;
        self.events.publish(AppEvent::HistoryChanged);
        Ok(state)
    }
}

/// Deletes one item and, for images, its file (HIST-006).
pub struct DeleteItem {
    history: Arc<dyn HistoryRepository>,
    images: Arc<dyn ImageStore>,
    events: Arc<dyn EventPublisher>,
}

impl DeleteItem {
    /// Builds the use case.
    pub fn new(
        history: Arc<dyn HistoryRepository>,
        images: Arc<dyn ImageStore>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            history,
            images,
            events,
        }
    }

    /// Deletes the item.
    ///
    /// # Errors
    /// [`crate::ports::PortError::NotFound`] or storage failures of the database.
    pub fn execute(&self, id: ClipId) -> Result<(), AppError> {
        let key = self.history.delete(id)?;
        remove_image_files(self.images.as_ref(), key.as_slice());
        self.events.publish(AppEvent::HistoryChanged);
        Ok(())
    }
}

/// Clears the history (HIST-007).
pub struct ClearHistory {
    history: Arc<dyn HistoryRepository>,
    images: Arc<dyn ImageStore>,
    events: Arc<dyn EventPublisher>,
}

impl ClearHistory {
    /// Builds the use case.
    pub fn new(
        history: Arc<dyn HistoryRepository>,
        images: Arc<dyn ImageStore>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            history,
            images,
            events,
        }
    }

    /// Removes the items in `scope` and returns how many were removed.
    ///
    /// # Errors
    /// Storage failures of the database.
    pub fn execute(&self, scope: ClearScope) -> Result<usize, AppError> {
        let removal = self.history.clear(scope)?;
        remove_image_files(self.images.as_ref(), &removal.image_keys);
        if removal.count > 0 {
            self.events.publish(AppEvent::HistoryChanged);
        }
        Ok(removal.count)
    }
}
