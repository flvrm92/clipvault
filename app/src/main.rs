// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    match clipvault_app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ClipVault failed to start: {error}");
            ExitCode::FAILURE
        }
    }
}
