//! Connection management, configuration and corruption recovery (REL-001, REL-005).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use clipvault_application::ports::PortError;
use rusqlite::{Connection, ErrorCode};

use super::migrations::{self, LATEST_SCHEMA_VERSION, MigrationOutcome};
use super::storage_error;

/// Errors opening the database.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// File system failure (creating the folder, setting a corrupt file aside).
    #[error("database file error: {0}")]
    Io(#[from] std::io::Error),
    /// SQLite failure.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The database was created by a newer app version; it is left untouched.
    #[error("database schema version {found} is newer than supported version {supported}")]
    NewerSchema {
        /// Version found in the file.
        found: u32,
        /// Latest version this build supports.
        supported: u32,
    },
}

/// Configuration and schema details, for diagnostics and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics {
    /// `PRAGMA journal_mode` (`wal` for file databases).
    pub journal_mode: String,
    /// `PRAGMA foreign_keys`.
    pub foreign_keys: bool,
    /// `PRAGMA synchronous` (1 = NORMAL).
    pub synchronous: i64,
    /// `PRAGMA user_version`.
    pub schema_version: u32,
}

/// Result of [`Database::open`].
#[derive(Debug, Clone)]
pub struct Opened {
    /// The ready-to-use database.
    pub database: Database,
    /// Where a corrupt database was moved before a new one was created (REL-005); the UI
    /// notifies the user when this is set.
    pub recovered_from: Option<PathBuf>,
}

/// A migrated, configured SQLite database shared by the repositories.
///
/// A single connection behind a mutex: writes are rare and short (human-paced captures), and
/// WAL keeps them durable. Revisit with a pool only if contention is measured.
#[derive(Debug, Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Opens (creating if needed) the database at `path`. A corrupt file is renamed to
    /// `<stem>.corrupt-<millis>.db` and replaced by a new database (REL-005).
    ///
    /// # Errors
    /// I/O or SQLite failures, or [`DatabaseError::NewerSchema`].
    pub fn open(path: &Path) -> Result<Opened, DatabaseError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match Self::open_at(path) {
            Err(DatabaseError::Sqlite(error)) if is_corruption(&error) => {
                tracing::error!(%error, "database is corrupt; setting it aside");
                let backup = set_aside(path)?;
                Ok(Opened {
                    database: Self::open_at(path)?,
                    recovered_from: Some(backup),
                })
            }
            result => result.map(|database| Opened {
                database,
                recovered_from: None,
            }),
        }
    }

    /// A private in-memory database (tests, benchmarks).
    ///
    /// # Errors
    /// SQLite failures.
    pub fn open_in_memory() -> Result<Self, DatabaseError> {
        Self::initialize(Connection::open_in_memory()?)
    }

    fn open_at(path: &Path) -> Result<Self, DatabaseError> {
        Self::initialize(Connection::open(path)?)
    }

    fn initialize(mut conn: Connection) -> Result<Self, DatabaseError> {
        // journal_mode returns a row, so it is read rather than updated.
        let _: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        match migrations::migrate(&mut conn)? {
            MigrationOutcome::UpToDate => Ok(Self {
                conn: Arc::new(Mutex::new(conn)),
            }),
            MigrationOutcome::NewerSchema(found) => Err(DatabaseError::NewerSchema {
                found,
                supported: LATEST_SCHEMA_VERSION,
            }),
        }
    }

    /// Current configuration and schema version.
    ///
    /// # Errors
    /// Storage failures.
    pub fn diagnostics(&self) -> Result<Diagnostics, PortError> {
        self.with(|conn| {
            Ok(Diagnostics {
                journal_mode: conn.pragma_query_value(None, "journal_mode", |r| r.get(0))?,
                foreign_keys: conn.pragma_query_value(None, "foreign_keys", |r| r.get(0))?,
                synchronous: conn.pragma_query_value(None, "synchronous", |r| r.get(0))?,
                schema_version: migrations::schema_version(conn)?,
            })
        })
    }

    /// Runs `operation` with exclusive access to the connection.
    pub(crate) fn with<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> rusqlite::Result<T>,
    ) -> Result<T, PortError> {
        self.with_mapped(storage_error, operation)
    }

    /// Like [`Self::with`], mapping SQLite errors with `map_error`.
    pub(crate) fn with_mapped<T>(
        &self,
        map_error: impl FnOnce(&rusqlite::Error) -> PortError,
        operation: impl FnOnce(&mut Connection) -> rusqlite::Result<T>,
    ) -> Result<T, PortError> {
        let mut conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        operation(&mut conn).map_err(|e| map_error(&e))
    }
}

fn is_corruption(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt)
    )
}

/// Moves the corrupt database (and its WAL/SHM side files) out of the way.
fn set_aside(path: &Path) -> std::io::Result<PathBuf> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let stem = path
        .file_stem()
        .map_or_else(|| "history".into(), |s| s.to_string_lossy());
    let backup = path.with_file_name(format!("{stem}.corrupt-{millis}.db"));
    std::fs::rename(path, &backup)?;
    for suffix in ["-wal", "-shm"] {
        let mut side = path.as_os_str().to_owned();
        side.push(suffix);
        let _ = std::fs::remove_file(PathBuf::from(side));
    }
    Ok(backup)
}
