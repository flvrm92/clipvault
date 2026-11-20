//! ClipVault infrastructure layer: adapters implementing the application ports
//! (SQLite, file system, Win32).
//!
//! See `specs/03-arquitetura.md` §3.3 and ADR-0002.

pub mod clock;
pub mod dib;
pub mod hotkey_codes;
pub mod images;
pub mod retry;
pub mod sqlite;
#[cfg(windows)]
pub mod win32;
