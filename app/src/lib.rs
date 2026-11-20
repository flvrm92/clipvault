//! ClipVault composition root (specs/03 §3.4).
//!
//! Wires adapters into use cases and exposes them to the UI. Contains no business rules.

mod container;
mod i18n;
mod logging;
mod tray;

use tauri::{Manager, RunEvent, WindowEvent};

use crate::container::Container;
use crate::i18n::{Locale, Translator};

/// Builds and runs the Tauri application until the user quits from the tray.
///
/// # Errors
/// Returns an error if the application fails to initialize.
pub fn run() -> tauri::Result<()> {
    let app = tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            if let Some(guard) = logging::init(&data_dir.join("logs")) {
                app.manage(guard);
            }
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "ClipVault starting");

            let translator = Translator::new(Locale::system())?;
            tray::create(app.handle(), &translator)?;

            let container = Container::build(&data_dir, app.handle().clone())?;
            container.start_maintenance();
            if let Err(error) = container.start_capture() {
                tracing::error!(%error, "clipboard capture could not start");
            }
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
