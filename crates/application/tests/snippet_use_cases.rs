//! Snippets and global shortcuts (SNP-001..006, HK-001..004).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use clipvault_application::AppError;
use clipvault_application::ports::{
    HotkeyRegistrar, PortError, SettingsRepository, SnippetRepository,
};
use clipvault_application::use_cases::{
    ApplyHotkeyBindings, ChangeOpenHotkey, DeleteSnippet, HotkeyConflict, ListSnippets,
    SaveSnippet, SnippetDependencies, SnippetDraft, SuspendHotkeys, Suspension,
};
use clipvault_domain::hotkey::{Hotkey, HotkeyError, HotkeyOwner};
use clipvault_domain::ids::SnippetId;
use clipvault_domain::snippet::SnippetError;
use clipvault_domain::time::Timestamp;
use clipvault_test_support::{
    FakeHotkeyRegistrar, FixedClock, FixedSettings, InMemorySnippetRepository,
};

struct Fixture {
    snippets: Arc<InMemorySnippetRepository>,
    settings: Arc<FixedSettings>,
    registrar: Arc<FakeHotkeyRegistrar>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            snippets: Arc::new(InMemorySnippetRepository::default()),
            settings: Arc::new(FixedSettings::default()),
            registrar: Arc::new(FakeHotkeyRegistrar::default()),
        }
    }

    fn deps(&self) -> SnippetDependencies {
        SnippetDependencies {
            snippets: self.snippets.clone(),
            settings: self.settings.clone(),
            registrar: self.registrar.clone(),
            clock: Arc::new(FixedClock::at(Timestamp::from_millis(1_000))),
        }
    }

    fn save(&self, draft: SnippetDraft) -> Result<SnippetId, AppError> {
        SaveSnippet::new(self.deps()).execute(draft).map(|s| s.id)
    }
}

fn draft(name: &str, body: &str, hotkey: Option<&str>) -> SnippetDraft {
    SnippetDraft {
        id: None,
        name: name.into(),
        body: body.into(),
        hotkey: hotkey.map(Into::into),
        enabled: true,
    }
}

fn hk(text: &str) -> Hotkey {
    Hotkey::parse(text).unwrap()
}

#[test]
fn snp_001_a_valid_snippet_is_saved_and_its_shortcut_registered() {
    let f = Fixture::new();

    let id = f
        .save(draft("Teste", "Teste de texto", Some("ctrl+shift+v")))
        .unwrap();

    let saved = f.snippets.get(id).unwrap().unwrap();
    assert_eq!(saved.name.as_str(), "Teste");
    assert_eq!(saved.hotkey, Some(hk("Ctrl+Shift+V")));
    assert_eq!(
        f.registrar.registered(HotkeyOwner::Snippet(id)),
        Some(hk("Ctrl+Shift+V"))
    );
}

#[test]
fn snp_001_invalid_fields_are_rejected_without_saving() {
    let f = Fixture::new();

    assert_eq!(
        f.save(draft(" ", "x", None)),
        Err(AppError::InvalidSnippet(SnippetError::NameBlank))
    );
    assert_eq!(
        f.save(draft("A", "  ", None)),
        Err(AppError::InvalidSnippet(SnippetError::BodyBlank))
    );
    assert_eq!(
        f.save(draft("A", "x", Some("Ctrl+C"))),
        Err(AppError::InvalidHotkey(HotkeyError::Reserved))
    );
    assert_eq!(f.snippets.list().unwrap(), vec![]);
}

#[test]
fn snp_001_names_must_be_unique_ignoring_case() {
    let f = Fixture::new();
    f.save(draft("Assinatura", "a", None)).unwrap();

    assert_eq!(
        f.save(draft("ASSINATURA", "b", None)),
        Err(AppError::NameTaken)
    );
}

#[test]
fn snp_002_many_snippets_can_have_their_own_shortcuts() {
    let f = Fixture::new();

    for n in 1..=9 {
        f.save(draft(
            &format!("S{n}"),
            "x",
            Some(&format!("Ctrl+Shift+{n}")),
        ))
        .unwrap();
    }

    assert_eq!(
        ListSnippets::new(f.snippets.clone())
            .execute()
            .unwrap()
            .len(),
        9
    );
    assert_eq!(f.registrar.count(), 9);
}

#[test]
fn snp_006_a_shortcut_used_by_another_snippet_is_rejected_with_its_name() {
    let f = Fixture::new();
    f.save(draft("Assinatura", "a", Some("Ctrl+Shift+1")))
        .unwrap();

    let result = f.save(draft("Outro", "b", Some("shift+ctrl+1")));

    assert_eq!(
        result,
        Err(AppError::HotkeyTaken(HotkeyConflict::Snippet(
            "Assinatura".into()
        )))
    );
}

#[test]
fn hk_003_a_snippet_cannot_take_the_open_history_shortcut() {
    let f = Fixture::new();

    let result = f.save(draft("X", "x", Some("Ctrl+Win+V")));

    assert_eq!(
        result,
        Err(AppError::HotkeyTaken(HotkeyConflict::OpenHistory))
    );
}

#[test]
fn hk_004_a_shortcut_used_by_another_app_is_not_saved() {
    let f = Fixture::new();
    f.registrar.block(hk("Ctrl+Shift+9"));

    let result = f.save(draft("X", "x", Some("Ctrl+Shift+9")));

    assert!(matches!(result, Err(AppError::Port(PortError::InUse(_)))));
    assert_eq!(f.snippets.list().unwrap(), vec![]);
}

#[test]
fn hk_004_editing_into_a_blocked_shortcut_keeps_the_previous_one() {
    let f = Fixture::new();
    let id = f.save(draft("X", "x", Some("Ctrl+Shift+1"))).unwrap();
    f.registrar.block(hk("Ctrl+Shift+9"));

    let mut edit = draft("X", "novo texto", Some("Ctrl+Shift+9"));
    edit.id = Some(id);
    let result = f.save(edit);

    assert!(matches!(result, Err(AppError::Port(PortError::InUse(_)))));
    let kept = f.snippets.get(id).unwrap().unwrap();
    assert_eq!(
        (kept.body.as_str(), kept.hotkey),
        ("x", Some(hk("Ctrl+Shift+1")))
    );
    assert_eq!(
        f.registrar.registered(HotkeyOwner::Snippet(id)),
        Some(hk("Ctrl+Shift+1"))
    );
}

#[test]
fn snp_006_editing_a_snippet_keeps_its_own_shortcut_valid() {
    let f = Fixture::new();
    let id = f.save(draft("X", "x", Some("Ctrl+Shift+1"))).unwrap();

    let mut edit = draft("X renomeado", "y", Some("Ctrl+Shift+1"));
    edit.id = Some(id);

    assert_eq!(f.save(edit), Ok(id));
    assert_eq!(
        f.snippets.get(id).unwrap().unwrap().name.as_str(),
        "X renomeado"
    );
}

#[test]
fn snp_001_disabled_snippets_or_without_shortcut_are_not_registered() {
    let f = Fixture::new();
    let mut disabled = draft("Off", "x", Some("Ctrl+Shift+2"));
    disabled.enabled = false;

    let off = f.save(disabled).unwrap();
    let none = f.save(draft("Sem atalho", "x", None)).unwrap();

    assert_eq!(f.registrar.registered(HotkeyOwner::Snippet(off)), None);
    assert_eq!(f.registrar.registered(HotkeyOwner::Snippet(none)), None);
}

#[test]
fn snp_006_disabling_a_snippet_unregisters_its_shortcut() {
    let f = Fixture::new();
    let id = f.save(draft("X", "x", Some("Ctrl+Shift+1"))).unwrap();

    let mut edit = draft("X", "x", Some("Ctrl+Shift+1"));
    edit.id = Some(id);
    edit.enabled = false;
    f.save(edit).unwrap();

    assert_eq!(f.registrar.registered(HotkeyOwner::Snippet(id)), None);
}

#[test]
fn snp_006_deleting_a_snippet_unregisters_its_shortcut() {
    let f = Fixture::new();
    let id = f.save(draft("X", "x", Some("Ctrl+Shift+1"))).unwrap();

    DeleteSnippet::new(f.deps()).execute(id).unwrap();

    assert_eq!(f.snippets.get(id).unwrap(), None);
    assert_eq!(f.registrar.registered(HotkeyOwner::Snippet(id)), None);
}

#[test]
fn saving_an_unknown_snippet_id_fails() {
    let f = Fixture::new();
    let mut edit = draft("X", "x", None);
    edit.id = Some(SnippetId::new(99));

    assert_eq!(f.save(edit), Err(AppError::Port(PortError::NotFound)));
}

#[test]
fn hk_001_the_open_shortcut_can_be_changed_and_is_registered() {
    let f = Fixture::new();
    ApplyHotkeyBindings::new(f.deps()).execute().unwrap();

    ChangeOpenHotkey::new(f.deps())
        .execute("ctrl+shift+space")
        .unwrap();

    assert_eq!(
        f.settings.load().unwrap().hotkeys.open,
        hk("Ctrl+Shift+Space")
    );
    assert_eq!(
        f.registrar.registered(HotkeyOwner::OpenHistory),
        Some(hk("Ctrl+Shift+Space"))
    );
}

#[test]
fn hk_003_the_open_shortcut_cannot_take_a_snippet_shortcut() {
    let f = Fixture::new();
    f.save(draft("Assinatura", "a", Some("Ctrl+Shift+1")))
        .unwrap();

    let result = ChangeOpenHotkey::new(f.deps()).execute("Ctrl+Shift+1");

    assert_eq!(
        result,
        Err(AppError::HotkeyTaken(HotkeyConflict::Snippet(
            "Assinatura".into()
        )))
    );
}

#[test]
fn hk_004_a_blocked_open_shortcut_keeps_the_previous_one() {
    let f = Fixture::new();
    ApplyHotkeyBindings::new(f.deps()).execute().unwrap();
    f.registrar.block(hk("Alt+V"));

    let result = ChangeOpenHotkey::new(f.deps()).execute("Alt+V");

    assert!(matches!(result, Err(AppError::Port(PortError::InUse(_)))));
    assert_eq!(
        f.settings.load().unwrap().hotkeys.open,
        Hotkey::default_open()
    );
    assert_eq!(
        f.registrar.registered(HotkeyOwner::OpenHistory),
        Some(Hotkey::default_open())
    );
}

#[test]
fn hk_003_invalid_open_shortcuts_are_rejected() {
    let f = Fixture::new();

    assert_eq!(
        ChangeOpenHotkey::new(f.deps()).execute("V"),
        Err(AppError::InvalidHotkey(HotkeyError::MissingModifier))
    );
}

#[test]
fn hk_001_startup_registers_the_open_shortcut_and_enabled_snippets() {
    let f = Fixture::new();
    let on = f
        .snippets
        .insert_for_tests("On", Some(hk("Ctrl+Shift+1")), true);
    let off = f
        .snippets
        .insert_for_tests("Off", Some(hk("Ctrl+Shift+2")), false);

    let failures = ApplyHotkeyBindings::new(f.deps()).execute().unwrap();

    assert_eq!(failures, vec![]);
    assert_eq!(
        f.registrar.registered(HotkeyOwner::OpenHistory),
        Some(Hotkey::default_open())
    );
    assert_eq!(
        f.registrar.registered(HotkeyOwner::Snippet(on)),
        Some(hk("Ctrl+Shift+1"))
    );
    assert_eq!(f.registrar.registered(HotkeyOwner::Snippet(off)), None);
}

#[test]
fn hk_004_startup_reports_shortcuts_taken_by_other_apps() {
    let f = Fixture::new();
    f.registrar.block(Hotkey::default_open());

    let failures = ApplyHotkeyBindings::new(f.deps()).execute().unwrap();

    assert_eq!(failures, vec![HotkeyOwner::OpenHistory]);
}

#[test]
fn hk_002_shortcuts_can_be_suspended_while_recording_a_new_one() {
    let f = Fixture::new();

    SuspendHotkeys::new(f.registrar.clone())
        .execute(Suspension::Suspended)
        .unwrap();
    assert!(f.registrar.is_suspended());
    SuspendHotkeys::new(f.registrar.clone())
        .execute(Suspension::Active)
        .unwrap();
    assert!(!f.registrar.is_suspended());
}

#[test]
fn the_registrar_port_is_object_safe() {
    let registrar: Arc<dyn HotkeyRegistrar> = Arc::new(FakeHotkeyRegistrar::default());
    registrar.unregister(HotkeyOwner::OpenHistory).unwrap();
}
