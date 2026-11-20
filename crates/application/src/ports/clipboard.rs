use clipvault_domain::app_name::AppName;
use clipvault_domain::image::ImagePixels;

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
