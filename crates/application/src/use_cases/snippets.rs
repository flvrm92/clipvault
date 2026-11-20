//! Snippets and global shortcuts (SNP-001..006, HK-001..004).

use std::sync::Arc;

use clipvault_domain::hotkey::{Hotkey, HotkeyOwner, find_conflict};
use clipvault_domain::ids::SnippetId;
use clipvault_domain::snippet::{NewSnippet, Snippet, SnippetBody, SnippetName};

use crate::error::{AppError, HotkeyConflict};
use crate::ports::{
    Clock, HotkeyRegistrar, PortError, SettingsRepository, SnippetRepository, Suspension,
};

/// Ports required by the snippet and shortcut use cases.
pub struct SnippetDependencies {
    /// Snippet persistence.
    pub snippets: Arc<dyn SnippetRepository>,
    /// User settings (open shortcut).
    pub settings: Arc<dyn SettingsRepository>,
    /// Global shortcuts.
    pub registrar: Arc<dyn HotkeyRegistrar>,
    /// Current time.
    pub clock: Arc<dyn Clock>,
}

/// Snippet fields as typed by the user; validated by [`SaveSnippet`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetDraft {
    /// `None` creates a snippet; `Some` edits it.
    pub id: Option<SnippetId>,
    /// Display name.
    pub name: String,
    /// Text to paste.
    pub body: String,
    /// Shortcut text such as `"Ctrl+Shift+1"`; `None` or blank for no shortcut.
    pub hotkey: Option<String>,
    /// Whether the shortcut is active.
    pub enabled: bool,
}

/// Owner placeholder for a snippet that has no id yet: matches no existing binding.
const UNSAVED: HotkeyOwner = HotkeyOwner::Snippet(SnippetId::new(i64::MIN));

/// Every ClipVault shortcut except `editing`'s, labelled for conflict messages.
fn conflict_with(
    deps: &SnippetDependencies,
    candidate: Hotkey,
    editing: HotkeyOwner,
) -> Result<Option<HotkeyConflict>, AppError> {
    let open = deps.settings.load()?.hotkeys.open;
    let snippets = deps.snippets.list()?;
    let bindings = std::iter::once((HotkeyOwner::OpenHistory, open)).chain(
        snippets
            .iter()
            .filter_map(|s| s.hotkey.map(|h| (HotkeyOwner::Snippet(s.id), h))),
    );
    let owners: Vec<(HotkeyOwner, Hotkey)> = bindings.collect();
    let conflict = find_conflict(&candidate, owners.iter().map(|(o, h)| (*o, h)), editing);
    Ok(conflict.map(|owner| match owner {
        HotkeyOwner::OpenHistory => HotkeyConflict::OpenHistory,
        HotkeyOwner::Snippet(id) => HotkeyConflict::Snippet(
            snippets
                .iter()
                .find(|s| s.id == id)
                .map_or_else(String::new, |s| s.name.as_str().to_owned()),
        ),
    }))
}

/// Registers the shortcut a snippet should have (enabled with a hotkey) or releases it.
fn sync_registration(registrar: &dyn HotkeyRegistrar, snippet: &Snippet) -> Result<(), PortError> {
    let owner = HotkeyOwner::Snippet(snippet.id);
    match snippet.hotkey.filter(|_| snippet.enabled) {
        Some(hotkey) => registrar.register(owner, hotkey),
        None => registrar.unregister(owner),
    }
}

/// Lists snippets in display order.
pub struct ListSnippets {
    snippets: Arc<dyn SnippetRepository>,
}

impl ListSnippets {
    /// Builds the use case.
    pub fn new(snippets: Arc<dyn SnippetRepository>) -> Self {
        Self { snippets }
    }

    /// All snippets.
    ///
    /// # Errors
    /// Storage failures.
    pub fn execute(&self) -> Result<Vec<Snippet>, AppError> {
        Ok(self.snippets.list()?)
    }
}

/// Creates or edits a snippet and keeps its global shortcut registered (SNP-001, SNP-006).
pub struct SaveSnippet {
    deps: SnippetDependencies,
}

impl SaveSnippet {
    /// Builds the use case.
    pub const fn new(deps: SnippetDependencies) -> Self {
        Self { deps }
    }

    /// Validates, persists and registers. When the shortcut is taken by another application,
    /// nothing changes and [`PortError::InUse`] is returned (HK-004).
    ///
    /// # Errors
    /// Validation errors, [`AppError::NameTaken`], [`AppError::HotkeyTaken`] or port failures.
    pub fn execute(&self, draft: SnippetDraft) -> Result<Snippet, AppError> {
        let SnippetDraft {
            id,
            name,
            body,
            hotkey,
            enabled,
        } = draft;
        let name = SnippetName::parse(&name)?;
        let body = SnippetBody::new(&body)?;
        let hotkey = hotkey
            .as_deref()
            .map(str::trim)
            .filter(|h| !h.is_empty())
            .map(Hotkey::parse)
            .transpose()?;

        let previous = match id {
            Some(id) => Some(self.deps.snippets.get(id)?.ok_or(PortError::NotFound)?),
            None => None,
        };
        let editing = previous
            .as_ref()
            .map_or(UNSAVED, |p| HotkeyOwner::Snippet(p.id));
        let name_taken = self
            .deps
            .snippets
            .list()?
            .iter()
            .any(|s| Some(s.id) != id && s.name.is_same_as(&name));
        if name_taken {
            return Err(AppError::NameTaken);
        }
        if let Some(hotkey) = hotkey
            && let Some(conflict) = conflict_with(&self.deps, hotkey, editing)?
        {
            return Err(AppError::HotkeyTaken(conflict));
        }

        let now = self.deps.clock.now();
        let saved = match previous {
            None => self.deps.snippets.insert(
                NewSnippet {
                    name,
                    body,
                    hotkey,
                    enabled,
                },
                now,
            )?,
            Some(ref old) => {
                let updated = Snippet {
                    name,
                    body,
                    hotkey,
                    enabled,
                    ..old.clone()
                };
                self.deps.snippets.update(&updated, now)?;
                updated
            }
        };

        if let Err(error) = sync_registration(self.deps.registrar.as_ref(), &saved) {
            // HK-004: undo the change and keep the previous shortcut.
            match &previous {
                None => {
                    let _ = self.deps.snippets.delete(saved.id);
                }
                Some(old) => {
                    let _ = self.deps.snippets.update(old, now);
                    let _ = sync_registration(self.deps.registrar.as_ref(), old);
                }
            }
            return Err(error.into());
        }
        Ok(saved)
    }
}

/// Deletes a snippet and releases its shortcut (SNP-006).
pub struct DeleteSnippet {
    deps: SnippetDependencies,
}

impl DeleteSnippet {
    /// Builds the use case.
    pub const fn new(deps: SnippetDependencies) -> Self {
        Self { deps }
    }

    /// Deletes the snippet.
    ///
    /// # Errors
    /// [`PortError::NotFound`] or storage failures.
    pub fn execute(&self, id: SnippetId) -> Result<(), AppError> {
        self.deps.snippets.delete(id)?;
        self.deps.registrar.unregister(HotkeyOwner::Snippet(id))?;
        Ok(())
    }
}

/// Changes the shortcut that opens the popup (HK-001..004).
pub struct ChangeOpenHotkey {
    deps: SnippetDependencies,
}

impl ChangeOpenHotkey {
    /// Builds the use case.
    pub const fn new(deps: SnippetDependencies) -> Self {
        Self { deps }
    }

    /// Validates, registers and persists the new shortcut; the previous one is kept on failure.
    ///
    /// # Errors
    /// [`AppError::InvalidHotkey`], [`AppError::HotkeyTaken`], [`PortError::InUse`] or storage
    /// failures.
    pub fn execute(&self, text: &str) -> Result<Hotkey, AppError> {
        let hotkey = Hotkey::parse(text)?;
        if let Some(conflict) = conflict_with(&self.deps, hotkey, HotkeyOwner::OpenHistory)? {
            return Err(AppError::HotkeyTaken(conflict));
        }
        let mut settings = self.deps.settings.load()?;
        let previous = settings.hotkeys.open;
        if let Err(error) = self
            .deps
            .registrar
            .register(HotkeyOwner::OpenHistory, hotkey)
        {
            let _ = self
                .deps
                .registrar
                .register(HotkeyOwner::OpenHistory, previous);
            return Err(error.into());
        }
        settings.hotkeys.open = hotkey;
        self.deps.settings.save(&settings)?;
        Ok(hotkey)
    }
}

/// Registers every shortcut at startup (HK-001, SNP-003).
pub struct ApplyHotkeyBindings {
    deps: SnippetDependencies,
}

impl ApplyHotkeyBindings {
    /// Builds the use case.
    pub const fn new(deps: SnippetDependencies) -> Self {
        Self { deps }
    }

    /// Registers the open shortcut and every enabled snippet shortcut; returns the owners whose
    /// shortcut could not be registered (taken by another application, HK-004).
    ///
    /// # Errors
    /// Storage failures reading settings or snippets.
    pub fn execute(&self) -> Result<Vec<HotkeyOwner>, AppError> {
        let open = self.deps.settings.load()?.hotkeys.open;
        let mut failures = Vec::new();
        if let Err(error) = self.deps.registrar.register(HotkeyOwner::OpenHistory, open) {
            tracing::warn!(%error, "open shortcut could not be registered");
            failures.push(HotkeyOwner::OpenHistory);
        }
        for snippet in self.deps.snippets.list()? {
            if snippet.enabled
                && snippet.hotkey.is_some()
                && let Err(error) = sync_registration(self.deps.registrar.as_ref(), &snippet)
            {
                tracing::warn!(%error, id = %snippet.id, "snippet shortcut could not be registered");
                failures.push(HotkeyOwner::Snippet(snippet.id));
            }
        }
        Ok(failures)
    }
}

/// Releases or restores every shortcut while one is being recorded (HK-002).
pub struct SuspendHotkeys {
    registrar: Arc<dyn HotkeyRegistrar>,
}

impl SuspendHotkeys {
    /// Builds the use case.
    pub fn new(registrar: Arc<dyn HotkeyRegistrar>) -> Self {
        Self { registrar }
    }

    /// Applies `suspension`.
    ///
    /// # Errors
    /// System failures.
    pub fn execute(&self, suspension: Suspension) -> Result<(), AppError> {
        Ok(self.registrar.set_suspension(suspension)?)
    }
}
