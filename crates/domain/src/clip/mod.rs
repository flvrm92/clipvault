//! History items (`ClipItem`) and their content.

use crate::app_name::AppName;
use crate::hash::ContentHash;
use crate::ids::ClipId;
use crate::image::ImageMeta;
use crate::retention::RetentionCandidate;
use crate::text::ClipText;
use crate::time::Timestamp;

/// Kind of a history item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipKind {
    /// Plain text.
    Text,
    /// Image stored on disk.
    Image,
}

/// Whether an item is pinned (HIST-005). Used instead of a boolean parameter (specs/04).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PinState {
    /// Kept on top and never expires.
    Pinned,
    /// Regular item.
    Unpinned,
}

impl PinState {
    /// From the stored flag.
    pub const fn from_flag(pinned: bool) -> Self {
        if pinned { Self::Pinned } else { Self::Unpinned }
    }

    /// Whether this is [`Self::Pinned`].
    pub const fn is_pinned(self) -> bool {
        matches!(self, Self::Pinned)
    }

    /// The opposite state.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Pinned => Self::Unpinned,
            Self::Unpinned => Self::Pinned,
        }
    }
}

/// Content of a persisted history item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipContent {
    /// Plain text.
    Text(ClipText),
    /// Image metadata; the pixels live on disk (ADR-0004).
    Image(ImageMeta),
}

impl ClipContent {
    /// The content kind.
    pub const fn kind(&self) -> ClipKind {
        match self {
            Self::Text(_) => ClipKind::Text,
            Self::Image(_) => ClipKind::Image,
        }
    }

    /// Text size in UTF-8 bytes, or stored PNG size (`clip_items.byte_size`).
    pub fn byte_size(&self) -> u64 {
        match self {
            Self::Text(text) => text.byte_len() as u64,
            Self::Image(meta) => meta.file_size,
        }
    }
}

/// A history item about to be inserted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewClipItem {
    /// Content (image already written to disk).
    pub content: ClipContent,
    /// Deduplication hash.
    pub hash: ContentHash,
    /// Source executable, when known (CAP-011).
    pub source_app: Option<AppName>,
    /// Capture time; also the initial `last_used_at`.
    pub captured_at: Timestamp,
}

/// A persisted history item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipItem {
    /// Identifier.
    pub id: ClipId,
    /// Content.
    pub content: ClipContent,
    /// Deduplication hash.
    pub hash: ContentHash,
    /// Source executable, when known.
    pub source_app: Option<AppName>,
    /// Pinned items stay on top and never expire.
    pub pinned: bool,
    /// Number of captures/pastes.
    pub use_count: u32,
    /// First capture.
    pub created_at: Timestamp,
    /// Last capture or paste; drives ordering and retention.
    pub last_used_at: Timestamp,
}

impl ClipItem {
    /// The fields relevant to retention.
    pub const fn retention_candidate(&self) -> RetentionCandidate {
        RetentionCandidate {
            id: self.id,
            last_used_at: self.last_used_at,
            pinned: self.pinned,
        }
    }
}

#[cfg(test)]
mod tests;
