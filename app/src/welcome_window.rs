//! The first-run welcome window (TRAY-006).

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

use crate::i18n::Translator;

/// Window label.
pub const WELCOME: &str = "welcome";

/// Shows the welcome window.
pub fn open(app: &AppHandle<Wry>) {
    if let Some(window) = app.get_webview_window(WELCOME) {
        let _ = window.set_focus();
        return;
    }
    let title = app.state::<Translator>().t("welcome.title");
    let built = WebviewWindowBuilder::new(app, WELCOME, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(440.0, 360.0)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .center()
        .focused(true)
        .build();
    if let Err(error) = built {
        tracing::error!(%error, "welcome window could not be created");
    }
}

/// Closes the welcome window, if open.
pub fn close(app: &AppHandle<Wry>) {
    if let Some(window) = app.get_webview_window(WELCOME) {
        let _ = window.destroy();
    }
}
