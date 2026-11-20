//! ClipVault composition root (specs/03 §3.4).
//!
//! Wires adapters into use cases and exposes them to the UI. Contains no business rules.

mod container;
mod i18n;
mod tray;

use tauri::{Manager, WindowEvent};

use crate::container::Container;
use crate::i18n::{Locale, Translator};

/// Builds and runs the Tauri application until the user quits from the tray.
///
/// # Errors
/// Returns an error if the application fails to initialize.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let translator = Translator::new(Locale::system())?;
            tray::create(app.handle(), &translator)?;

            let data_dir = app.path().app_local_data_dir()?;
            let container = Container::build(&data_dir, app.handle().clone())?;
            container.start_maintenance();
            app.manage(container);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window hides it; the app keeps running in the tray (TRAY-001).
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == tray::MAIN_WINDOW
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
}
