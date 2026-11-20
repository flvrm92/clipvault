use std::fmt;

use clipvault_domain::app_name::AppName;
use clipvault_domain::image::ImagePixels;

use super::PortError;

/// Windows clipboard sequence number (`GetClipboardSequenceNumber`); identifies one change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClipboardSequence(u32);

impl ClipboardSequence {
    /// Wraps a raw sequence number.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// The raw number.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Opt-out formats placed on the clipboard by password managers and Windows (CAP-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)] // one flag per Windows clipboard format
pub struct PrivacyFlags {
    /// `ExcludeClipboardContentFromMonitorProcessing` is present.
    pub exclude_from_monitor: bool,
    /// `Clipboard Viewer Ignore` is present.
    pub viewer_ignore: bool,
    /// `CanIncludeInClipboardHistory` is present with value `0`.
    pub history_disallowed: bool,
}

impl PrivacyFlags {
    /// Whether any opt-out is present.
    pub const fn any(self) -> bool {
        self.exclude_from_monitor || self.viewer_ignore || self.history_disallowed
    }
}

/// What the clipboard contained after a change, as read by the clipboard adapter.
///
/// `Debug` hides the text (PRIV-003).
#[derive(Clone, PartialEq, Eq)]
pub struct ClipboardSnapshot {
    /// Sequence number of the change.
    pub sequence: ClipboardSequence,
    /// Unicode text, if present.
    pub text: Option<String>,
    /// Decoded image (from `PNG`, `CF_DIBV5` or `CF_DIB`), if present.
    pub image: Option<ImagePixels>,
    /// Opt-out formats.
    pub privacy: PrivacyFlags,
    /// Executable that owns the clipboard, when identifiable.
    pub owner: Option<AppName>,
}

impl fmt::Debug for ClipboardSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipboardSnapshot")
            .field("sequence", &self.sequence)
            .field(
                "text",
                &self.text.as_ref().map(|t| format!("<{} bytes>", t.len())),
            )
            .field("image", &self.image)
            .field("privacy", &self.privacy)
            .field("owner", &self.owner)
            .finish()
    }
}

/// Content ClipVault places on the clipboard (to paste a history item or a snippet).
///
/// `Debug` hides the text (PRIV-003).
#[derive(Clone, PartialEq, Eq)]
pub enum ClipboardContent {
    /// Unicode text (`CF_UNICODETEXT`).
    Text(String),
    /// An image, written as `PNG` and `CF_DIB` for compatibility (PST-003).
    Image(ImagePixels),
}

impl fmt::Debug for ClipboardContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => write!(f, "Text(<{} bytes>)", text.len()),
            Self::Image(pixels) => f.debug_tuple("Image").field(pixels).finish(),
        }
    }
}

/// Reads the system clipboard.
pub trait ClipboardReader: Send + Sync {
    /// The current clipboard content. Retries while another process holds the clipboard
    /// (REL-003).
    ///
    /// # Errors
    /// [`PortError::Unavailable`] when the clipboard stays locked.
    fn snapshot(&self) -> Result<ClipboardSnapshot, PortError>;
}

/// Writes the system clipboard.
pub trait ClipboardWriter: Send + Sync {
    /// Replaces the clipboard content and returns the resulting sequence number, which the
    /// caller records in the [`crate::capture::OwnWriteRegistry`] (CAP-010).
    ///
    /// # Errors
    /// [`PortError::Unavailable`] when the clipboard stays locked.
    fn write(&self, content: &ClipboardContent) -> Result<ClipboardSequence, PortError>;
}
