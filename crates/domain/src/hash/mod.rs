//! Content hashes used for deduplication (CAP-004) and image file names (specs/05 §2).

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use crate::image::ImagePixels;

/// Errors parsing a [`ContentHash`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HashError {
    /// Not 64 hexadecimal characters.
    #[error("invalid content hash hex")]
    InvalidHex,
}

/// SHA-256 of a clip's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Hash of the UTF-8 bytes, without normalization: texts differing in whitespace differ.
    pub fn of_text(text: &str) -> Self {
        Self(Sha256::digest(text.as_bytes()).into())
    }

    /// Hash of the decoded RGBA pixels plus dimensions, so the same image yields the same hash
    /// regardless of the clipboard format it came from.
    pub fn of_image(pixels: &ImagePixels) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(pixels.width().to_le_bytes());
        hasher.update(pixels.height().to_le_bytes());
        hasher.update(pixels.rgba());
        Self(hasher.finalize().into())
    }

    /// Wraps raw hash bytes (e.g. read from the database).
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Raw hash bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hexadecimal representation (64 characters).
    pub fn to_hex(&self) -> String {
        self.0
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            })
    }

    /// Parses a 64-character hexadecimal hash.
    ///
    /// # Errors
    /// [`HashError::InvalidHex`] for any other input.
    pub fn from_hex(hex: &str) -> Result<Self, HashError> {
        if hex.len() != 64 || !hex.is_ascii() {
            return Err(HashError::InvalidHex);
        }
        let mut bytes = [0_u8; 32];
        let (pairs, _) = hex.as_bytes().as_chunks::<2>();
        for (byte, pair) in bytes.iter_mut().zip(pairs) {
            let pair = std::str::from_utf8(pair).map_err(|_| HashError::InvalidHex)?;
            *byte = u8::from_str_radix(pair, 16).map_err(|_| HashError::InvalidHex)?;
        }
        Ok(Self(bytes))
    }
}

#[cfg(test)]
mod tests;
