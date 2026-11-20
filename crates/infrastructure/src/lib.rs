//! ClipVault infrastructure layer: adapters implementing the application ports
//! (SQLite, file system, Win32).
//!
//! See `specs/03-arquitetura.md` §3.3 and ADR-0002.

pub mod clock;
pub mod images;
pub mod sqlite;
