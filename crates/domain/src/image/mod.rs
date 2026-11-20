//! Images: decoded pixels (as captured), stored-image metadata and file keys (ADR-0004).

use crate::hash::{ContentHash, HashError};

/// Errors building image values.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImageError {
    /// Width or height is zero.
    #[error("image dimensions must be greater than zero")]
    ZeroDimension,
    /// The RGBA buffer length does not equal `width * height * 4`.
    #[error("RGBA buffer has {actual} bytes, expected {expected}")]
    BufferSizeMismatch {
        /// Expected length.
        expected: usize,
        /// Actual length.
        actual: usize,
    },
    /// A file key is not a valid content hash.
    #[error("invalid image file key")]
    InvalidFileKey,
}

impl From<HashError> for ImageError {
    fn from(_: HashError) -> Self {
        Self::InvalidFileKey
    }
}

/// A decoded image in 8-bit RGBA, as read from the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePixels {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl ImagePixels {
    /// Validates the dimensions against the buffer.
    ///
    /// # Errors
    /// [`ImageError::ZeroDimension`] or [`ImageError::BufferSizeMismatch`].
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, ImageError> {
        if width == 0 || height == 0 {
            return Err(ImageError::ZeroDimension);
        }
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(ImageError::BufferSizeMismatch {
                expected: usize::MAX,
                actual: rgba.len(),
            })?;
        if rgba.len() != expected {
            return Err(ImageError::BufferSizeMismatch {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    /// Width in pixels.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Uncompressed size in bytes, compared against the image size limit (CAP-006).
    pub fn byte_len(&self) -> usize {
        self.rgba.len()
    }

    /// Raw RGBA bytes, row-major.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

/// Identifier of an image file on disk: the hex content hash. Never a path (SEC-003).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileKey(String);

impl FileKey {
    /// The key for content with the given hash.
    pub fn from_hash(hash: &ContentHash) -> Self {
        Self(hash.to_hex())
    }

    /// Parses a stored key, accepting only valid content hashes.
    ///
    /// # Errors
    /// [`ImageError::InvalidFileKey`] for anything else (prevents path traversal).
    pub fn parse(key: &str) -> Result<Self, ImageError> {
        Ok(Self::from_hash(&ContentHash::from_hex(key)?))
    }

    /// The key as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Sub-directory name: the first two characters (specs/05 §4).
    pub fn shard(&self) -> &str {
        &self.0[..2]
    }
}

/// Metadata of an image stored on disk (`image_assets`, specs/05 §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMeta {
    /// File identifier.
    pub file_key: FileKey,
    /// Original width in pixels.
    pub width: u32,
    /// Original height in pixels.
    pub height: u32,
    /// Size of the stored PNG in bytes.
    pub file_size: u64,
    /// Thumbnail width in pixels.
    pub thumb_width: u32,
    /// Thumbnail height in pixels.
    pub thumb_height: u32,
}

#[cfg(test)]
mod tests;
