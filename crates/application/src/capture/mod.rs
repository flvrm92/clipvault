//! Clipboard capture pipeline (specs/03 §3.2).
//!
//! Pattern: Chain of Responsibility — specs/04 §P-05.

mod debounce;
mod filters;
mod own_writes;
mod pipeline;

pub use debounce::Debouncer;
pub use filters::{
    ContentSelector, Deduplicator, EmptyFilter, ExcludedAppFilter, OwnWriteFilter, PausedFilter,
    PrivacyFlagsFilter, SizeLimitFilter,
};
pub use own_writes::OwnWriteRegistry;
pub use pipeline::CapturePipeline;

use clipvault_domain::capture::CaptureDecision;
use clipvault_domain::image::ImagePixels;
use clipvault_domain::settings::CaptureSettings;

use crate::AppError;
use crate::ports::ClipboardSnapshot;

/// Content chosen by [`ContentSelector`], not yet validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Candidate {
    /// Text from the clipboard.
    Text(String),
    /// Image from the clipboard.
    Image(ImagePixels),
}

impl Candidate {
    /// Size compared against the limits (CAP-006).
    pub fn byte_len(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Image(pixels) => pixels.byte_len(),
        }
    }
}

/// State passed along the filter chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureContext {
    /// The clipboard change being evaluated.
    pub snapshot: ClipboardSnapshot,
    /// Capture settings at the time of the change.
    pub settings: CaptureSettings,
    /// Content selected so far.
    pub candidate: Option<Candidate>,
}

impl CaptureContext {
    /// A context with no candidate selected yet.
    pub const fn new(snapshot: ClipboardSnapshot, settings: CaptureSettings) -> Self {
        Self {
            snapshot,
            settings,
            candidate: None,
        }
    }
}

/// Result of one filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOutcome {
    /// Pass the (possibly updated) context to the next filter.
    Continue(CaptureContext),
    /// Final decision; later filters are skipped.
    Stop(CaptureDecision),
}

/// One rule of the capture pipeline.
pub trait CaptureFilter: Send + Sync {
    /// Stable name, used to verify the chain order.
    fn name(&self) -> &'static str;

    /// Applies the rule.
    ///
    /// # Errors
    /// Port failures (e.g. the history lookup).
    fn apply(&self, ctx: CaptureContext) -> Result<FilterOutcome, AppError>;
}
