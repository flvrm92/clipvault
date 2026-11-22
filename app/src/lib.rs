//! ClipVault composition root (specs/03 §3.4).
//!
//! Wires adapters into use cases and exposes them to the UI. Contains no business rules.

mod appearance;
mod commands;
mod container;
mod dto;
mod hotkeys;
mod i18n;
mod logging;
mod notify;
mod popup;
mod preferences;
mod protocol;
mod runtime_check;
mod settings_commands;
mod settings_window;
mod tray;
mod welcome_window;

use clipvault_application::ports::SettingsRepository;
use tauri::{Manager, RunEvent, WindowEvent};

use crate::container::Container;
use crate::i18n::Translator;
use crate::popup::{POPUP, PopupState};

pub use crate::runtime_check::webview2_available;

fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    match event {
        // PST-007: the popup closes when it loses focus; closing hides it (TRAY-001).
        WindowEvent::Focused(false) if window.label() == POPUP => {
            popup::hide(window.app_handle());
        }
        WindowEvent::CloseRequested { api, .. } if window.label() == POPUP => {
            api.prevent_close();
            popup::hide(window.app_handle());
        }
        // TRAY-006: closing the welcome window without a choice is no consent.
        WindowEvent::Destroyed if window.label() == welcome_window::WELCOME => {
            settings_commands::dismiss_welcome(window.app_handle());
        }
        // THM-001/003: follow theme changes without restarting.
        WindowEvent::ThemeChanged(_) => {
            let app = window.app_handle();
            preferences::apply_appearance(app, &preferences::settings(app));
            tray::refresh(app);
        }
        _ => {}
    }
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
        .plugin(tauri_plugin_opener::init())
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
            commands::paste_snippet,
            commands::list_snippets,
            commands::get_shortcut_settings,
            commands::save_snippet,
            commands::delete_snippet,
            commands::change_open_hotkey,
            commands::set_hotkey_suspension,
            commands::set_restore_clipboard,
            commands::open_settings,
            settings_commands::get_settings,
            settings_commands::set_language,
            settings_commands::set_theme,
            settings_commands::set_use_accent_color,
            settings_commands::preview_retention,
            settings_commands::set_retention,
            settings_commands::set_clear_on_exit,
            settings_commands::set_size_limit,
            settings_commands::add_excluded_app,
            settings_commands::remove_excluded_app,
            settings_commands::list_running_apps,
            settings_commands::restore_defaults,
            settings_commands::get_about,
            settings_commands::open_about_link,
            settings_commands::open_data_folder,
            settings_commands::get_startup,
            settings_commands::set_startup,
            settings_commands::get_welcome,
            settings_commands::complete_welcome,
        ])
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            if let Some(guard) = logging::init(&data_dir.join("logs")) {
                app.manage(guard);
            }
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "ClipVault starting");

            let container = Container::build(&data_dir, app.handle().clone())?;
            // I18N-001/002: native strings start in the configured language.
            let settings = container
                .settings
                .load()
                .inspect_err(|error| tracing::warn!(%error, "settings could not be read"))
                .unwrap_or_default();
            app.manage(Translator::new(preferences::locale(&settings))?);
            container.start_maintenance();
            if let Err(error) = container.start_capture() {
                tracing::error!(%error, "clipboard capture could not start");
            }
            let unavailable = container.start_hotkeys();
            app.manage(container);

            tray::create(app.handle())?;
            if !unavailable.is_empty() {
                notify::shortcuts_unavailable(app.handle());
            }
            preferences::apply_appearance(app.handle(), &settings);
            settings_commands::show_welcome_if_pending(app.handle());
            Ok(())
        })
        .on_window_event(on_window_event)
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
