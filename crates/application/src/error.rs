//! Application error type (specs/03 §6).

use crate::ports::PortError;

/// Errors returned by use cases.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    /// A port (storage, clipboard, …) failed.
    #[error(transparent)]
    Port(#[from] PortError),
}
