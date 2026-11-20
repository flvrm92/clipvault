//! Forward-only schema migrations tracked in `PRAGMA user_version` (specs/05 §6, REL-006).
//!
//! Each migration runs in its own transaction together with the version bump, so a failure
//! leaves the database at the previous version.

use rusqlite::Connection;

const MIGRATIONS: &[&str] = &[include_str!("migrations/001_initial.sql")];

/// Schema version this build creates and understands.
#[allow(clippy::cast_possible_truncation)] // a handful of migrations
pub const LATEST_SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

/// Outcome of [`migrate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationOutcome {
    /// The schema is now at [`LATEST_SCHEMA_VERSION`].
    UpToDate,
    /// The database was created by a newer app version.
    NewerSchema(u32),
}

/// Current schema version of the database.
pub fn schema_version(conn: &Connection) -> rusqlite::Result<u32> {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
}

/// Applies pending migrations.
pub fn migrate(conn: &mut Connection) -> rusqlite::Result<MigrationOutcome> {
    let current = schema_version(conn)?;
    if current > LATEST_SCHEMA_VERSION {
        return Ok(MigrationOutcome::NewerSchema(current));
    }
    for (version, sql) in (1..).zip(MIGRATIONS).skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
    }
    Ok(MigrationOutcome::UpToDate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rel_006_migrating_twice_is_a_no_op() {
        let mut conn = Connection::open_in_memory().unwrap();

        assert_eq!(migrate(&mut conn).unwrap(), MigrationOutcome::UpToDate);
        assert_eq!(migrate(&mut conn).unwrap(), MigrationOutcome::UpToDate);
        assert_eq!(schema_version(&conn).unwrap(), LATEST_SCHEMA_VERSION);
    }

    #[test]
    fn rel_006_initial_schema_creates_every_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert_eq!(
            tables,
            ["clip_items", "image_assets", "settings", "snippets"]
        );
    }

    #[test]
    fn rel_006_newer_schemas_are_reported_untouched() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", LATEST_SCHEMA_VERSION + 3)
            .unwrap();

        assert_eq!(
            migrate(&mut conn).unwrap(),
            MigrationOutcome::NewerSchema(LATEST_SCHEMA_VERSION + 3)
        );
    }
}
