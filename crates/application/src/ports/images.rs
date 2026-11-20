use clipvault_domain::image::{FileKey, ImagePixels};

use super::PortError;

/// Result of writing an image (original PNG + thumbnail).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredImage {
    /// Size of the PNG on disk, in bytes.
    pub file_size: u64,
    /// Thumbnail width in pixels.
    pub thumb_width: u32,
    /// Thumbnail height in pixels.
    pub thumb_height: u32,
}

/// Image files on disk (ADR-0004, specs/05 §4).
///
/// Every implementation must pass `clipvault_test_support::image_store_contract!`.
pub trait ImageStore: Send + Sync {
    /// Writes the image and its thumbnail atomically under `key` (REL-002).
    ///
    /// # Errors
    /// [`PortError::Storage`] on failure; no partial file is left behind.
    fn save(&self, key: &FileKey, pixels: &ImagePixels) -> Result<StoredImage, PortError>;

    /// Removes the image and its thumbnail; removing a missing key is not an error.
    ///
    /// # Errors
    /// [`PortError::Storage`] on failure.
    fn remove(&self, key: &FileKey) -> Result<(), PortError>;

    /// Decodes the original image (to paste it, PST-003).
    ///
    /// # Errors
    /// [`PortError::NotFound`] when no image is stored under `key`; [`PortError::Storage`]
    /// when it cannot be read or decoded.
    fn load(&self, key: &FileKey) -> Result<ImagePixels, PortError>;

    /// Keys of every stored original image (RET-005).
    ///
    /// # Errors
    /// [`PortError::Storage`] on failure.
    fn keys(&self) -> Result<Vec<FileKey>, PortError>;
}
