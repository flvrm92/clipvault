use std::sync::Arc;

use clipvault_domain::capture::{CaptureDecision, IgnoreReason};

use super::filters::{
    ContentSelector, Deduplicator, EmptyFilter, ExcludedAppFilter, OwnWriteFilter, PausedFilter,
    PrivacyFlagsFilter, SizeLimitFilter,
};
use super::{CaptureContext, CaptureFilter, FilterOutcome, OwnWriteRegistry};
use crate::AppError;
use crate::ports::HistoryRepository;

/// An ordered chain of [`CaptureFilter`]s.
pub struct CapturePipeline {
    filters: Vec<Box<dyn CaptureFilter>>,
}

impl CapturePipeline {
    /// A pipeline with custom filters (tests, future extensions).
    pub fn new(filters: Vec<Box<dyn CaptureFilter>>) -> Self {
        Self { filters }
    }

    /// The pipeline of specs/03 §3.2, in its specified order.
    pub fn standard(
        own_writes: Arc<OwnWriteRegistry>,
        history: Arc<dyn HistoryRepository>,
    ) -> Self {
        Self::new(vec![
            Box::new(PausedFilter),
            Box::new(OwnWriteFilter::new(own_writes)),
            Box::new(PrivacyFlagsFilter),
            Box::new(ExcludedAppFilter),
            Box::new(ContentSelector),
            Box::new(EmptyFilter),
            Box::new(SizeLimitFilter),
            Box::new(Deduplicator::new(history)),
        ])
    }

    /// Filter names in application order.
    pub fn filter_names(&self) -> Vec<&'static str> {
        self.filters.iter().map(|f| f.name()).collect()
    }

    /// Runs the filters until one decides. A chain that never decides ignores the change.
    ///
    /// # Errors
    /// The first port failure raised by a filter.
    pub fn evaluate(&self, mut ctx: CaptureContext) -> Result<CaptureDecision, AppError> {
        for filter in &self.filters {
            match filter.apply(ctx)? {
                FilterOutcome::Continue(next) => ctx = next,
                FilterOutcome::Stop(decision) => return Ok(decision),
            }
        }
        Ok(CaptureDecision::Ignore(IgnoreReason::Empty))
    }
}
