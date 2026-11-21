//! Applies language and appearance preferences to every surface after a settings change
//! (pattern: Observer — specs/04 §P-07; I18N-002, THM-001..004).

use clipvault_application::ports::SettingsRepository;
use clipvault_domain::settings::Settings;
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::container::Container;
use crate::i18n::{Locale, Translator};
use crate::popup::POPUP;
use crate::{appearance, settings_window, tray};

/// Current settings; the defaults when storage fails (CFG-006).
pub fn settings(app: &AppHandle<Wry>) -> Settings {
    app.try_state::<Container>()
        .and_then(|c| c.settings.load().ok())
        .unwrap_or_default()
}

/// The effective interface language.
pub fn locale(settings: &Settings) -> Locale {
    Locale::resolve(settings.ui.language, Locale::system)
}

/// Switches the native strings to the configured language and re-applies the appearance.
pub fn apply(app: &AppHandle<Wry>) {
    let settings = settings(app);
    if app.state::<Translator>().set_locale(locale(&settings)) {
        tray::refresh(app);
        settings_window::retitle(app);
    }
    apply_appearance(app, &settings);
}

/// Backdrop of the popup and `appearance://changed` for both windows (THM-001..004).
pub fn apply_appearance(app: &AppHandle<Wry>, settings: &Settings) {
    let appearance = appearance::current(settings.ui);
    if let Some(window) = app.get_webview_window(POPUP) {
        appearance::apply_backdrop(&window, &appearance);
    }
    let _ = app.emit("appearance://changed", appearance);
}
