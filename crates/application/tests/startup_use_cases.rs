//! Start with Windows and the welcome screen (CFG-003, TRAY-006).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use clipvault_application::ports::{AppEvent, SettingsRepository, StartupState};
use clipvault_application::use_cases::{CompleteWelcome, GetStartup, SetStartup, WelcomeChoice};
use clipvault_test_support::{FakeStartupManager, FixedSettings, RecordingEvents};

fn manager(state: StartupState) -> Arc<FakeStartupManager> {
    Arc::new(FakeStartupManager::new(state))
}

#[test]
fn cfg_003_reports_the_current_state() {
    let startup = manager(StartupState::DisabledByPolicy);

    assert_eq!(
        GetStartup::new(startup).execute().unwrap(),
        StartupState::DisabledByPolicy
    );
}

#[test]
fn cfg_003_enables_and_disables_when_the_user_may_change_it() {
    let startup = manager(StartupState::Disabled);
    let set = SetStartup::new(startup.clone());

    assert_eq!(set.execute(true).unwrap(), StartupState::Enabled);
    assert_eq!(set.execute(false).unwrap(), StartupState::Disabled);
    assert_eq!(startup.calls(), vec!["enable", "disable"]);
}

#[test]
fn cfg_003_asking_for_the_current_state_changes_nothing() {
    let startup = manager(StartupState::Enabled);

    assert_eq!(
        SetStartup::new(startup.clone()).execute(true).unwrap(),
        StartupState::Enabled
    );
    assert_eq!(startup.calls(), Vec::<&str>::new());
}

#[test]
fn cfg_003_states_locked_by_the_user_or_a_policy_are_never_overridden() {
    for locked in [
        StartupState::DisabledByUser,
        StartupState::DisabledByPolicy,
        StartupState::EnabledByPolicy,
    ] {
        let startup = manager(locked);
        let set = SetStartup::new(startup.clone());

        assert_eq!(set.execute(true).unwrap(), locked);
        assert_eq!(set.execute(false).unwrap(), locked);
        assert_eq!(startup.calls(), Vec::<&str>::new(), "{locked:?}");
    }
}

#[test]
fn cfg_003_state_helpers() {
    assert!(StartupState::Enabled.is_enabled());
    assert!(StartupState::EnabledByPolicy.is_enabled());
    assert!(!StartupState::DisabledByUser.is_enabled());
    assert!(StartupState::Disabled.can_change());
    assert!(!StartupState::DisabledByUser.can_change());
}

struct Welcome {
    settings: Arc<FixedSettings>,
    events: Arc<RecordingEvents>,
    startup: Arc<FakeStartupManager>,
}

impl Welcome {
    fn new() -> Self {
        Self {
            settings: Arc::new(FixedSettings::default()),
            events: Arc::new(RecordingEvents::default()),
            startup: manager(StartupState::Disabled),
        }
    }

    fn use_case(&self) -> CompleteWelcome {
        CompleteWelcome::new(
            self.settings.clone(),
            self.events.clone(),
            self.startup.clone(),
        )
    }
}

#[test]
fn tray_006_the_welcome_is_pending_only_on_the_first_run() {
    let w = Welcome::new();

    assert!(w.use_case().is_pending().unwrap());
    w.use_case().execute(WelcomeChoice::Dismissed).unwrap();

    assert!(!w.use_case().is_pending().unwrap());
    assert!(w.settings.load().unwrap().general.first_run_done);
    assert_eq!(w.events.recorded(), vec![AppEvent::SettingsChanged]);
}

#[test]
fn tray_006_confirming_with_startup_enables_it() {
    let w = Welcome::new();

    let state = w
        .use_case()
        .execute(WelcomeChoice::Confirmed {
            start_with_windows: true,
        })
        .unwrap();

    assert_eq!(state, StartupState::Enabled);
    assert!(!w.use_case().is_pending().unwrap());
}

#[test]
fn tray_006_unchecking_or_dismissing_leaves_startup_off() {
    for choice in [
        WelcomeChoice::Confirmed {
            start_with_windows: false,
        },
        WelcomeChoice::Dismissed,
    ] {
        let w = Welcome::new();

        assert_eq!(
            w.use_case().execute(choice).unwrap(),
            StartupState::Disabled
        );
        assert_eq!(w.startup.calls(), Vec::<&str>::new());
    }
}
