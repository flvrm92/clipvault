//! ClipVault domain layer: entities, value objects and business rules.
//!
//! This crate must remain free of I/O, platform and framework dependencies.
//! See `specs/03-arquitetura.md` §3.1 and ADR-0002.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod app_name;
pub mod capture;
pub mod clip;
pub mod hash;
pub mod hotkey;
pub mod ids;
pub mod image;
pub mod retention;
pub mod settings;
pub mod snippet;
pub mod text;
pub mod time;
