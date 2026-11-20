//! Win32 adapters (specs/03 §3.3): clipboard reading/writing and change notifications.
//!
//! This is the only module allowed to use `unsafe` (FFI). Every Win32 resource is owned by a
//! RAII guard (pattern: RAII Guard — specs/04 §P-12) and every `unsafe` block states why the
//! call is sound.
#![allow(unsafe_code)] // FFI boundary; see module docs.

mod accent;
mod apps;
mod clipboard;
mod focus;
mod hotkeys;
mod input;
mod listener;
mod owner;
mod process;
mod startup;
mod theme;
mod window;

pub use accent::{AccentColors, Rgb, system_accent};
pub use apps::Win32RunningApps;
pub use clipboard::Win32Clipboard;
pub use focus::Win32FocusTracker;
pub use hotkeys::Win32HotkeyRegistrar;
pub use input::Win32KeystrokeSender;
pub use listener::ClipboardListener;
pub use startup::{MsixStartupTask, RunKeyStartup, has_package_identity, startup_manager};
pub use theme::{SystemAppearance, system_appearance};
