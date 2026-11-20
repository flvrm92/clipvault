//! The capture filters, in pipeline order.

use std::sync::Arc;

use clipvault_domain::capture::{CaptureDecision, CapturedContent, IgnoreReason, NewClip};
use clipvault_domain::text::ClipText;

use super::{Candidate, CaptureContext, CaptureFilter, FilterOutcome, OwnWriteRegistry};
use crate::AppError;
use crate::ports::HistoryRepository;

#[allow(clippy::unnecessary_wraps)] // keeps filter bodies uniform
const fn ignore(reason: IgnoreReason) -> Result<FilterOutcome, AppError> {
    Ok(FilterOutcome::Stop(CaptureDecision::Ignore(reason)))
}

/// CAP-008: nothing is recorded while capture is paused.
#[derive(Debug, Default)]
pub struct PausedFilter;

impl CaptureFilter for PausedFilter {
    fn name(&self) -> &'static str {
        "paused"
    }

    fn apply(&self, ctx: CaptureContext) -> Result<FilterOutcome, AppError> {
        if ctx.settings.paused {
            return ignore(IgnoreReason::Paused);
        }
        Ok(FilterOutcome::Continue(ctx))
    }
}

/// CAP-010: changes written by ClipVault itself are not recorded.
#[derive(Debug)]
pub struct OwnWriteFilter {
    registry: Arc<OwnWriteRegistry>,
}

impl OwnWriteFilter {
    /// Uses the registry shared with the clipboard-writing use cases.
    pub const fn new(registry: Arc<OwnWriteRegistry>) -> Self {
        Self { registry }
    }
}

impl CaptureFilter for OwnWriteFilter {
    fn name(&self) -> &'static str {
        "own-write"
    }

    fn apply(&self, ctx: CaptureContext) -> Result<FilterOutcome, AppError> {
        if self.registry.take(ctx.snapshot.sequence) {
            return ignore(IgnoreReason::OwnWrite);
        }
        Ok(FilterOutcome::Continue(ctx))
    }
}

/// CAP-007: content marked as private by its source is not recorded.
#[derive(Debug, Default)]
pub struct PrivacyFlagsFilter;

impl CaptureFilter for PrivacyFlagsFilter {
    fn name(&self) -> &'static str {
        "privacy-flags"
    }

    fn apply(&self, ctx: CaptureContext) -> Result<FilterOutcome, AppError> {
        if ctx.snapshot.privacy.any() {
            return ignore(IgnoreReason::PrivacyFlag);
        }
        Ok(FilterOutcome::Continue(ctx))
    }
}

/// CAP-009: copies from excluded applications are not recorded; unknown owners are.
#[derive(Debug, Default)]
pub struct ExcludedAppFilter;

impl CaptureFilter for ExcludedAppFilter {
    fn name(&self) -> &'static str {
        "excluded-app"
    }

    fn apply(&self, ctx: CaptureContext) -> Result<FilterOutcome, AppError> {
        let excluded = ctx
            .snapshot
            .owner
            .as_ref()
            .is_some_and(|app| ctx.settings.is_excluded(app));
        if excluded {
            return ignore(IgnoreReason::ExcludedApp);
        }
        Ok(FilterOutcome::Continue(ctx))
    }
}

/// CAP-003: picks one content — non-blank text first, otherwise the image.
#[derive(Debug, Default)]
pub struct ContentSelector;

impl CaptureFilter for ContentSelector {
    fn name(&self) -> &'static str {
        "content-selector"
    }

    fn apply(&self, mut ctx: CaptureContext) -> Result<FilterOutcome, AppError> {
        let text = ctx.snapshot.text.take().filter(|t| !t.trim().is_empty());
        ctx.candidate = match text {
            Some(text) => Some(Candidate::Text(text)),
            None => ctx.snapshot.image.take().map(Candidate::Image),
        };
        Ok(FilterOutcome::Continue(ctx))
    }
}

/// CAP-005: changes without usable content are not recorded.
#[derive(Debug, Default)]
pub struct EmptyFilter;

impl CaptureFilter for EmptyFilter {
    fn name(&self) -> &'static str {
        "empty"
    }

    fn apply(&self, ctx: CaptureContext) -> Result<FilterOutcome, AppError> {
        if ctx.candidate.is_none() {
            return ignore(IgnoreReason::Empty);
        }
        Ok(FilterOutcome::Continue(ctx))
    }
}

/// CAP-006: content above the configured size limit is not recorded.
#[derive(Debug, Default)]
pub struct SizeLimitFilter;

impl CaptureFilter for SizeLimitFilter {
    fn name(&self) -> &'static str {
        "size-limit"
    }

    fn apply(&self, ctx: CaptureContext) -> Result<FilterOutcome, AppError> {
        let limit = match ctx.candidate {
            Some(Candidate::Text(_)) => ctx.settings.max_text_bytes,
            Some(Candidate::Image(_)) => ctx.settings.max_image_bytes,
            None => u64::MAX,
        };
        let size = ctx.candidate.as_ref().map_or(0, Candidate::byte_len) as u64;
        if size > limit {
            return ignore(IgnoreReason::TooLarge);
        }
        Ok(FilterOutcome::Continue(ctx))
    }
}

/// CAP-004: new content is stored; content already in the history is promoted.
pub struct Deduplicator {
    history: Arc<dyn HistoryRepository>,
}

impl Deduplicator {
    /// Looks up hashes in `history`.
    pub fn new(history: Arc<dyn HistoryRepository>) -> Self {
        Self { history }
    }
}

impl CaptureFilter for Deduplicator {
    fn name(&self) -> &'static str {
        "deduplicator"
    }

    fn apply(&self, ctx: CaptureContext) -> Result<FilterOutcome, AppError> {
        let content = match ctx.candidate {
            Some(Candidate::Text(text)) => match ClipText::new(&text) {
                Ok(text) => CapturedContent::Text(text),
                Err(_) => return ignore(IgnoreReason::Empty),
            },
            Some(Candidate::Image(pixels)) => CapturedContent::Image(pixels),
            None => return ignore(IgnoreReason::Empty),
        };
        let clip = NewClip::new(content, ctx.snapshot.owner);
        let decision = match self.history.find_by_hash(&clip.hash)? {
            Some(existing) => CaptureDecision::Promote(existing),
            None => CaptureDecision::Store(clip),
        };
        Ok(FilterOutcome::Stop(decision))
    }
}
