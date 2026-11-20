//! The settings window (CFG-001), created on demand.

use tauri::utils::config::WindowEffectsConfig;
use tauri::window::Effect;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

use crate::dto::BackdropDto;
use crate::i18n::Translator;

/// Window label.
pub const SETTINGS: &str = "settings";

/// The section shown when the window opens (TRAY-002: "Configurações" or "Sobre").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    General,
    About,
}

impl Page {
    /// Fragment of the settings URL, also sent to an open window.
    pub const fn anchor(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::About => "about",
        }
    }
}

fn title(app: &AppHandle<Wry>) -> String {
    format!(
        "ClipVault — {}",
        app.state::<Translator>().t("settings.title")
    )
}

/// Shows the settings window on `page`, creating it if needed.
pub fn open(app: &AppHandle<Wry>, backdrop: BackdropDto, page: Page) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit_to(SETTINGS, "settings://navigate", page.anchor());
        return;
    }
    let url = WebviewUrl::App(format!("index.html#{}", page.anchor()).into());
    let builder = WebviewWindowBuilder::new(app, SETTINGS, url)
        .title(title(app))
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

/// Translates the title after a language change (I18N-002).
pub fn retitle(app: &AppHandle<Wry>) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.set_title(&title(app));
    }
}
