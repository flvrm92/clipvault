//! [`SettingsRepository`] over SQLite: one JSON value per key (specs/05 §3).
//!
//! Each key is parsed independently; an invalid or unknown value falls back to its default
//! and is logged, so a bad entry never prevents startup (CFG-006).

use std::collections::BTreeMap;
use std::time::Duration;

use clipvault_application::ports::{PortError, SettingsRepository};
use clipvault_domain::app_name::AppName;
use clipvault_domain::hotkey::Hotkey;
use clipvault_domain::retention::{MaxItems, RetentionPeriod};
use clipvault_domain::settings::{LanguagePreference, Settings, ThemeMode};
use serde_json::{Value, json};

use super::Database;

const SETTINGS_VERSION: u64 = 1;

/// Settings stored in SQLite.
#[derive(Debug, Clone)]
pub struct SqliteSettingsRepository {
    db: Database,
}

impl SqliteSettingsRepository {
    /// A repository over `db`.
    pub const fn new(db: Database) -> Self {
        Self { db }
    }
}

const fn theme_name(theme: ThemeMode) -> &'static str {
    match theme {
        ThemeMode::System => "system",
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    }
}

const fn language_name(language: LanguagePreference) -> &'static str {
    match language {
        LanguagePreference::System => "system",
        LanguagePreference::PtBr => "pt-BR",
        LanguagePreference::En => "en",
    }
}

/// Serializes `settings` to `(key, JSON text)` entries.
pub fn settings_to_entries(settings: &Settings) -> BTreeMap<String, String> {
    let s = settings;
    let entries: [(&str, Value); 15] = [
        ("capture.paused", json!(s.capture.paused)),
        ("capture.max_text_bytes", json!(s.capture.max_text_bytes)),
        ("capture.max_image_bytes", json!(s.capture.max_image_bytes)),
        (
            "capture.excluded_apps",
            json!(
                s.capture
                    .excluded_apps
                    .iter()
                    .map(AppName::as_str)
                    .collect::<Vec<_>>()
            ),
        ),
        (
            "retention.max_age_days",
            json!(s.retention.policy.period.days()),
        ),
        (
            "retention.max_items",
            json!(s.retention.policy.max_items.limit()),
        ),
        ("retention.clear_on_exit", json!(s.retention.clear_on_exit)),
        ("hotkeys.open", json!(s.hotkeys.open.to_string())),
        (
            "snippets.restore_clipboard",
            json!(s.snippets.restore_clipboard),
        ),
        (
            "snippets.restore_delay_ms",
            json!(u64::try_from(s.snippets.restore_delay.as_millis()).unwrap_or(u64::MAX)),
        ),
        ("ui.theme", json!(theme_name(s.ui.theme))),
        ("ui.language", json!(language_name(s.ui.language))),
        ("ui.use_accent_color", json!(s.ui.use_accent_color)),
        ("general.first_run_done", json!(s.general.first_run_done)),
        ("schema.settings_version", json!(SETTINGS_VERSION)),
    ];
    entries
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_string()))
        .collect()
}

/// `None` = invalid value; `Some(None)` = JSON `null` (no limit).
#[allow(clippy::option_option)]
fn optional_u32(value: &Value) -> Option<Option<u32>> {
    if value.is_null() {
        return Some(None);
    }
    value.as_u64().and_then(|n| u32::try_from(n).ok()).map(Some)
}

/// Applies one stored entry; `None` means the value is invalid for that key.
fn apply(settings: &mut Settings, key: &str, value: &Value) -> Option<()> {
    let s = settings;
    match key {
        "capture.paused" => s.capture.paused = value.as_bool()?,
        "capture.max_text_bytes" => s.capture.max_text_bytes = value.as_u64()?,
        "capture.max_image_bytes" => s.capture.max_image_bytes = value.as_u64()?,
        "capture.excluded_apps" => {
            s.capture.excluded_apps = value
                .as_array()?
                .iter()
                .filter_map(|v| v.as_str().and_then(|name| AppName::parse(name).ok()))
                .collect();
        }
        "retention.max_age_days" => {
            s.retention.policy.period = RetentionPeriod::from_days(optional_u32(value)?).ok()?;
        }
        "retention.max_items" => {
            s.retention.policy.max_items = MaxItems::from_count(optional_u32(value)?).ok()?;
        }
        "retention.clear_on_exit" => s.retention.clear_on_exit = value.as_bool()?,
        "hotkeys.open" => s.hotkeys.open = Hotkey::parse(value.as_str()?).ok()?,
        "snippets.restore_clipboard" => s.snippets.restore_clipboard = value.as_bool()?,
        "snippets.restore_delay_ms" => {
            s.snippets.restore_delay = Duration::from_millis(value.as_u64()?);
        }
        "ui.theme" => {
            s.ui.theme = match value.as_str()? {
                "system" => ThemeMode::System,
                "light" => ThemeMode::Light,
                "dark" => ThemeMode::Dark,
                _ => return None,
            };
        }
        "ui.language" => {
            s.ui.language = match value.as_str()? {
                "system" => LanguagePreference::System,
                "pt-BR" => LanguagePreference::PtBr,
                "en" => LanguagePreference::En,
                _ => return None,
            };
        }
        "ui.use_accent_color" => s.ui.use_accent_color = value.as_bool()?,
        "general.first_run_done" => s.general.first_run_done = value.as_bool()?,
        "schema.settings_version" => {}
        _ => tracing::debug!(key, "ignoring unknown setting"),
    }
    Some(())
}

/// Builds settings from stored entries, falling back to the default for each invalid key.
pub fn settings_from_entries(entries: &BTreeMap<String, String>) -> Settings {
    let mut settings = Settings::default();
    for (key, raw) in entries {
        let applied = serde_json::from_str::<Value>(raw)
            .ok()
            .and_then(|value| apply(&mut settings, key, &value));
        if applied.is_none() {
            tracing::warn!(key, "invalid setting value; using the default");
        }
    }
    settings
}

impl SettingsRepository for SqliteSettingsRepository {
    fn load(&self) -> Result<Settings, PortError> {
        let entries = self.db.with(|conn| {
            conn.prepare("SELECT key, value FROM settings")?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<BTreeMap<String, String>>>()
        })?;
        Ok(settings_from_entries(&entries))
    }

    fn save(&self, settings: &Settings) -> Result<(), PortError> {
        let entries = settings_to_entries(settings);
        self.db.with(|conn| {
            let tx = conn.transaction()?;
            {
                let mut upsert = tx.prepare(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                )?;
                for (key, value) in &entries {
                    upsert.execute([key, value])?;
                }
            }
            tx.commit()
        })
    }
}
