//! Snippets: user-defined texts pasted through global shortcuts (SNP-001..002).

use crate::hotkey::Hotkey;
use crate::ids::SnippetId;

/// Maximum snippet name length, in characters.
pub const SNIPPET_NAME_MAX_CHARS: usize = 60;
/// Maximum snippet body size, in bytes (100 KiB).
pub const SNIPPET_BODY_MAX_BYTES: usize = 100 * 1024;

/// Validation errors of snippet fields.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SnippetError {
    /// The name is empty after trimming.
    #[error("snippet name is required")]
    NameBlank,
    /// The name exceeds the limit.
    #[error("snippet name exceeds {max} characters")]
    NameTooLong {
        /// The limit.
        max: usize,
    },
    /// The body is empty or whitespace only.
    #[error("snippet content is required")]
    BodyBlank,
    /// The body exceeds the limit.
    #[error("snippet content exceeds {max} bytes")]
    BodyTooLarge {
        /// The limit.
        max: usize,
    },
}

/// A trimmed name of 1 to [`SNIPPET_NAME_MAX_CHARS`] characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SnippetName(String);

impl SnippetName {
    /// Trims and validates a name.
    ///
    /// # Errors
    /// [`SnippetError::NameBlank`] or [`SnippetError::NameTooLong`].
    pub fn parse(value: &str) -> Result<Self, SnippetError> {
        let name = value.trim();
        if name.is_empty() {
            return Err(SnippetError::NameBlank);
        }
        if name.chars().count() > SNIPPET_NAME_MAX_CHARS {
            return Err(SnippetError::NameTooLong {
                max: SNIPPET_NAME_MAX_CHARS,
            });
        }
        Ok(Self(name.to_owned()))
    }

    /// The name.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Names must be unique ignoring case.
    pub fn is_same_as(&self, other: &Self) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

/// Non-blank snippet text up to [`SNIPPET_BODY_MAX_BYTES`], kept verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SnippetBody(String);

impl SnippetBody {
    /// Validates a body.
    ///
    /// # Errors
    /// [`SnippetError::BodyBlank`] or [`SnippetError::BodyTooLarge`].
    pub fn new(value: &str) -> Result<Self, SnippetError> {
        if value.trim().is_empty() {
            return Err(SnippetError::BodyBlank);
        }
        if value.len() > SNIPPET_BODY_MAX_BYTES {
            return Err(SnippetError::BodyTooLarge {
                max: SNIPPET_BODY_MAX_BYTES,
            });
        }
        Ok(Self(value.to_owned()))
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A snippet not yet persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSnippet {
    /// Display name.
    pub name: SnippetName,
    /// Text to paste.
    pub body: SnippetBody,
    /// Optional global shortcut; without one the snippet is only reachable from the popup.
    pub hotkey: Option<Hotkey>,
    /// Disabled snippets keep their data but do not register their shortcut.
    pub enabled: bool,
}

/// A persisted snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    /// Identifier.
    pub id: SnippetId,
    /// Display name.
    pub name: SnippetName,
    /// Text to paste.
    pub body: SnippetBody,
    /// Optional global shortcut.
    pub hotkey: Option<Hotkey>,
    /// Whether the shortcut is active.
    pub enabled: bool,
    /// Position in the settings list.
    pub order: u32,
}

#[cfg(test)]
mod tests;
