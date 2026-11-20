//! The settings window (CFG-001), created on demand.

use tauri::utils::config::WindowEffectsConfig;
use tauri::window::Effect;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

use crate::dto::BackdropDto;

/// Window label.
pub const SETTINGS: &str = "settings";

/// Shows the settings window, creating it if needed.
pub fn open(app: &AppHandle<Wry>, backdrop: BackdropDto) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let title = format!(
        "ClipVault — {}",
        app.state::<crate::i18n::Translator>().t("settings.title")
    );
    let builder = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(760.0, 560.0)
        .min_inner_size(560.0, 420.0)
        .center()
        .transparent(backdrop == BackdropDto::Acrylic);
    // THM-002: Mica for the settings window on Windows 11.
    let builder = if backdrop == BackdropDto::Acrylic {
        builder.effects(WindowEffectsConfig {
            effects: vec![Effect::Mica],
            ..WindowEffectsConfig::default()
        })
    } else {
        builder
    };
    if let Err(error) = builder.build() {
        tracing::error!(%error, "settings window could not be created");
    }
}
