use clipvault_domain::clip::NewClipItem;
use clipvault_domain::hash::ContentHash;
use clipvault_domain::ids::ClipId;
use clipvault_domain::time::Timestamp;

use super::PortError;

/// Persistence of history items.
pub trait HistoryRepository: Send + Sync {
    /// The item with this content hash, if any (CAP-004).
    ///
    /// # Errors
    /// [`PortError::Storage`] on storage failure.
    fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<ClipId>, PortError>;

    /// Inserts a new item with `use_count = 1` and `last_used_at = captured_at`.
    ///
    /// # Errors
    /// [`PortError::Storage`] on storage failure.
    fn insert(&self, item: NewClipItem) -> Result<ClipId, PortError>;

    /// Marks an item as used now: updates `last_used_at` and increments `use_count`.
    ///
    /// # Errors
    /// [`PortError::NotFound`] or [`PortError::Storage`].
    fn touch(&self, id: ClipId, at: Timestamp) -> Result<(), PortError>;
}
