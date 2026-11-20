//! Actions of the global shortcuts (HK-001, SNP-003).

use clipvault_application::use_cases::{PasteOutcome, PasteSnippet, SnippetTarget};
use clipvault_domain::hotkey::HotkeyOwner;
use tauri::{AppHandle, Manager, Wry};

use crate::container::Container;
use crate::{notify, popup};

/// Runs on the hotkey thread: hands the work over and returns immediately.
pub fn on_press(app: &AppHandle<Wry>, owner: HotkeyOwner) {
    let app = app.clone();
    std::thread::spawn(move || match owner {
        HotkeyOwner::OpenHistory => popup::toggle(&app),
        HotkeyOwner::Snippet(id) => {
            let deps = app.state::<Container>().paste_snippet_dependencies();
            match PasteSnippet::new(deps).execute(id, SnippetTarget::Foreground) {
                Ok(PasteOutcome::Pasted) => {}
                Ok(PasteOutcome::CopiedOnly(reason)) => {
                    tracing::info!(?reason, "snippet copied; user pastes manually");
                    notify::copied_only(&app);
                }
                Err(error) => tracing::warn!(%error, "snippet could not be pasted"),
            }
        }
    });
}
