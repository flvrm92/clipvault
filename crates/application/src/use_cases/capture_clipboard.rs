//! Records a clipboard change in the history (CAP-001..012, specs/03 §5.1).

use std::sync::Arc;

use clipvault_domain::capture::{CaptureDecision, CapturedContent, IgnoreReason, NewClip};
use clipvault_domain::clip::{ClipContent, NewClipItem};
use clipvault_domain::ids::ClipId;
use clipvault_domain::image::{FileKey, ImageMeta};

use crate::AppError;
use crate::capture::{CaptureContext, CapturePipeline, OwnWriteRegistry};
use crate::ports::{
    AppEvent, ClipboardSnapshot, Clock, EventPublisher, HistoryRepository, ImageStore,
    SettingsRepository,
};

/// Ports required by [`CaptureClipboard`].
pub struct CaptureDependencies {
    /// History persistence.
    pub history: Arc<dyn HistoryRepository>,
    /// Image files.
    pub images: Arc<dyn ImageStore>,
    /// User settings.
    pub settings: Arc<dyn SettingsRepository>,
    /// Current time.
    pub clock: Arc<dyn Clock>,
    /// UI notifications.
    pub events: Arc<dyn EventPublisher>,
    /// Clipboard changes made by ClipVault (shared with the paste use cases).
    pub own_writes: Arc<OwnWriteRegistry>,
}

/// What happened to a clipboard change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureOutcome {
    /// Stored as a new item.
    Stored(ClipId),
    /// Existing item moved to the top.
    Promoted(ClipId),
    /// Not recorded.
    Ignored(IgnoreReason),
}

/// Evaluates a clipboard snapshot and records it.
pub struct CaptureClipboard {
    pipeline: CapturePipeline,
    history: Arc<dyn HistoryRepository>,
    images: Arc<dyn ImageStore>,
    settings: Arc<dyn SettingsRepository>,
    clock: Arc<dyn Clock>,
    events: Arc<dyn EventPublisher>,
}

impl CaptureClipboard {
    /// Builds the use case with the standard capture pipeline.
    pub fn new(deps: CaptureDependencies) -> Self {
        Self {
            pipeline: CapturePipeline::standard(deps.own_writes, Arc::clone(&deps.history)),
            history: deps.history,
            images: deps.images,
            settings: deps.settings,
            clock: deps.clock,
            events: deps.events,
        }
    }

    /// Captures one clipboard change. Settings are read on every call so changes apply
    /// immediately (CFG-002).
    ///
    /// # Errors
    /// Port failures. When storing an image fails half-way, no orphan file is left (REL-002).
    pub fn execute(&self, snapshot: ClipboardSnapshot) -> Result<CaptureOutcome, AppError> {
        let settings = self.settings.load()?;
        let decision = self
            .pipeline
            .evaluate(CaptureContext::new(snapshot, settings.capture))?;

        let outcome = match decision {
            CaptureDecision::Store(clip) => CaptureOutcome::Stored(self.store(clip)?),
            CaptureDecision::Promote(id) => {
                self.history.touch(id, self.clock.now())?;
                CaptureOutcome::Promoted(id)
            }
            CaptureDecision::Ignore(reason) => return Ok(CaptureOutcome::Ignored(reason)),
        };
        self.events.publish(AppEvent::HistoryChanged);
        Ok(outcome)
    }

    fn store(&self, clip: NewClip) -> Result<ClipId, AppError> {
        let NewClip {
            content,
            hash,
            source_app,
        } = clip;
        let new_item = |content| NewClipItem {
            content,
            hash,
            source_app,
            captured_at: self.clock.now(),
        };

        match content {
            CapturedContent::Text(text) => {
                Ok(self.history.insert(new_item(ClipContent::Text(text)))?)
            }
            CapturedContent::Image(pixels) => {
                let file_key = FileKey::from_hash(&hash);
                let stored = self.images.save(&file_key, &pixels)?;
                let meta = ImageMeta {
                    file_key: file_key.clone(),
                    width: pixels.width(),
                    height: pixels.height(),
                    file_size: stored.file_size,
                    thumb_width: stored.thumb_width,
                    thumb_height: stored.thumb_height,
                };
                self.history
                    .insert(new_item(ClipContent::Image(meta)))
                    .map_err(|error| {
                        // Best effort: reconciliation at startup removes leftovers (RET-005).
                        let _ = self.images.remove(&file_key);
                        error.into()
                    })
            }
        }
    }
}
