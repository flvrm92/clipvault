//! System tray icon and menu (TRAY-001..003, THM-003).

use clipvault_application::use_cases::{CaptureState, SetCaptureState};
use clipvault_infrastructure::win32::system_appearance;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::container::Container;
use crate::i18n::Translator;
use crate::popup;

const TRAY_ID: &str = "main";

const ICON_LIGHT: &[u8] = include_bytes!("../icons/tray/tray-light.png");
const ICON_DARK: &[u8] = include_bytes!("../icons/tray/tray-dark.png");
const ICON_PAUSED_LIGHT: &[u8] = include_bytes!("../icons/tray/tray-paused-light.png");
const ICON_PAUSED_DARK: &[u8] = include_bytes!("../icons/tray/tray-paused-dark.png");

/// Actions available in the tray context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    TogglePause,
    Settings,
    Quit,
}

impl MenuAction {
    const ALL: [Self; 4] = [Self::Open, Self::TogglePause, Self::Settings, Self::Quit];

    /// Stable menu item identifier.
    pub fn id(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::TogglePause => "toggle-pause",
            Self::Settings => "settings",
            Self::Quit => "quit",
        }
    }

    /// Parses a menu item identifier.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.id() == id)
    }
}

/// Icon for the taskbar theme and capture state (THM-003, TRAY-003). The glyph on a light
/// taskbar is dark and vice versa.
pub fn icon_bytes(taskbar_light: bool, state: CaptureState) -> &'static [u8] {
    match (taskbar_light, state) {
        (true, CaptureState::Active) => ICON_LIGHT,
        (false, CaptureState::Active) => ICON_DARK,
        (true, CaptureState::Paused) => ICON_PAUSED_LIGHT,
        (false, CaptureState::Paused) => ICON_PAUSED_DARK,
    }
}

/// Translation key of the pause/resume menu item.
pub const fn pause_label_key(state: CaptureState) -> &'static str {
    match state {
        CaptureState::Active => "tray.pause",
        CaptureState::Paused => "tray.resume",
    }
}

/// Translation key of the tooltip.
pub const fn tooltip_key(state: CaptureState) -> &'static str {
    match state {
        CaptureState::Active => "tray.tooltip",
        CaptureState::Paused => "tray.tooltipPaused",
    }
}

/// The menu item whose label follows the capture state.
pub struct TrayMenu {
    pause: MenuItem<Wry>,
}

fn capture_state(app: &AppHandle<Wry>) -> CaptureState {
    let container = app.state::<Container>();
    SetCaptureState::new(container.settings.clone(), container.events.clone())
        .current()
        .unwrap_or(CaptureState::Active)
}

fn toggle_pause(app: &AppHandle<Wry>) {
    let container = app.state::<Container>();
    let use_case = SetCaptureState::new(container.settings.clone(), container.events.clone());
    let next = match use_case.current() {
        Ok(CaptureState::Active) => CaptureState::Paused,
        _ => CaptureState::Active,
    };
    if let Err(error) = use_case.execute(next) {
        tracing::warn!(%error, "capture state could not be changed");
    }
}

/// Creates the tray icon with its context menu.
///
/// # Errors
/// Fails if the native menu or tray icon cannot be created.
pub fn create(app: &AppHandle<Wry>) -> tauri::Result<()> {
    let translator = app.state::<Translator>();
    let item = |action: MenuAction, key: &str| {
        MenuItem::with_id(app, action.id(), translator.t(key), true, None::<&str>)
    };
    let open = item(MenuAction::Open, "tray.open")?;
    let pause = item(MenuAction::TogglePause, "tray.pause")?;
    let settings = item(MenuAction::Settings, "tray.settings")?;
    let quit = item(MenuAction::Quit, "tray.quit")?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &pause,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    app.manage(TrayMenu { pause });

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(
            |app, event| match MenuAction::from_id(event.id().as_ref()) {
                Some(MenuAction::Open) => popup::show(app),
                Some(MenuAction::TogglePause) => toggle_pause(app),
                Some(MenuAction::Settings) => {
                    // Created off the event thread, like the async `open_settings` command.
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        crate::commands::open_settings_window(&app);
                    });
                }
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
                popup::toggle(tray.app_handle());
            }
        })
        .build(app)?;
    refresh(app);
    Ok(())
}

/// Updates icon, tooltip and menu label after a capture-state or taskbar-theme change.
pub fn refresh(app: &AppHandle<Wry>) {
    let state = capture_state(app);
    let translator = app.state::<Translator>();
    if let Some(menu) = app.try_state::<TrayMenu>() {
        let _ = menu.pause.set_text(translator.t(pause_label_key(state)));
    }
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    match Image::from_bytes(icon_bytes(system_appearance().taskbar_light, state)) {
        Ok(icon) => {
            let _ = tray.set_icon(Some(icon));
        }
        Err(error) => tracing::warn!(%error, "tray icon could not be decoded"),
    }
    let _ = tray.set_tooltip(Some(translator.t(tooltip_key(state))));
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
    fn tray_003_paused_state_changes_icon_tooltip_and_label() {
        assert_ne!(
            icon_bytes(true, CaptureState::Active),
            icon_bytes(true, CaptureState::Paused)
        );
        assert_eq!(pause_label_key(CaptureState::Paused), "tray.resume");
        assert_eq!(tooltip_key(CaptureState::Paused), "tray.tooltipPaused");
    }

    #[test]
    fn thm_003_icon_follows_the_taskbar_theme() {
        assert_ne!(
            icon_bytes(true, CaptureState::Active),
            icon_bytes(false, CaptureState::Active)
        );
        for bytes in [ICON_LIGHT, ICON_DARK, ICON_PAUSED_LIGHT, ICON_PAUSED_DARK] {
            assert!(Image::from_bytes(bytes).is_ok());
        }
    }

    #[test]
    fn tray_002_labels_are_translated_in_both_locales() {
        for locale in [Locale::En, Locale::PtBr] {
            let tr = Translator::new(locale).unwrap();
            for key in [
                "tray.open",
                "tray.pause",
                "tray.resume",
                "tray.quit",
                "tray.settings",
                "notification.shortcutsUnavailable",
                "tray.tooltip",
                "tray.tooltipPaused",
                "notification.copiedOnly",
            ] {
                assert_ne!(tr.t(key), key, "{locale:?} {key}");
            }
        }
    }
}
