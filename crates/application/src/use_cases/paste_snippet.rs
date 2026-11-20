//! Pasting snippets (SNP-003..005, SNP-007).

use std::sync::Arc;

use clipvault_domain::ids::SnippetId;

use super::paste::{FallbackReason, PasteOutcome};
use crate::AppError;
use crate::capture::OwnWriteRegistry;
use crate::ports::{
    ClipboardContent, ClipboardReader, ClipboardSnapshot, ClipboardWriter, Delay, FocusTracker,
    KeystrokeSender, PortError, SettingsRepository, SnippetRepository, TargetWindow,
};

/// Ports required by [`PasteSnippet`].
pub struct PasteSnippetDependencies {
    /// Snippet persistence.
    pub snippets: Arc<dyn SnippetRepository>,
    /// Reads the content to restore (SNP-005).
    pub reader: Arc<dyn ClipboardReader>,
    /// System clipboard.
    pub writer: Arc<dyn ClipboardWriter>,
    /// Foreground window management.
    pub focus: Arc<dyn FocusTracker>,
    /// Synthetic keystrokes.
    pub keys: Arc<dyn KeystrokeSender>,
    /// Clipboard changes made by ClipVault: snippets never enter the history (SNP-003).
    pub own_writes: Arc<OwnWriteRegistry>,
    /// User settings (restore option and delay).
    pub settings: Arc<dyn SettingsRepository>,
    /// Waiting before the restore.
    pub delay: Arc<dyn Delay>,
}

/// Where the snippet goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnippetTarget {
    /// The window in the foreground: the snippet's global shortcut was pressed in it (SNP-003).
    Foreground,
    /// The window that was active before the popup (SNP-007).
    Window(Option<TargetWindow>),
}

fn restorable(snapshot: ClipboardSnapshot) -> Option<ClipboardContent> {
    snapshot
        .text
        .map(ClipboardContent::Text)
        .or_else(|| snapshot.image.map(ClipboardContent::Image))
}

/// Pastes a snippet and then restores the previous clipboard content.
pub struct PasteSnippet {
    deps: PasteSnippetDependencies,
}

impl PasteSnippet {
    /// Builds the use case.
    pub const fn new(deps: PasteSnippetDependencies) -> Self {
        Self { deps }
    }

    fn write(&self, content: &ClipboardContent) -> Result<(), PortError> {
        let sequence = self.deps.writer.write(content)?;
        self.deps.own_writes.record(sequence);
        Ok(())
    }

    fn deliver(&self, target: SnippetTarget) -> PasteOutcome {
        let result = match target {
            SnippetTarget::Foreground => {
                if self.deps.focus.capture_target().is_some_and(|t| t.elevated) {
                    return PasteOutcome::CopiedOnly(FallbackReason::TargetElevated);
                }
                self.deps.keys.send_paste()
            }
            SnippetTarget::Window(None) => {
                return PasteOutcome::CopiedOnly(FallbackReason::NoTarget);
            }
            SnippetTarget::Window(Some(window)) if window.elevated => {
                return PasteOutcome::CopiedOnly(FallbackReason::TargetElevated);
            }
            SnippetTarget::Window(Some(window)) => self
                .deps
                .focus
                .restore(&window)
                .and_then(|()| self.deps.keys.send_paste()),
        };
        match result {
            Ok(()) => PasteOutcome::Pasted,
            Err(error) => {
                tracing::info!(%error, "snippet left on the clipboard");
                PasteOutcome::CopiedOnly(FallbackReason::TargetUnavailable)
            }
        }
    }

    /// Places the snippet on the clipboard, pastes it and, when enabled, restores the previous
    /// text or image after the configured delay (SNP-005). When pasting is impossible the
    /// snippet stays on the clipboard for a manual paste and nothing is restored.
    ///
    /// # Errors
    /// [`PortError::NotFound`] for an unknown snippet, or clipboard/settings failures.
    pub fn execute(&self, id: SnippetId, target: SnippetTarget) -> Result<PasteOutcome, AppError> {
        let snippet = self.deps.snippets.get(id)?.ok_or(PortError::NotFound)?;
        let settings = self.deps.settings.load()?.snippets;
        let previous = if settings.restore_clipboard {
            self.deps.reader.snapshot().ok().and_then(restorable)
        } else {
            None
        };

        self.write(&ClipboardContent::Text(snippet.body.as_str().to_owned()))?;
        let outcome = self.deliver(target);

        if outcome == PasteOutcome::Pasted
            && let Some(previous) = previous
        {
            self.deps.delay.wait(settings.restore_delay);
            self.write(&previous)?;
        }
        Ok(outcome)
    }
}
