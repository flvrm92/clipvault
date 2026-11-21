// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    // CMP-003: no window can be created without WebView2.
    if !clipvault_app::webview2_available() {
        return ExitCode::FAILURE;
    }
    match clipvault_app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ClipVault failed to start: {error}");
            ExitCode::FAILURE
        }
    }
}
