//! Win32 adapters (specs/03 §3.3): clipboard reading/writing and change notifications.
//!
//! This is the only module allowed to use `unsafe` (FFI). Every Win32 resource is owned by a
//! RAII guard (pattern: RAII Guard — specs/04 §P-12) and every `unsafe` block states why the
//! call is sound.
#![allow(unsafe_code)] // FFI boundary; see module docs.

mod clipboard;
mod listener;
mod owner;
mod window;

pub use clipboard::Win32Clipboard;
pub use listener::ClipboardListener;
