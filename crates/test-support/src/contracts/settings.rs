//! Contract of [`SettingsRepository`] (CFG-002, CFG-006).

use std::time::Duration;

use clipvault_application::ports::SettingsRepository;
use clipvault_domain::app_name::AppName;
use clipvault_domain::hotkey::Hotkey;
use clipvault_domain::retention::{MaxItems, RetentionPeriod};
use clipvault_domain::settings::{LanguagePreference, Settings, ThemeMode};

/// A settings value different from the defaults in every field.
pub fn customized() -> Settings {
    let mut s = Settings::default();
    s.capture.paused = true;
    s.capture.max_text_bytes = 2_048;
    s.capture.max_image_bytes = 4_096;
    s.capture.excluded_apps = vec![AppName::parse("keepass.exe").unwrap()];
    s.retention.policy.period = RetentionPeriod::Forever;
    s.retention.policy.max_items = MaxItems::FiveThousand;
    s.retention.clear_on_exit = true;
    s.hotkeys.open = Hotkey::parse("Ctrl+Shift+Space").unwrap();
    s.snippets.restore_clipboard = false;
    s.snippets.restore_delay = Duration::from_millis(750);
    s.ui.theme = ThemeMode::Dark;
    s.ui.language = LanguagePreference::PtBr;
    s.ui.use_accent_color = true;
    s.general.first_run_done = true;
    s
}

/// A fresh store returns the defaults.
pub fn empty_store_returns_defaults(repo: &dyn SettingsRepository) {
    assert_eq!(repo.load().unwrap(), Settings::default());
}

/// CFG-002: saved settings are loaded back unchanged.
pub fn cfg_002_saved_settings_round_trip(repo: &dyn SettingsRepository) {
    repo.save(&customized()).unwrap();

    assert_eq!(repo.load().unwrap(), customized());
}

/// Saving again overwrites the previous values.
pub fn saving_again_overwrites(repo: &dyn SettingsRepository) {
    repo.save(&customized()).unwrap();
    repo.save(&Settings::default()).unwrap();

    assert_eq!(repo.load().unwrap(), Settings::default());
}

/// Generates one `#[test]` per [`SettingsRepository`] contract case.
#[macro_export]
macro_rules! settings_repository_contract {
    ($make:expr) => {
        $crate::__contract_tests!(
            settings,
            $make,
            empty_store_returns_defaults,
            cfg_002_saved_settings_round_trip,
            saving_again_overwrites,
        );
    };
}
