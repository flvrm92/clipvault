//! SQLite storage (ADR-0003, specs/05).

mod database;
mod history;
mod migrations;
mod settings;
mod snippets;

pub use database::{Database, DatabaseError, Diagnostics, Opened};
pub use history::SqliteHistoryRepository;
pub use migrations::LATEST_SCHEMA_VERSION;
pub use settings::{SqliteSettingsRepository, settings_from_entries, settings_to_entries};
pub use snippets::SqliteSnippetRepository;

use clipvault_application::ports::PortError;

/// Maps SQLite failures to the port error. SQLite messages never contain row values,
/// so no clipboard content can leak through them (PRIV-003).
fn storage_error(error: &rusqlite::Error) -> PortError {
    PortError::Storage(error.to_string())
}
