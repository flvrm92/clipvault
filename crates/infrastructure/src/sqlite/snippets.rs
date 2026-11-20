//! [`SnippetRepository`] over SQLite (specs/05 §2, table `snippets`).

use clipvault_application::ports::{PortError, SnippetRepository};
use clipvault_domain::hotkey::Hotkey;
use clipvault_domain::ids::SnippetId;
use clipvault_domain::snippet::{NewSnippet, Snippet, SnippetBody, SnippetName};
use clipvault_domain::time::Timestamp;
use rusqlite::types::Type;
use rusqlite::{ErrorCode, OptionalExtension, Row, params};

use super::{Database, storage_error};

const SELECT: &str = "SELECT id, name, body, hotkey, enabled, sort_order FROM snippets";

/// Snippets stored in SQLite.
#[derive(Debug, Clone)]
pub struct SqliteSnippetRepository {
    db: Database,
}

impl SqliteSnippetRepository {
    /// A repository over `db`.
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

fn invalid(column: usize) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, "invalid snippet field".into())
}

fn read(row: &Row<'_>) -> rusqlite::Result<Snippet> {
    let hotkey = row
        .get::<_, Option<String>>(3)?
        .map(|text| Hotkey::parse(&text).map_err(|_| invalid(3)))
        .transpose()?;
    Ok(Snippet {
        id: SnippetId::new(row.get(0)?),
        name: SnippetName::parse(&row.get::<_, String>(1)?).map_err(|_| invalid(1))?,
        body: SnippetBody::new(&row.get::<_, String>(2)?).map_err(|_| invalid(2))?,
        hotkey,
        enabled: row.get(4)?,
        order: u32::try_from(row.get::<_, i64>(5)?).map_err(|_| invalid(5))?,
    })
}

/// Unique-constraint violations (duplicate name or shortcut) become [`PortError::InUse`].
fn write_error(error: &rusqlite::Error) -> PortError {
    if error.sqlite_error_code() == Some(ErrorCode::ConstraintViolation) {
        PortError::InUse("snippet name or shortcut".into())
    } else {
        storage_error(error)
    }
}

impl SnippetRepository for SqliteSnippetRepository {
    fn list(&self) -> Result<Vec<Snippet>, PortError> {
        self.db.with(|conn| {
            conn.prepare(&format!("{SELECT} ORDER BY sort_order, id"))?
                .query_map([], read)?
                .collect()
        })
    }

    fn get(&self, id: SnippetId) -> Result<Option<Snippet>, PortError> {
        self.db.with(|conn| {
            conn.query_row(&format!("{SELECT} WHERE id = ?1"), [id.get()], read)
                .optional()
        })
    }

    fn insert(&self, snippet: NewSnippet, at: Timestamp) -> Result<Snippet, PortError> {
        self.db.with_mapped(write_error, |conn| {
            let order: i64 = conn.query_row(
                "SELECT COALESCE(MAX(sort_order) + 1, 0) FROM snippets",
                [],
                |r| r.get(0),
            )?;
            conn.execute(
                "INSERT INTO snippets (name, body, hotkey, enabled, sort_order, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![
                    snippet.name.as_str(),
                    snippet.body.as_str(),
                    snippet.hotkey.map(|h| h.to_string()),
                    snippet.enabled,
                    order,
                    at.as_millis(),
                ],
            )?;
            Ok(Snippet {
                id: SnippetId::new(conn.last_insert_rowid()),
                name: snippet.name,
                body: snippet.body,
                hotkey: snippet.hotkey,
                enabled: snippet.enabled,
                order: u32::try_from(order).unwrap_or(u32::MAX),
            })
        })
    }

    fn update(&self, snippet: &Snippet, at: Timestamp) -> Result<(), PortError> {
        let changed = self.db.with_mapped(write_error, |conn| {
            conn.execute(
                "UPDATE snippets SET name = ?2, body = ?3, hotkey = ?4, enabled = ?5,
                     sort_order = ?6, updated_at = ?7
                 WHERE id = ?1",
                params![
                    snippet.id.get(),
                    snippet.name.as_str(),
                    snippet.body.as_str(),
                    snippet.hotkey.map(|h| h.to_string()),
                    snippet.enabled,
                    snippet.order,
                    at.as_millis(),
                ],
            )
        })?;
        if changed == 0 {
            return Err(PortError::NotFound);
        }
        Ok(())
    }

    fn delete(&self, id: SnippetId) -> Result<(), PortError> {
        let changed = self
            .db
            .with(|conn| conn.execute("DELETE FROM snippets WHERE id = ?1", [id.get()]))?;
        if changed == 0 {
            return Err(PortError::NotFound);
        }
        Ok(())
    }
}
