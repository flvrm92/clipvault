//! Captured plain text (`ClipText`) and its derived forms for listing and search.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Maximum number of lines shown in a history preview (HIST-001).
pub const PREVIEW_MAX_LINES: usize = 3;
/// Maximum number of characters of a preview, before the ellipsis (specs/05 `preview`).
pub const PREVIEW_MAX_CHARS: usize = 300;
const ELLIPSIS: char = '…';

/// Errors building a [`ClipText`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TextError {
    /// The text is empty or contains only whitespace (CAP-005).
    #[error("text is empty or whitespace only")]
    Blank,
}

/// Non-blank text captured from the clipboard, kept verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClipText(String);

impl ClipText {
    /// Validates that `text` is not blank.
    ///
    /// # Errors
    /// [`TextError::Blank`] when the text is empty or whitespace only.
    pub fn new(text: &str) -> Result<Self, TextError> {
        if text.trim().is_empty() {
            return Err(TextError::Blank);
        }
        Ok(Self(text.to_owned()))
    }

    /// The original text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Size in bytes (UTF-8).
    pub fn byte_len(&self) -> usize {
        self.0.len()
    }

    /// Up to [`PREVIEW_MAX_LINES`] non-blank lines with collapsed whitespace, cut to
    /// [`PREVIEW_MAX_CHARS`] characters; an ellipsis marks omitted content (HIST-001).
    pub fn preview(&self) -> String {
        let mut lines = self
            .0
            .lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|line| !line.is_empty());

        let mut preview = lines
            .by_ref()
            .take(PREVIEW_MAX_LINES)
            .collect::<Vec<_>>()
            .join("\n");
        let mut truncated = lines.next().is_some();
        if preview.chars().count() > PREVIEW_MAX_CHARS {
            preview = preview.chars().take(PREVIEW_MAX_CHARS).collect();
            truncated = true;
        }
        if truncated {
            preview.push(ELLIPSIS);
        }
        preview
    }

    /// Lowercase form without diacritics, used for accent-insensitive search (HIST-003).
    pub fn search_form(&self) -> String {
        normalize_for_search(&self.0)
    }
}

/// Lowercases `text` and strips diacritics (`"Café"` → `"cafe"`), for both stored text and queries.
pub fn normalize_for_search(text: &str) -> String {
    text.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests;
