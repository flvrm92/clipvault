use clipvault_domain::clip::{ClipItem, NewClipItem, PinState};
use clipvault_domain::hash::ContentHash;
use clipvault_domain::ids::ClipId;
use clipvault_domain::image::FileKey;
use clipvault_domain::retention::RetentionCandidate;
use clipvault_domain::time::Timestamp;

use super::PortError;

/// Which items a listing includes (HIST-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KindFilter {
    /// Every item.
    #[default]
    All,
    /// Text items only.
    Text,
    /// Image items only.
    Images,
    /// Pinned items only.
    Pinned,
}

/// A page request over the history (HIST-001..004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryQuery {
    /// Kind filter.
    pub filter: KindFilter,
    /// Text to search for, matched as a substring ignoring case and accents (HIST-003).
    /// Image items never match a search.
    pub search: Option<String>,
    /// Number of items to skip.
    pub offset: usize,
    /// Maximum number of items to return.
    pub limit: usize,
}

impl HistoryQuery {
    /// Page size used by the popup (HIST-002).
    pub const DEFAULT_PAGE_SIZE: usize = 50;
    /// Upper bound accepted for `limit`.
    pub const MAX_PAGE_SIZE: usize = 200;

    /// The first page of the whole history.
    pub const fn first_page() -> Self {
        Self {
            filter: KindFilter::All,
            search: None,
            offset: 0,
            limit: Self::DEFAULT_PAGE_SIZE,
        }
    }
}

impl Default for HistoryQuery {
    fn default() -> Self {
        Self::first_page()
    }
}

/// One page of results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// Items, ordered pinned first, then most recently used first.
    pub items: Vec<T>,
    /// Whether more items exist after this page.
    pub has_more: bool,
}

/// Result of a bulk deletion.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Removal {
    /// Number of items deleted.
    pub count: usize,
    /// File keys of the deleted image items, whose files must be removed.
    pub image_keys: Vec<FileKey>,
}

/// What [`HistoryRepository::clear`] removes (HIST-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearScope {
    /// Only unpinned items.
    UnpinnedOnly,
    /// Every item, including pinned ones.
    Everything,
}

/// Persistence of history items (pattern: Repository — specs/04 §P-02).
///
/// Every implementation must pass `clipvault_test_support::history_repository_contract!`.
pub trait HistoryRepository: Send + Sync {
    /// The item with this content hash, if any (CAP-004).
    ///
    /// # Errors
    /// [`PortError::Storage`] on storage failure.
    fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<ClipId>, PortError>;

    /// Inserts a new item with `use_count = 1`, unpinned, and `last_used_at = captured_at`.
    ///
    /// # Errors
    /// [`PortError::Storage`] on failure, including a duplicate content hash.
    fn insert(&self, item: NewClipItem) -> Result<ClipId, PortError>;

    /// The item with `id`, if any.
    ///
    /// # Errors
    /// [`PortError::Storage`] on storage failure.
    fn get(&self, id: ClipId) -> Result<Option<ClipItem>, PortError>;

    /// A page of items: pinned first, then by `last_used_at` descending (HIST-001).
    ///
    /// # Errors
    /// [`PortError::Storage`] on storage failure.
    fn list(&self, query: &HistoryQuery) -> Result<Page<ClipItem>, PortError>;

    /// Marks an item as used now: updates `last_used_at` and increments `use_count`.
    ///
    /// # Errors
    /// [`PortError::NotFound`] or [`PortError::Storage`].
    fn touch(&self, id: ClipId, at: Timestamp) -> Result<(), PortError>;

    /// Pins or unpins an item (HIST-005).
    ///
    /// # Errors
    /// [`PortError::NotFound`] or [`PortError::Storage`].
    fn set_pin_state(&self, id: ClipId, state: PinState) -> Result<(), PortError>;

    /// Deletes an item; returns its image file key, if it was an image (HIST-006).
    ///
    /// # Errors
    /// [`PortError::NotFound`] or [`PortError::Storage`].
    fn delete(&self, id: ClipId) -> Result<Option<FileKey>, PortError>;

    /// Deletes items in bulk; unknown ids are ignored.
    ///
    /// # Errors
    /// [`PortError::Storage`] on storage failure (nothing is deleted).
    fn delete_many(&self, ids: &[ClipId]) -> Result<Removal, PortError>;

    /// Deletes items according to `scope` (HIST-007).
    ///
    /// # Errors
    /// [`PortError::Storage`] on storage failure.
    fn clear(&self, scope: ClearScope) -> Result<Removal, PortError>;

    /// The retention view of every item (RET-001..003).
    ///
    /// # Errors
    /// [`PortError::Storage`] on storage failure.
    fn retention_candidates(&self) -> Result<Vec<RetentionCandidate>, PortError>;

    /// Ids and file keys of every image item (RET-005).
    ///
    /// # Errors
    /// [`PortError::Storage`] on storage failure.
    fn image_entries(&self) -> Result<Vec<(ClipId, FileKey)>, PortError>;
}
