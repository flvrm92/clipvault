//! System tray icon and menu (specs/01 TRAY-001, TRAY-002 — foundation subset).

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::i18n::Translator;

/// Label of the main window declared in `tauri.conf.json`.
pub const MAIN_WINDOW: &str = "main";
const TRAY_ID: &str = "main";

/// Actions available in the tray context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    Quit,
}

impl MenuAction {
    const ALL: [Self; 2] = [Self::Open, Self::Quit];

    /// Stable menu item identifier.
    pub fn id(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Quit => "quit",
        }
    }

    /// Translation key of the menu label.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Open => "tray.open",
            Self::Quit => "tray.quit",
        }
    }

    /// Parses a menu item identifier.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.id() == id)
    }
}

/// Creates the tray icon with its context menu.
///
/// # Errors
/// Fails if the native menu or tray icon cannot be created.
pub fn create<R: Runtime>(app: &AppHandle<R>, translator: &Translator) -> tauri::Result<()> {
    let items = MenuAction::ALL
        .into_iter()
        .map(|a| MenuItem::with_id(app, a.id(), translator.t(a.label_key()), true, None::<&str>))
        .collect::<tauri::Result<Vec<_>>>()?;
    let refs: Vec<&dyn tauri::menu::IsMenuItem<R>> = items
        .iter()
        .map(|i| i as &dyn tauri::menu::IsMenuItem<R>)
        .collect();
    let menu = Menu::with_items(app, &refs)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(translator.t("tray.tooltip"))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(
            |app, event| match MenuAction::from_id(event.id().as_ref()) {
                Some(MenuAction::Open) => show_main_window(app),
                Some(MenuAction::Quit) => app.exit(0),
                None => {}
            },
        )
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        // Failures here are non-fatal: the user can retry from the tray.
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        show_main_window(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Locale;

    #[test]
    fn tray_002_menu_ids_round_trip() {
        for action in MenuAction::ALL {
            assert_eq!(MenuAction::from_id(action.id()), Some(action));
        }
        assert_eq!(MenuAction::from_id("unknown"), None);
    }

    #[test]
    fn tray_002_menu_labels_are_translated_in_both_locales() {
        for locale in [Locale::En, Locale::PtBr] {
            let tr = Translator::new(locale).unwrap();
            for action in MenuAction::ALL {
                assert_ne!(
                    tr.t(action.label_key()),
                    action.label_key(),
                    "{locale:?} {action:?}"
                );
            }
        }
    }
}
