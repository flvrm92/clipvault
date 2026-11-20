//! SQLite database: configuration, migrations and corruption recovery (REL-001, REL-005, REL-006).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use clipvault_infrastructure::sqlite::{Database, DatabaseError, LATEST_SCHEMA_VERSION};
use rusqlite::Connection;

#[test]
fn rel_001_file_database_uses_wal_foreign_keys_and_normal_sync() {
    let dir = tempfile::tempdir().unwrap();

    let opened = Database::open(&dir.path().join("history.db")).unwrap();
    let diagnostics = opened.database.diagnostics().unwrap();

    assert_eq!(diagnostics.journal_mode, "wal");
    assert!(diagnostics.foreign_keys);
    assert_eq!(diagnostics.synchronous, 1, "NORMAL");
    assert_eq!(opened.recovered_from, None);
}

#[test]
fn rel_006_a_new_database_is_migrated_to_the_latest_schema() {
    let db = Database::open_in_memory().unwrap();

    assert_eq!(
        db.diagnostics().unwrap().schema_version,
        LATEST_SCHEMA_VERSION
    );
    const { assert!(LATEST_SCHEMA_VERSION >= 1) };
}

#[test]
fn rel_006_reopening_an_existing_database_keeps_its_data_and_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.db");
    drop(Database::open(&path).unwrap());
    Connection::open(&path)
        .unwrap()
        .execute(
            "INSERT INTO settings (key, value) VALUES ('probe', '1')",
            [],
        )
        .unwrap();

    let reopened = Database::open(&path).unwrap().database;

    assert_eq!(
        reopened.diagnostics().unwrap().schema_version,
        LATEST_SCHEMA_VERSION
    );
    let count: i64 = Connection::open(&path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn rel_006_a_database_from_a_newer_app_version_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.db");
    drop(Database::open(&path).unwrap());
    Connection::open(&path)
        .unwrap()
        .pragma_update(None, "user_version", LATEST_SCHEMA_VERSION + 1)
        .unwrap();

    let result = Database::open(&path);

    assert!(
        matches!(result, Err(DatabaseError::NewerSchema { found, .. }) if found == LATEST_SCHEMA_VERSION + 1)
    );
}

#[test]
fn rel_005_a_corrupt_database_is_set_aside_and_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.db");
    std::fs::write(
        &path,
        b"this is definitely not a sqlite database, just garbage bytes!!",
    )
    .unwrap();

    let opened = Database::open(&path).unwrap();

    let backup = opened
        .recovered_from
        .expect("corrupt file should be set aside");
    assert!(
        backup
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("history.corrupt-")
    );
    assert!(
        std::fs::read(&backup)
            .unwrap()
            .starts_with(b"this is definitely not")
    );
    assert_eq!(
        opened.database.diagnostics().unwrap().schema_version,
        LATEST_SCHEMA_VERSION
    );
}

#[test]
fn the_database_folder_is_created_if_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("folder").join("history.db");

    assert!(Database::open(&path).is_ok());
    assert!(path.exists());
}
