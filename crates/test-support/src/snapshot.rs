//! Builder for [`ClipboardSnapshot`]s (pattern: Test Data Builder — specs/04 §P-10).
//!
//! ```ignore
//! let snap = snapshot::text("abc").owner("code.exe").sequence(7).build();
//! ```

use clipvault_application::ports::{ClipboardSequence, ClipboardSnapshot, PrivacyFlags};
use clipvault_domain::app_name::AppName;
use clipvault_domain::image::ImagePixels;

/// A clipboard change containing `text`.
pub fn text(text: &str) -> SnapshotBuilder {
    SnapshotBuilder {
        text: Some(text.to_owned()),
        ..SnapshotBuilder::default()
    }
}

/// A clipboard change with no content.
pub fn empty() -> SnapshotBuilder {
    SnapshotBuilder::default()
}

/// A clipboard sequence number.
pub const fn sequence(raw: u32) -> ClipboardSequence {
    ClipboardSequence::new(raw)
}

/// Deterministic pixels: the same dimensions always produce the same image.
///
/// # Panics
/// If the dimensions are zero.
pub fn pixels(width: u32, height: u32) -> ImagePixels {
    let len = width as usize * height as usize * 4;
    let rgba = (0..len)
        .map(|i| u8::try_from(i % 251).unwrap_or_default())
        .collect();
    ImagePixels::new(width, height, rgba).unwrap_or_else(|e| panic!("invalid test image: {e}"))
}

/// Fluent builder for snapshots.
#[derive(Debug, Clone, Default)]
pub struct SnapshotBuilder {
    sequence: u32,
    text: Option<String>,
    image: Option<(u32, u32)>,
    privacy: PrivacyFlags,
    owner: Option<String>,
}

impl SnapshotBuilder {
    /// Adds an image of the given size (see [`pixels`]).
    #[must_use]
    pub const fn image(mut self, width: u32, height: u32) -> Self {
        self.image = Some((width, height));
        self
    }

    /// Sets the clipboard owner executable.
    #[must_use]
    pub fn owner(mut self, executable: &str) -> Self {
        self.owner = Some(executable.to_owned());
        self
    }

    /// Sets the sequence number.
    #[must_use]
    pub const fn sequence(mut self, raw: u32) -> Self {
        self.sequence = raw;
        self
    }

    /// Adds `ExcludeClipboardContentFromMonitorProcessing`.
    #[must_use]
    pub const fn exclude_from_monitor(mut self) -> Self {
        self.privacy.exclude_from_monitor = true;
        self
    }

    /// Adds `Clipboard Viewer Ignore`.
    #[must_use]
    pub const fn viewer_ignore(mut self) -> Self {
        self.privacy.viewer_ignore = true;
        self
    }

    /// Adds `CanIncludeInClipboardHistory = 0`.
    #[must_use]
    pub const fn history_disallowed(mut self) -> Self {
        self.privacy.history_disallowed = true;
        self
    }

    /// Builds the snapshot.
    ///
    /// # Panics
    /// If the owner name is blank.
    pub fn build(self) -> ClipboardSnapshot {
        ClipboardSnapshot {
            sequence: ClipboardSequence::new(self.sequence),
            text: self.text,
            image: self.image.map(|(w, h)| pixels(w, h)),
            privacy: self.privacy,
            owner: self.owner.map(|o| {
                AppName::parse(&o).unwrap_or_else(|e| panic!("invalid test owner {o:?}: {e}"))
            }),
        }
    }
}
