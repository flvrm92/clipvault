use std::time::Duration;

use super::*;
use crate::app_name::AppName;
use crate::hotkey::Hotkey;
use crate::retention::{MaxItems, RetentionPeriod};

/// Defaults from specs/05 §3.
#[test]
fn defaults_match_the_specification() {
    let s = Settings::default();

    assert!(!s.capture.paused);
    assert_eq!(s.capture.max_text_bytes, 1_048_576);
    assert_eq!(s.capture.max_image_bytes, 67_108_864);
    assert_eq!(s.capture.excluded_apps, vec![]);
    assert_eq!(s.retention.policy.period, RetentionPeriod::ThirtyDays);
    assert_eq!(s.retention.policy.max_items, MaxItems::Thousand);
    assert!(!s.retention.clear_on_exit);
    assert_eq!(s.hotkeys.open, Hotkey::default_open());
    assert!(s.snippets.restore_clipboard);
    assert_eq!(s.snippets.restore_delay, Duration::from_millis(500));
    assert_eq!(s.ui.theme, ThemeMode::System);
    assert_eq!(s.ui.language, LanguagePreference::System);
    assert!(!s.ui.use_accent_color);
    assert!(!s.general.first_run_done);
}

#[test]
fn cap_009_excluded_apps_match_normalized_names() {
    let capture = CaptureSettings {
        excluded_apps: vec![AppName::parse("KeePass.exe").unwrap()],
        ..CaptureSettings::default()
    };

    assert!(capture.is_excluded(&AppName::parse(r"C:\Tools\KEEPASS.EXE").unwrap()));
    assert!(!capture.is_excluded(&AppName::parse("notepad.exe").unwrap()));
}

fn customized() -> Settings {
    let mut s = Settings::default();
    s.capture.paused = true;
    s.capture.max_text_bytes = 256 * 1024;
    s.capture.max_image_bytes = 16 * 1024 * 1024;
    s.capture.excluded_apps = vec![AppName::parse("keepass.exe").unwrap()];
    s.retention.policy.period = RetentionPeriod::OneDay;
    s.retention.policy.max_items = MaxItems::Hundred;
    s.retention.clear_on_exit = true;
    s.hotkeys.open = Hotkey::parse("Alt+V").unwrap();
    s.snippets.restore_clipboard = false;
    s.snippets.restore_delay = Duration::from_millis(900);
    s.ui.theme = ThemeMode::Dark;
    s.ui.language = LanguagePreference::En;
    s.ui.use_accent_color = true;
    s.general.first_run_done = true;
    s
}

#[test]
fn cfg_004_general_restores_language_and_capture_only() {
    let mut s = customized();

    s.restore_defaults(SettingsSection::General);

    let mut expected = customized();
    expected.ui.language = LanguagePreference::System;
    expected.capture.paused = false;
    assert_eq!(s, expected);
}

#[test]
fn cfg_004_history_restores_retention_and_size_limits_only() {
    let mut s = customized();

    s.restore_defaults(SettingsSection::History);

    let mut expected = customized();
    expected.retention = RetentionSettings::default();
    expected.capture.max_text_bytes = CaptureSettings::default().max_text_bytes;
    expected.capture.max_image_bytes = CaptureSettings::default().max_image_bytes;
    assert_eq!(s, expected);
}

#[test]
fn cfg_004_shortcuts_restores_open_hotkey_and_snippet_behaviour_only() {
    let mut s = customized();

    s.restore_defaults(SettingsSection::Shortcuts);

    let mut expected = customized();
    expected.hotkeys = HotkeySettings::default();
    expected.snippets = SnippetSettings::default();
    assert_eq!(s, expected);
}

#[test]
fn cfg_004_privacy_clears_the_excluded_apps_only() {
    let mut s = customized();

    s.restore_defaults(SettingsSection::Privacy);

    let mut expected = customized();
    expected.capture.excluded_apps.clear();
    assert_eq!(s, expected);
}

#[test]
fn cfg_004_appearance_restores_theme_and_accent_only() {
    let mut s = customized();

    s.restore_defaults(SettingsSection::Appearance);

    let mut expected = customized();
    expected.ui.theme = ThemeMode::System;
    expected.ui.use_accent_color = false;
    assert_eq!(s, expected);
}

#[test]
fn cap_006_size_limits_accept_only_the_offered_options() {
    let mut capture = CaptureSettings::default();

    for option in CaptureSettings::TEXT_LIMIT_OPTIONS {
        assert_eq!(capture.set_max_text_bytes(option), Ok(()));
        assert_eq!(capture.max_text_bytes, option);
    }
    for option in CaptureSettings::IMAGE_LIMIT_OPTIONS {
        assert_eq!(capture.set_max_image_bytes(option), Ok(()));
        assert_eq!(capture.max_image_bytes, option);
    }

    assert_eq!(
        capture.set_max_text_bytes(1234),
        Err(SettingsError::UnsupportedTextLimit(1234))
    );
    assert_eq!(
        capture.set_max_image_bytes(0),
        Err(SettingsError::UnsupportedImageLimit(0))
    );
    assert_eq!(
        capture.max_image_bytes,
        *CaptureSettings::IMAGE_LIMIT_OPTIONS.last().unwrap()
    );
}

#[test]
fn cap_006_options_include_the_defaults() {
    let defaults = CaptureSettings::default();

    assert_eq!(
        CaptureSettings::TEXT_LIMIT_OPTIONS,
        [256 * 1024, 1024 * 1024, 5 * 1024 * 1024, 10 * 1024 * 1024]
    );
    assert_eq!(
        CaptureSettings::IMAGE_LIMIT_OPTIONS,
        [16 << 20, 64 << 20, 128 << 20, 256 << 20]
    );
    assert!(CaptureSettings::TEXT_LIMIT_OPTIONS.contains(&defaults.max_text_bytes));
    assert!(CaptureSettings::IMAGE_LIMIT_OPTIONS.contains(&defaults.max_image_bytes));
}

#[test]
fn cap_009_excluding_an_app_twice_keeps_one_entry() {
    let mut capture = CaptureSettings::default();

    assert!(capture.exclude(AppName::parse("KeePass.exe").unwrap()));
    assert!(!capture.exclude(AppName::parse(r"C:\x\keepass.EXE").unwrap()));

    assert_eq!(
        capture.excluded_apps,
        vec![AppName::parse("keepass.exe").unwrap()]
    );
}

#[test]
fn cap_009_including_an_app_removes_it() {
    let keepass = AppName::parse("keepass.exe").unwrap();
    let mut capture = CaptureSettings {
        excluded_apps: vec![keepass.clone(), AppName::parse("1password.exe").unwrap()],
        ..CaptureSettings::default()
    };

    assert!(capture.include(&keepass));
    assert!(!capture.include(&keepass));

    assert_eq!(
        capture.excluded_apps,
        vec![AppName::parse("1password.exe").unwrap()]
    );
}
