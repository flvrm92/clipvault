//! Native notifications (PST-005, HK-004).

use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_notification::NotificationExt;

use crate::i18n::Translator;

fn show(app: &AppHandle<Wry>, key: &str) {
    let body = app.state::<Translator>().t(key);
    if let Err(error) = app
        .notification()
        .builder()
        .title("ClipVault")
        .body(body)
        .show()
    {
        tracing::warn!(%error, "notification could not be shown");
    }
}

/// PST-005: the content is on the clipboard; the user pastes with Ctrl+V.
pub fn copied_only(app: &AppHandle<Wry>) {
    show(app, "notification.copiedOnly");
}

/// HK-004: some shortcuts are held by another application.
pub fn shortcuts_unavailable(app: &AppHandle<Wry>) {
    show(app, "notification.shortcutsUnavailable");
}
