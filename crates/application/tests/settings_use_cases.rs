//! Settings window use cases (CFG-004, CAP-006, CAP-009, RET-004, RET-006).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use clipvault_application::AppError;
use clipvault_application::ports::{
    AppEvent, HistoryRepository, HotkeyRegistrar, SettingsRepository, SnippetRepository,
};
use clipvault_application::use_cases::{
    ChangeRetention, ChangeSizeLimit, ClearOnExit, ExcludeApp, IncludeApp, ListExclusionCandidates,
    RestoreDefaults, RetentionDependencies, SizeLimit, SnippetDependencies,
};
use clipvault_domain::app_name::AppName;
use clipvault_domain::clip::PinState;
use clipvault_domain::hotkey::{Hotkey, HotkeyOwner};
use clipvault_domain::retention::{MaxItems, RetentionPeriod, RetentionPolicy};
use clipvault_domain::settings::{LanguagePreference, SettingsError, SettingsSection, ThemeMode};
use clipvault_domain::snippet::{NewSnippet, SnippetBody, SnippetName};
use clipvault_domain::time::Timestamp;
use clipvault_test_support::contracts::history::text_item;
use clipvault_test_support::{
    FakeHotkeyRegistrar, FakeRunningApps, FixedClock, FixedSettings, InMemoryHistoryRepository,
    InMemoryImageStore, InMemorySnippetRepository, RecordingEvents,
};

const DAY_MS: i64 = 24 * 60 * 60 * 1_000;

struct Fixture {
    history: Arc<InMemoryHistoryRepository>,
    images: Arc<InMemoryImageStore>,
    settings: Arc<FixedSettings>,
    clock: Arc<FixedClock>,
    events: Arc<RecordingEvents>,
    snippets: Arc<InMemorySnippetRepository>,
    registrar: Arc<FakeHotkeyRegistrar>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            history: Arc::new(InMemoryHistoryRepository::default()),
            images: Arc::new(InMemoryImageStore::default()),
            settings: Arc::new(FixedSettings::default()),
            clock: Arc::new(FixedClock::at(Timestamp::from_millis(100 * DAY_MS))),
            events: Arc::new(RecordingEvents::default()),
            snippets: Arc::new(InMemorySnippetRepository::default()),
            registrar: Arc::new(FakeHotkeyRegistrar::default()),
        }
    }

    fn retention(&self) -> RetentionDependencies {
        RetentionDependencies {
            history: self.history.clone(),
            images: self.images.clone(),
            settings: self.settings.clone(),
            clock: self.clock.clone(),
            events: self.events.clone(),
        }
    }

    fn snippet_deps(&self) -> SnippetDependencies {
        SnippetDependencies {
            snippets: self.snippets.clone(),
            settings: self.settings.clone(),
            registrar: self.registrar.clone(),
            clock: self.clock.clone(),
        }
    }

    fn restore(&self) -> RestoreDefaults {
        RestoreDefaults::new(self.retention(), self.snippet_deps())
    }

    fn text(&self, text: &str, days_ago: i64) {
        self.history
            .insert(text_item(text, (100 - days_ago) * DAY_MS))
            .unwrap();
    }

    fn stored(&self) -> clipvault_domain::settings::Settings {
        self.settings.load().unwrap()
    }
}

fn policy(period: RetentionPeriod, max_items: MaxItems) -> RetentionPolicy {
    RetentionPolicy { period, max_items }
}

fn app(name: &str) -> AppName {
    AppName::parse(name).unwrap()
}

// ---- RET-004: changing the retention policy ----

#[test]
fn ret_004_preview_counts_without_changing_anything() {
    let f = Fixture::new();
    f.text("old", 10);
    f.text("new", 0);

    let count = ChangeRetention::new(f.retention())
        .preview(policy(RetentionPeriod::SevenDays, MaxItems::Thousand))
        .unwrap();

    assert_eq!(count, 1);
    assert_eq!(f.history.retention_candidates().unwrap().len(), 2);
    assert_eq!(f.stored().retention.policy, RetentionPolicy::default());
}

#[test]
fn ret_004_a_new_policy_is_saved_and_applied_immediately() {
    let f = Fixture::new();
    f.text("old", 10);
    f.text("new", 0);
    let new_policy = policy(RetentionPeriod::SevenDays, MaxItems::Thousand);

    let removed = ChangeRetention::new(f.retention())
        .execute(new_policy)
        .unwrap();

    assert_eq!(removed, 1);
    assert_eq!(f.stored().retention.policy, new_policy);
    assert_eq!(f.history.retention_candidates().unwrap().len(), 1);
    assert_eq!(
        f.events.recorded(),
        vec![AppEvent::SettingsChanged, AppEvent::HistoryChanged]
    );
}

#[test]
fn ret_004_a_policy_that_removes_nothing_does_not_announce_history_changes() {
    let f = Fixture::new();
    f.text("new", 0);

    let removed = ChangeRetention::new(f.retention())
        .execute(policy(RetentionPeriod::Forever, MaxItems::Unlimited))
        .unwrap();

    assert_eq!(removed, 0);
    assert_eq!(f.events.recorded(), vec![AppEvent::SettingsChanged]);
}

// ---- CFG-004: restore defaults per section ----

#[test]
fn cfg_004_general_unpauses_capture_and_resets_language() {
    let f = Fixture::new();
    f.settings.update(|s| {
        s.capture.paused = true;
        s.ui.language = LanguagePreference::En;
        s.ui.theme = ThemeMode::Dark;
    });

    f.restore().execute(SettingsSection::General).unwrap();

    let s = f.stored();
    assert!(!s.capture.paused);
    assert_eq!(s.ui.language, LanguagePreference::System);
    assert_eq!(s.ui.theme, ThemeMode::Dark, "other sections are kept");
    assert_eq!(
        f.events.recorded(),
        vec![AppEvent::SettingsChanged, AppEvent::CaptureStateChanged]
    );
}

#[test]
fn cfg_004_history_restores_the_policy_and_applies_it() {
    let f = Fixture::new();
    f.settings.update(|s| {
        s.retention.policy = policy(RetentionPeriod::Forever, MaxItems::Unlimited);
        s.retention.clear_on_exit = true;
    });
    f.text("very old", 60);
    f.text("new", 0);

    let removed = f.restore().execute(SettingsSection::History).unwrap();

    assert_eq!(removed, 1);
    assert_eq!(f.stored().retention.policy, RetentionPolicy::default());
    assert!(!f.stored().retention.clear_on_exit);
    assert!(f.events.recorded().contains(&AppEvent::HistoryChanged));
}

#[test]
fn cfg_004_shortcuts_registers_the_default_open_hotkey() {
    let f = Fixture::new();
    let custom = Hotkey::parse("Alt+V").unwrap();
    f.settings.update(|s| {
        s.hotkeys.open = custom;
        s.snippets.restore_clipboard = false;
    });
    f.registrar
        .register(HotkeyOwner::OpenHistory, custom)
        .unwrap();

    f.restore().execute(SettingsSection::Shortcuts).unwrap();

    assert_eq!(f.stored().hotkeys.open, Hotkey::default_open());
    assert!(f.stored().snippets.restore_clipboard);
    assert_eq!(
        f.registrar.registered(HotkeyOwner::OpenHistory),
        Some(Hotkey::default_open())
    );
}

#[test]
fn cfg_004_shortcuts_keeps_everything_when_the_default_is_taken_by_a_snippet() {
    let f = Fixture::new();
    let custom = Hotkey::parse("Alt+V").unwrap();
    f.settings.update(|s| {
        s.hotkeys.open = custom;
        s.snippets.restore_clipboard = false;
    });
    f.snippets
        .insert(
            NewSnippet {
                name: SnippetName::parse("Teste").unwrap(),
                body: SnippetBody::new("x").unwrap(),
                hotkey: Some(Hotkey::default_open()),
                enabled: true,
            },
            Timestamp::from_millis(0),
        )
        .unwrap();

    let result = f.restore().execute(SettingsSection::Shortcuts);

    assert!(matches!(result, Err(AppError::HotkeyTaken(_))));
    assert_eq!(f.stored().hotkeys.open, custom);
    assert!(!f.stored().snippets.restore_clipboard);
}

#[test]
fn cfg_004_privacy_and_appearance_reset_their_own_settings() {
    let f = Fixture::new();
    f.settings.update(|s| {
        s.capture.excluded_apps = vec![app("keepass.exe")];
        s.ui.theme = ThemeMode::Light;
        s.ui.use_accent_color = true;
    });

    f.restore().execute(SettingsSection::Privacy).unwrap();
    assert_eq!(f.stored().capture.excluded_apps, vec![]);
    assert_eq!(f.stored().ui.theme, ThemeMode::Light);

    f.restore().execute(SettingsSection::Appearance).unwrap();
    assert_eq!(f.stored().ui.theme, ThemeMode::System);
    assert!(!f.stored().ui.use_accent_color);
}

// ---- CAP-006: size limits ----

#[test]
fn cap_006_offered_size_limits_are_saved() {
    let f = Fixture::new();
    let change = ChangeSizeLimit::new(f.settings.clone(), f.events.clone());

    change.execute(SizeLimit::Text, 5 * 1024 * 1024).unwrap();
    change.execute(SizeLimit::Image, 256 << 20).unwrap();

    assert_eq!(f.stored().capture.max_text_bytes, 5 * 1024 * 1024);
    assert_eq!(f.stored().capture.max_image_bytes, 256 << 20);
    assert_eq!(
        f.events.recorded(),
        vec![AppEvent::SettingsChanged, AppEvent::SettingsChanged]
    );
}

#[test]
fn cap_006_other_size_limits_are_rejected_without_saving() {
    let f = Fixture::new();

    let result =
        ChangeSizeLimit::new(f.settings.clone(), f.events.clone()).execute(SizeLimit::Text, 42);

    assert_eq!(
        result,
        Err(AppError::InvalidSetting(
            SettingsError::UnsupportedTextLimit(42)
        ))
    );
    assert_eq!(f.stored().capture.max_text_bytes, 1024 * 1024);
    assert_eq!(f.events.recorded(), vec![]);
}

// ---- CAP-009: excluded applications ----

#[test]
fn cap_009_excluding_normalizes_and_does_not_duplicate() {
    let f = Fixture::new();
    let exclude = ExcludeApp::new(f.settings.clone(), f.events.clone());

    assert_eq!(
        exclude.execute(r"C:\Apps\KeePass.EXE").unwrap(),
        app("keepass.exe")
    );
    exclude.execute("keepass.exe").unwrap();

    assert_eq!(f.stored().capture.excluded_apps, vec![app("keepass.exe")]);
}

#[test]
fn cap_009_blank_names_are_rejected() {
    let f = Fixture::new();

    let result = ExcludeApp::new(f.settings.clone(), f.events.clone()).execute("   ");

    assert!(matches!(result, Err(AppError::InvalidAppName(_))));
    assert_eq!(f.events.recorded(), vec![]);
}

#[test]
fn cap_009_including_removes_an_excluded_app() {
    let f = Fixture::new();
    f.settings
        .update(|s| s.capture.excluded_apps = vec![app("keepass.exe"), app("1password.exe")]);

    IncludeApp::new(f.settings.clone(), f.events.clone())
        .execute("KeePass.exe")
        .unwrap();

    assert_eq!(f.stored().capture.excluded_apps, vec![app("1password.exe")]);
    assert_eq!(f.events.recorded(), vec![AppEvent::SettingsChanged]);
}

#[test]
fn cap_009_candidates_are_sorted_unique_and_not_yet_excluded() {
    let f = Fixture::new();
    f.settings
        .update(|s| s.capture.excluded_apps = vec![app("keepass.exe")]);
    let running = Arc::new(FakeRunningApps::new(vec![
        app("notepad.exe"),
        app("code.exe"),
        app("keepass.exe"),
        app("notepad.exe"),
    ]));

    let candidates = ListExclusionCandidates::new(running, f.settings.clone())
        .execute()
        .unwrap();

    assert_eq!(candidates, vec![app("code.exe"), app("notepad.exe")]);
}

// ---- RET-006: clear on exit ----

#[test]
fn ret_006_clear_on_exit_removes_unpinned_items_when_enabled() {
    let f = Fixture::new();
    f.settings.update(|s| s.retention.clear_on_exit = true);
    f.text("a", 0);
    let pinned = f.history.insert(text_item("pinned", 100 * DAY_MS)).unwrap();
    f.history.set_pin_state(pinned, PinState::Pinned).unwrap();

    let removed = ClearOnExit::new(f.history.clone(), f.images.clone(), f.settings.clone())
        .execute()
        .unwrap();

    assert_eq!(removed, 1);
    assert_eq!(f.history.retention_candidates().unwrap().len(), 1);
}

#[test]
fn ret_006_nothing_is_cleared_when_disabled() {
    let f = Fixture::new();
    f.text("a", 0);

    let removed = ClearOnExit::new(f.history.clone(), f.images.clone(), f.settings.clone())
        .execute()
        .unwrap();

    assert_eq!(removed, 0);
    assert_eq!(f.history.retention_candidates().unwrap().len(), 1);
}
