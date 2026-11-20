use std::time::Duration;

use clipvault_domain::hotkey::{Hotkey, HotkeyOwner};
use clipvault_domain::ids::SnippetId;
use clipvault_domain::snippet::{NewSnippet, Snippet};
use clipvault_domain::time::Timestamp;

use super::PortError;

/// Whether global shortcuts are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suspension {
    /// Shortcuts trigger their actions.
    Active,
    /// Shortcuts are released while the user records a new one (HK-002).
    Suspended,
}

/// System-wide keyboard shortcuts (HK-001, SNP-003). Presses are delivered by the adapter to
/// the composition root, which runs the matching action.
pub trait HotkeyRegistrar: Send + Sync {
    /// Registers `hotkey` for `owner`, replacing the owner's previous one.
    ///
    /// # Errors
    /// [`PortError::InUse`] when another application holds the combination (HK-004).
    fn register(&self, owner: HotkeyOwner, hotkey: Hotkey) -> Result<(), PortError>;

    /// Releases the owner's shortcut; releasing an unregistered owner is not an error.
    ///
    /// # Errors
    /// [`PortError::Unavailable`] when the system call fails.
    fn unregister(&self, owner: HotkeyOwner) -> Result<(), PortError>;

    /// Temporarily releases (or restores) every shortcut without forgetting them.
    ///
    /// # Errors
    /// [`PortError::Unavailable`] when the system call fails.
    fn set_suspension(&self, suspension: Suspension) -> Result<(), PortError>;
}

/// Persistence of snippets (pattern: Repository — specs/04 §P-02).
///
/// Every implementation must pass `clipvault_test_support::snippet_repository_contract!`.
pub trait SnippetRepository: Send + Sync {
    /// All snippets in display order.
    ///
    /// # Errors
    /// [`PortError::Storage`] on failure.
    fn list(&self) -> Result<Vec<Snippet>, PortError>;

    /// The snippet with `id`, if any.
    ///
    /// # Errors
    /// [`PortError::Storage`] on failure.
    fn get(&self, id: SnippetId) -> Result<Option<Snippet>, PortError>;

    /// Stores a new snippet at the end of the list.
    ///
    /// # Errors
    /// [`PortError::InUse`] for a duplicate name or shortcut; [`PortError::Storage`].
    fn insert(&self, snippet: NewSnippet, at: Timestamp) -> Result<Snippet, PortError>;

    /// Replaces every field of an existing snippet.
    ///
    /// # Errors
    /// [`PortError::NotFound`], [`PortError::InUse`] or [`PortError::Storage`].
    fn update(&self, snippet: &Snippet, at: Timestamp) -> Result<(), PortError>;

    /// Deletes a snippet.
    ///
    /// # Errors
    /// [`PortError::NotFound`] or [`PortError::Storage`].
    fn delete(&self, id: SnippetId) -> Result<(), PortError>;
}

/// Waiting, injectable so that tests do not sleep (SNP-005 restore delay).
pub trait Delay: Send + Sync {
    /// Blocks the calling thread for `duration`.
    fn wait(&self, duration: Duration);
}
