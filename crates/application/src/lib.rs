//! ClipVault application layer: use cases and ports (traits).
//!
//! Depends only on `clipvault-domain`. Adapters live in `clipvault-infrastructure`.
//! See `specs/03-arquitetura.md` §3.2 and ADR-0002.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod capture;
mod error;
pub mod placement;
pub mod ports;
pub mod use_cases;

pub use error::{AppError, HotkeyConflict};
