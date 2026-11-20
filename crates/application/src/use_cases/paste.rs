//! Pasting and copying history items (PST-001..006, HIST-011, specs/03 §5.2).

use std::sync::Arc;

use clipvault_domain::clip::{ClipContent, ClipItem};
use clipvault_domain::ids::ClipId;

use crate::AppError;
use crate::capture::OwnWriteRegistry;
use crate::ports::{
    AppEvent, ClipboardContent, ClipboardWriter, Clock, EventPublisher, FocusTracker,
    HistoryRepository, ImageStore, KeystrokeSender, PortError, TargetWindow,
};

/// Ports required by [`PasteItem`] and [`CopyItem`].
pub struct PasteDependencies {
    /// History persistence.
    pub history: Arc<dyn HistoryRepository>,
    /// Image files.
    pub images: Arc<dyn ImageStore>,
    /// System clipboard.
    pub writer: Arc<dyn ClipboardWriter>,
    /// Foreground window management.
    pub focus: Arc<dyn FocusTracker>,
    /// Synthetic keystrokes.
    pub keys: Arc<dyn KeystrokeSender>,
    /// Clipboard changes made by ClipVault, shared with the capture pipeline (CAP-010).
    pub own_writes: Arc<OwnWriteRegistry>,
    /// Current time.
    pub clock: Arc<dyn Clock>,
    /// UI notifications.
    pub events: Arc<dyn EventPublisher>,
}

/// How text is pasted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMode {
    /// Exactly as captured (PST-001).
    AsIs,
    /// Without leading/trailing whitespace and line breaks (PST-002).
    TrimmedText,
}

/// Why an item was copied but not pasted (PST-005, PST-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    /// No window was in the foreground when the popup opened.
    NoTarget,
    /// The target window is gone or refused focus, or the keystroke failed.
    TargetUnavailable,
    /// The target runs elevated; Windows blocks synthetic input to it (UIPI).
    TargetElevated,
}

/// Result of [`PasteItem::execute`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteOutcome {
    /// Pasted into the target window.
    Pasted,
    /// Left on the clipboard; the UI tells the user to press `Ctrl+V` (PST-005).
    CopiedOnly(FallbackReason),
}

/// Puts an item on the clipboard, registers the write as our own and promotes the item.
fn place_on_clipboard(
    deps: &PasteDependencies,
    id: ClipId,
    mode: PasteMode,
) -> Result<(), AppError> {
    let item: ClipItem = deps.history.get(id)?.ok_or(PortError::NotFound)?;
    let content = match &item.content {
        ClipContent::Text(text) => ClipboardContent::Text(match mode {
            PasteMode::AsIs => text.as_str().to_owned(),
            PasteMode::TrimmedText => text.as_str().trim().to_owned(),
        }),
        ClipContent::Image(meta) => ClipboardContent::Image(deps.images.load(&meta.file_key)?),
    };
    let sequence = deps.writer.write(&content)?;
    deps.own_writes.record(sequence);
    deps.history.touch(id, deps.clock.now())?;
    deps.events.publish(AppEvent::HistoryChanged);
    Ok(())
}

/// Pastes an item into the window that was active before the popup (PST-001).
pub struct PasteItem {
    deps: PasteDependencies,
}

impl PasteItem {
    /// Builds the use case.
    pub const fn new(deps: PasteDependencies) -> Self {
        Self { deps }
    }

    /// Places the item on the clipboard, restores `target` and sends `Ctrl+V`. The popup must
    /// already be hidden. When pasting is impossible the item stays on the clipboard.
    ///
    /// # Errors
    /// [`PortError::NotFound`] for an unknown item, or clipboard/storage failures.
    pub fn execute(
        &self,
        id: ClipId,
        mode: PasteMode,
        target: Option<&TargetWindow>,
    ) -> Result<PasteOutcome, AppError> {
        place_on_clipboard(&self.deps, id, mode)?;
        let Some(target) = target else {
            return Ok(PasteOutcome::CopiedOnly(FallbackReason::NoTarget));
        };
        if target.elevated {
            return Ok(PasteOutcome::CopiedOnly(FallbackReason::TargetElevated));
        }
        let pasted = self
            .deps
            .focus
            .restore(target)
            .and_then(|()| self.deps.keys.send_paste());
        Ok(match pasted {
            Ok(()) => PasteOutcome::Pasted,
            Err(error) => {
                tracing::info!(%error, "paste fell back to clipboard only");
                PasteOutcome::CopiedOnly(FallbackReason::TargetUnavailable)
            }
        })
    }
}

/// Copies an item to the clipboard without pasting (HIST-011).
pub struct CopyItem {
    deps: PasteDependencies,
}

impl CopyItem {
    /// Builds the use case.
    pub const fn new(deps: PasteDependencies) -> Self {
        Self { deps }
    }

    /// Places the item on the clipboard and promotes it.
    ///
    /// # Errors
    /// [`PortError::NotFound`] for an unknown item, or clipboard/storage failures.
    pub fn execute(&self, id: ClipId) -> Result<(), AppError> {
        place_on_clipboard(&self.deps, id, PasteMode::AsIs)
    }
}
