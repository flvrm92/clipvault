//! SQLite repositories honour the port contracts (specs/04 §P-11) and CFG-006.
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use clipvault_application::ports::SettingsRepository;
use clipvault_domain::hotkey::Hotkey;
use clipvault_domain::retention::{MaxItems, RetentionPeriod};
use clipvault_domain::settings::{Settings, ThemeMode};
use clipvault_infrastructure::sqlite::{
    Database, SqliteHistoryRepository, SqliteSettingsRepository, settings_from_entries,
    settings_to_entries,
};
use clipvault_test_support::contracts::settings::customized;

fn history() -> SqliteHistoryRepository {
    SqliteHistoryRepository::new(Database::open_in_memory().unwrap())
}

fn settings() -> SqliteSettingsRepository {
    SqliteSettingsRepository::new(Database::open_in_memory().unwrap())
}

mod history_contract {
    use super::*;
    clipvault_test_support::history_repository_contract!(history());
}

mod snippets_contract {
    use super::*;
    clipvault_test_support::snippet_repository_contract!(
        clipvault_infrastructure::sqlite::SqliteSnippetRepository::new(
            Database::open_in_memory().unwrap()
        )
    );
}

mod settings_contract {
    use super::*;
    clipvault_test_support::settings_repository_contract!(settings());
}

fn entries(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn settings_are_stored_one_json_value_per_key() {
    let stored = settings_to_entries(&Settings::default());

    assert_eq!(
        stored.get("hotkeys.open").map(String::as_str),
        Some("\"Ctrl+Win+V\"")
    );
    assert_eq!(
        stored.get("retention.max_age_days").map(String::as_str),
        Some("30")
    );
    assert_eq!(
        stored.get("retention.max_items").map(String::as_str),
        Some("1000")
    );
    assert_eq!(
        stored.get("ui.theme").map(String::as_str),
        Some("\"system\"")
    );
    assert_eq!(
        stored.get("schema.settings_version").map(String::as_str),
        Some("1")
    );
}

#[test]
fn settings_entries_round_trip() {
    assert_eq!(
        settings_from_entries(&settings_to_entries(&customized())),
        customized()
    );
}

#[test]
fn cfg_006_invalid_values_fall_back_to_defaults_per_key() {
    let loaded = settings_from_entries(&entries(&[
        ("ui.theme", "\"purple\""),            // unknown option
        ("hotkeys.open", "\"Ctrl+C\""),        // reserved shortcut
        ("retention.max_age_days", "2"),       // not an offered option
        ("capture.max_text_bytes", "\"abc\""), // wrong type
        ("capture.paused", "not json at all"), // unparseable
        ("retention.max_items", "5000"),       // valid: kept
        ("unknown.key", "true"),               // ignored
    ]));

    let defaults = Settings::default();
    assert_eq!(loaded.ui.theme, defaults.ui.theme);
    assert_eq!(loaded.hotkeys.open, defaults.hotkeys.open);
    assert_eq!(
        loaded.retention.policy.period,
        defaults.retention.policy.period
    );
    assert_eq!(
        loaded.capture.max_text_bytes,
        defaults.capture.max_text_bytes
    );
    assert_eq!(loaded.capture.paused, defaults.capture.paused);
    assert_eq!(loaded.retention.policy.max_items, MaxItems::FiveThousand);
}

#[test]
fn cfg_006_invalid_excluded_apps_are_skipped_individually() {
    let loaded = settings_from_entries(&entries(&[(
        "capture.excluded_apps",
        r#"["KeePass.exe", "  ", 42]"#,
    )]));

    let names: Vec<_> = loaded
        .capture
        .excluded_apps
        .iter()
        .map(clipvault_domain::app_name::AppName::as_str)
        .collect();
    assert_eq!(names, ["keepass.exe"]);
}

#[test]
fn null_retention_values_mean_no_limit() {
    let loaded = settings_from_entries(&entries(&[
        ("retention.max_age_days", "null"),
        ("retention.max_items", "null"),
    ]));

    assert_eq!(loaded.retention.policy.period, RetentionPeriod::Forever);
    assert_eq!(loaded.retention.policy.max_items, MaxItems::Unlimited);
}

#[test]
fn settings_survive_reopening_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.db");
    let mut custom = Settings::default();
    custom.ui.theme = ThemeMode::Light;
    custom.hotkeys.open = Hotkey::parse("Alt+V").unwrap();

    SqliteSettingsRepository::new(Database::open(&path).unwrap().database)
        .save(&custom)
        .unwrap();
    let reloaded = SqliteSettingsRepository::new(Database::open(&path).unwrap().database)
        .load()
        .unwrap();

    assert_eq!(reloaded, custom);
}
