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
