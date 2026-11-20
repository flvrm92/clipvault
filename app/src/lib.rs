//! ClipVault composition root (specs/03 §3.4).
//!
//! Wires adapters into use cases and exposes them to the UI. Contains no business rules.

mod appearance;
mod commands;
mod container;
mod dto;
mod i18n;
mod logging;
mod popup;
mod protocol;
mod tray;

use clipvault_application::ports::SettingsRepository;
use clipvault_domain::settings::ThemeMode;
use tauri::{Emitter, Manager, RunEvent, WindowEvent, Wry};

use crate::container::Container;
use crate::i18n::{Locale, Translator};
use crate::popup::{POPUP, PopupState};

fn apply_appearance(app: &tauri::AppHandle<Wry>) {
    let mode = app
        .state::<Container>()
        .settings
        .load()
        .map_or(ThemeMode::default(), |s| s.ui.theme);
    let appearance = appearance::current(mode);
    if let Some(window) = app.get_webview_window(POPUP) {
        appearance::apply_backdrop(&window, appearance);
    }
    let _ = app.emit("appearance://changed", appearance);
}

/// Builds and runs the Tauri application until the user quits from the tray.
///
/// # Errors
/// Returns an error if the application fails to initialize.
pub fn run() -> tauri::Result<()> {
    let app = tauri::Builder::default()
        // TRAY-004: a second launch opens the popup of the running instance and exits.
        // Registered first, as the plugin requires.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            popup::show(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .manage(PopupState::default())
        .register_uri_scheme_protocol(protocol::SCHEME, |ctx, request| {
            protocol::handle(ctx.app_handle(), request.uri().path())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_history,
            commands::get_item_text,
            commands::paste_item,
            commands::copy_item,
            commands::toggle_pin,
            commands::delete_item,
            commands::clear_history,
            commands::set_capture_state,
            commands::get_popup_state,
            commands::hide_popup,
        ])
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            if let Some(guard) = logging::init(&data_dir.join("logs")) {
                app.manage(guard);
            }
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "ClipVault starting");
            app.manage(Translator::new(Locale::system())?);

            let container = Container::build(&data_dir, app.handle().clone())?;
            container.start_maintenance();
            if let Err(error) = container.start_capture() {
                tracing::error!(%error, "clipboard capture could not start");
            }
            app.manage(container);

            tray::create(app.handle())?;
            apply_appearance(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // PST-007: the popup closes when it loses focus; closing hides it (TRAY-001).
            WindowEvent::Focused(false) if window.label() == POPUP => {
                popup::hide(window.app_handle());
            }
            WindowEvent::CloseRequested { api, .. } if window.label() == POPUP => {
                api.prevent_close();
                popup::hide(window.app_handle());
            }
            // THM-001/003: follow theme changes without restarting.
            WindowEvent::ThemeChanged(_) => {
                apply_appearance(window.app_handle());
                tray::refresh(window.app_handle());
            }
            _ => {}
        })
        .build(tauri::generate_context!())?;

    app.run(|handle, event| {
        // TRAY-005: unregister the listener and finish pending captures before exiting.
        if let RunEvent::Exit = event
            && let Some(container) = handle.try_state::<Container>()
        {
            container.shutdown();
            tracing::info!("ClipVault stopped");
        }
    });
    Ok(())
}
