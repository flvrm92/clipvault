//! Pasting snippets (SNP-003..005, SNP-007, CAP-010).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use clipvault_application::AppError;
use clipvault_application::capture::OwnWriteRegistry;
use clipvault_application::ports::{
    ClipboardContent, ClipboardSequence, PortError, ScreenPoint, TargetWindow, WindowHandle,
};
use clipvault_application::use_cases::{
    FallbackReason, PasteOutcome, PasteSnippet, PasteSnippetDependencies, SnippetTarget,
};
use clipvault_domain::hotkey::Hotkey;
use clipvault_domain::ids::SnippetId;
use clipvault_test_support::snapshot::{self, pixels};
use clipvault_test_support::{
    FakeClipboardReader, FakeFocusTracker, FixedSettings, InMemorySnippetRepository,
    RecordingClipboardWriter, RecordingDelay, RecordingKeystrokes,
};

struct Fixture {
    snippets: Arc<InMemorySnippetRepository>,
    reader: Arc<FakeClipboardReader>,
    writer: Arc<RecordingClipboardWriter>,
    focus: Arc<FakeFocusTracker>,
    keys: Arc<RecordingKeystrokes>,
    own_writes: Arc<OwnWriteRegistry>,
    settings: Arc<FixedSettings>,
    delay: Arc<RecordingDelay>,
    id: SnippetId,
}

impl Fixture {
    fn new() -> Self {
        let snippets = Arc::new(InMemorySnippetRepository::default());
        let id = snippets.insert_body_for_tests("Teste", "Teste de texto");
        let reader = Arc::new(FakeClipboardReader::default());
        reader.set(snapshot::text("anterior").sequence(1).build());
        Self {
            snippets,
            reader,
            writer: Arc::new(RecordingClipboardWriter::default()),
            focus: Arc::new(FakeFocusTracker::default()),
            keys: Arc::new(RecordingKeystrokes::default()),
            own_writes: Arc::new(OwnWriteRegistry::default()),
            settings: Arc::new(FixedSettings::default()),
            delay: Arc::new(RecordingDelay::default()),
            id,
        }
    }

    fn paste(&self, target: SnippetTarget) -> Result<PasteOutcome, AppError> {
        PasteSnippet::new(PasteSnippetDependencies {
            snippets: self.snippets.clone(),
            reader: self.reader.clone(),
            writer: self.writer.clone(),
            focus: self.focus.clone(),
            keys: self.keys.clone(),
            own_writes: self.own_writes.clone(),
            settings: self.settings.clone(),
            delay: self.delay.clone(),
        })
        .execute(self.id, target)
    }
}

fn window(elevated: bool) -> TargetWindow {
    TargetWindow {
        handle: WindowHandle::new(7),
        caret: None,
        cursor: ScreenPoint::default(),
        elevated,
    }
}

#[test]
fn snp_003_the_snippet_is_pasted_into_the_foreground_window() {
    let f = Fixture::new();

    let outcome = f.paste(SnippetTarget::Foreground).unwrap();

    assert_eq!(outcome, PasteOutcome::Pasted);
    assert_eq!(
        f.writer.writes()[0],
        ClipboardContent::Text("Teste de texto".into())
    );
    assert_eq!(f.keys.pastes(), 1);
    assert_eq!(
        f.focus.restored(),
        vec![],
        "the foreground window already has focus"
    );
}

#[test]
fn snp_005_the_previous_clipboard_content_is_restored_after_the_delay() {
    let f = Fixture::new();

    f.paste(SnippetTarget::Foreground).unwrap();

    assert_eq!(
        f.writer.writes(),
        vec![
            ClipboardContent::Text("Teste de texto".into()),
            ClipboardContent::Text("anterior".into()),
        ]
    );
    assert_eq!(f.delay.waits(), vec![Duration::from_millis(500)]);
}

#[test]
fn snp_005_previous_images_are_restored_too() {
    let f = Fixture::new();
    f.reader.set(snapshot::empty().image(2, 2).build());

    f.paste(SnippetTarget::Foreground).unwrap();

    assert_eq!(f.writer.writes()[1], ClipboardContent::Image(pixels(2, 2)));
}

#[test]
fn snp_005_restoring_can_be_turned_off() {
    let f = Fixture::new();
    f.settings.update(|s| s.snippets.restore_clipboard = false);

    f.paste(SnippetTarget::Foreground).unwrap();

    assert_eq!(f.writer.writes().len(), 1);
    assert_eq!(f.delay.waits(), vec![]);
}

#[test]
fn snp_005_the_configured_delay_is_used() {
    let f = Fixture::new();
    f.settings
        .update(|s| s.snippets.restore_delay = Duration::from_millis(750));

    f.paste(SnippetTarget::Foreground).unwrap();

    assert_eq!(f.delay.waits(), vec![Duration::from_millis(750)]);
}

#[test]
fn snp_005_nothing_is_restored_when_the_clipboard_could_not_be_read() {
    let f = Fixture::new();
    f.reader.fail_next();

    f.paste(SnippetTarget::Foreground).unwrap();

    assert_eq!(f.writer.writes().len(), 1);
}

#[test]
fn cap_010_snippet_writes_and_restores_are_registered_as_our_own() {
    let f = Fixture::new();
    f.writer.next_sequence(40);

    f.paste(SnippetTarget::Foreground).unwrap();

    // The fake returns sequence 40 for both writes: snippet and restore are each recorded.
    assert!(f.own_writes.take(ClipboardSequence::new(40)));
    assert!(f.own_writes.take(ClipboardSequence::new(40)));
    assert!(!f.own_writes.take(ClipboardSequence::new(40)));
}

#[test]
fn pst_006_elevated_foreground_windows_get_the_snippet_on_the_clipboard_only() {
    let f = Fixture::new();
    f.focus.set_target(Some(window(true)));

    let outcome = f.paste(SnippetTarget::Foreground).unwrap();

    assert_eq!(
        outcome,
        PasteOutcome::CopiedOnly(FallbackReason::TargetElevated)
    );
    assert_eq!(f.keys.pastes(), 0);
    assert_eq!(
        f.writer.writes().len(),
        1,
        "no restore: the user must paste manually"
    );
}

#[test]
fn snp_007_from_the_popup_focus_returns_to_the_target_first() {
    let f = Fixture::new();

    let outcome = f.paste(SnippetTarget::Window(Some(window(false)))).unwrap();

    assert_eq!(outcome, PasteOutcome::Pasted);
    assert_eq!(f.focus.restored(), vec![WindowHandle::new(7)]);
}

#[test]
fn snp_007_from_the_popup_without_target_the_snippet_is_only_copied() {
    let f = Fixture::new();

    let outcome = f.paste(SnippetTarget::Window(None)).unwrap();

    assert_eq!(outcome, PasteOutcome::CopiedOnly(FallbackReason::NoTarget));
    assert_eq!(f.writer.writes().len(), 1);
}

#[test]
fn a_failed_keystroke_keeps_the_snippet_on_the_clipboard() {
    let f = Fixture::new();
    f.keys.fail_next();

    let outcome = f.paste(SnippetTarget::Foreground).unwrap();

    assert_eq!(
        outcome,
        PasteOutcome::CopiedOnly(FallbackReason::TargetUnavailable)
    );
    assert_eq!(f.writer.writes().len(), 1);
}

#[test]
fn an_unknown_snippet_is_not_found() {
    let mut f = Fixture::new();
    f.id = SnippetId::new(404);

    assert_eq!(
        f.paste(SnippetTarget::Foreground),
        Err(AppError::Port(PortError::NotFound))
    );
    assert_eq!(f.writer.writes(), vec![]);
}

#[test]
fn snp_003_disabled_snippets_still_paste_from_the_popup_list() {
    let f = Fixture::new();
    f.snippets.set_enabled_for_tests(f.id, false);
    let _ = Hotkey::default_open();

    assert_eq!(
        f.paste(SnippetTarget::Foreground).unwrap(),
        PasteOutcome::Pasted
    );
}
