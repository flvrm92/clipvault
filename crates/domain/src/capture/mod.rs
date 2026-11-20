//! Outcome of evaluating a clipboard change (specs/03 §3.2, CAP-*).

use crate::app_name::AppName;
use crate::hash::ContentHash;
use crate::ids::ClipId;
use crate::image::ImagePixels;
use crate::text::ClipText;

/// Content selected from the clipboard, before anything is persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapturedContent {
    /// Non-blank text.
    Text(ClipText),
    /// Decoded image.
    Image(ImagePixels),
}

impl CapturedContent {
    /// UTF-8 size of text, or uncompressed RGBA size of an image.
    pub fn byte_len(&self) -> usize {
        match self {
            Self::Text(text) => text.byte_len(),
            Self::Image(pixels) => pixels.byte_len(),
        }
    }

    fn hash(&self) -> ContentHash {
        match self {
            Self::Text(text) => ContentHash::of_text(text.as_str()),
            Self::Image(pixels) => ContentHash::of_image(pixels),
        }
    }
}

/// Content that is new to the history and must be stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewClip {
    /// The content.
    pub content: CapturedContent,
    /// Its deduplication hash.
    pub hash: ContentHash,
    /// Source executable, when known.
    pub source_app: Option<AppName>,
}

impl NewClip {
    /// Builds a new clip, computing its hash.
    pub fn new(content: CapturedContent, source_app: Option<AppName>) -> Self {
        let hash = content.hash();
        Self {
            content,
            hash,
            source_app,
        }
    }
}

/// Why a clipboard change was not recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IgnoreReason {
    /// Capture is paused (CAP-008).
    Paused,
    /// The change was written by ClipVault itself (CAP-010).
    OwnWrite,
    /// The source asked not to be monitored (CAP-007).
    PrivacyFlag,
    /// The source application is excluded (CAP-009).
    ExcludedApp,
    /// No usable text or image (CAP-003, CAP-005).
    Empty,
    /// Above the configured size limit (CAP-006).
    TooLarge,
}

/// Decision for one clipboard change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureDecision {
    /// Store as a new history item.
    Store(NewClip),
    /// Same content already exists: move it to the top (CAP-004).
    Promote(ClipId),
    /// Do not record.
    Ignore(IgnoreReason),
}

#[cfg(test)]
mod tests;
