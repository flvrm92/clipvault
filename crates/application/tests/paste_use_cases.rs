//! Pasting and copying history items (PST-001..006, HIST-011, CAP-010) and capture state (CAP-008).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use clipvault_application::AppError;
use clipvault_application::capture::OwnWriteRegistry;
use clipvault_application::ports::{
    AppEvent, ClipboardContent, ClipboardSequence, HistoryRepository, ImageStore, PortError,
    ScreenPoint, SettingsRepository, TargetWindow, WindowHandle,
};
use clipvault_application::use_cases::{
    CaptureState, CopyItem, FallbackReason, GetItem, PasteDependencies, PasteItem, PasteMode,
    PasteOutcome, SetCaptureState,
};
use clipvault_domain::clip::ClipContent;
use clipvault_domain::ids::ClipId;
use clipvault_domain::time::Timestamp;
use clipvault_test_support::contracts::history::{image_item, text_item};
use clipvault_test_support::snapshot::pixels;
use clipvault_test_support::{
    FakeFocusTracker, FixedClock, FixedSettings, InMemoryHistoryRepository, InMemoryImageStore,
    RecordingClipboardWriter, RecordingEvents, RecordingKeystrokes,
};

fn target(elevated: bool) -> TargetWindow {
    TargetWindow {
        handle: WindowHandle::new(42),
        caret: Some(ScreenPoint { x: 10, y: 20 }),
        cursor: ScreenPoint { x: 30, y: 40 },
        elevated,
    }
}

struct Fixture {
    history: Arc<InMemoryHistoryRepository>,
    images: Arc<InMemoryImageStore>,
    writer: Arc<RecordingClipboardWriter>,
    focus: Arc<FakeFocusTracker>,
    keys: Arc<RecordingKeystrokes>,
    own_writes: Arc<OwnWriteRegistry>,
    events: Arc<RecordingEvents>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            history: Arc::new(InMemoryHistoryRepository::default()),
            images: Arc::new(InMemoryImageStore::default()),
            writer: Arc::new(RecordingClipboardWriter::default()),
            focus: Arc::new(FakeFocusTracker::default()),
            keys: Arc::new(RecordingKeystrokes::default()),
            own_writes: Arc::new(OwnWriteRegistry::default()),
            events: Arc::new(RecordingEvents::default()),
        }
    }

    fn deps(&self) -> PasteDependencies {
        PasteDependencies {
            history: self.history.clone(),
            images: self.images.clone(),
            writer: self.writer.clone(),
            focus: self.focus.clone(),
            keys: self.keys.clone(),
            own_writes: self.own_writes.clone(),
            clock: Arc::new(FixedClock::at(Timestamp::from_millis(9_000))),
            events: self.events.clone(),
        }
    }

    fn paste(
        &self,
        id: ClipId,
        mode: PasteMode,
        target: Option<&TargetWindow>,
    ) -> Result<PasteOutcome, AppError> {
        PasteItem::new(self.deps()).execute(id, mode, target)
    }
}

#[test]
fn pst_001_pasting_writes_the_clipboard_restores_focus_and_sends_ctrl_v() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("relatório final", 1)).unwrap();

    let outcome = f.paste(id, PasteMode::AsIs, Some(&target(false))).unwrap();

    assert_eq!(outcome, PasteOutcome::Pasted);
    assert_eq!(
        f.writer.writes(),
        vec![ClipboardContent::Text("relatório final".into())]
    );
    assert_eq!(f.focus.restored(), vec![WindowHandle::new(42)]);
    assert_eq!(f.keys.pastes(), 1);
}

#[test]
fn pst_001_the_pasted_item_moves_to_the_top() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("a", 1)).unwrap();
    f.history.insert(text_item("b", 2)).unwrap();

    f.paste(id, PasteMode::AsIs, Some(&target(false))).unwrap();

    let item = f.history.get(id).unwrap();
    assert_eq!(
        (item.last_used_at, item.use_count),
        (Timestamp::from_millis(9_000), 2)
    );
    assert_eq!(f.events.recorded(), vec![AppEvent::HistoryChanged]);
}

#[test]
fn cap_010_the_clipboard_write_is_registered_as_our_own() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("a", 1)).unwrap();
    f.writer.next_sequence(77);

    f.paste(id, PasteMode::AsIs, Some(&target(false))).unwrap();

    assert!(f.own_writes.take(ClipboardSequence::new(77)));
}

#[test]
fn pst_002_trimmed_mode_removes_surrounding_whitespace() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("  12345-678 \r\n", 1)).unwrap();

    f.paste(id, PasteMode::TrimmedText, Some(&target(false)))
        .unwrap();

    assert_eq!(
        f.writer.writes(),
        vec![ClipboardContent::Text("12345-678".into())]
    );
}

#[test]
fn pst_003_images_are_loaded_from_disk_and_written_as_images() {
    let f = Fixture::new();
    let item = image_item("print", 1);
    let ClipContent::Image(meta) = &item.content else {
        unreachable!()
    };
    f.images.save(&meta.file_key, &pixels(3, 2)).unwrap();
    let id = f.history.insert(item.clone()).unwrap();

    f.paste(id, PasteMode::AsIs, Some(&target(false))).unwrap();

    assert_eq!(
        f.writer.writes(),
        vec![ClipboardContent::Image(pixels(3, 2))]
    );
}

#[test]
fn pst_005_when_focus_cannot_be_restored_the_content_stays_on_the_clipboard() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("a", 1)).unwrap();
    f.focus.fail_restore();

    let outcome = f.paste(id, PasteMode::AsIs, Some(&target(false))).unwrap();

    assert_eq!(
        outcome,
        PasteOutcome::CopiedOnly(FallbackReason::TargetUnavailable)
    );
    assert_eq!(f.writer.writes().len(), 1);
    assert_eq!(f.keys.pastes(), 0);
}

#[test]
fn pst_005_without_a_target_window_the_content_is_only_copied() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("a", 1)).unwrap();

    let outcome = f.paste(id, PasteMode::AsIs, None).unwrap();

    assert_eq!(outcome, PasteOutcome::CopiedOnly(FallbackReason::NoTarget));
    assert_eq!(f.keys.pastes(), 0);
}

#[test]
fn pst_006_elevated_targets_are_not_sent_keystrokes() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("a", 1)).unwrap();

    let outcome = f.paste(id, PasteMode::AsIs, Some(&target(true))).unwrap();

    assert_eq!(
        outcome,
        PasteOutcome::CopiedOnly(FallbackReason::TargetElevated)
    );
    assert_eq!(f.focus.restored(), vec![]);
    assert_eq!(f.keys.pastes(), 0);
}

#[test]
fn pasting_a_missing_item_fails_without_touching_the_clipboard() {
    let f = Fixture::new();

    let result = f.paste(ClipId::new(5), PasteMode::AsIs, Some(&target(false)));

    assert_eq!(result, Err(AppError::Port(PortError::NotFound)));
    assert_eq!(f.writer.writes(), vec![]);
}

#[test]
fn a_clipboard_failure_is_reported_and_nothing_is_pasted() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("a", 1)).unwrap();
    f.writer.fail_next();

    let result = f.paste(id, PasteMode::AsIs, Some(&target(false)));

    assert!(matches!(
        result,
        Err(AppError::Port(PortError::Unavailable(_)))
    ));
    assert_eq!(f.keys.pastes(), 0);
}

#[test]
fn a_keystroke_failure_falls_back_to_copied_only() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("a", 1)).unwrap();
    f.keys.fail_next();

    let outcome = f.paste(id, PasteMode::AsIs, Some(&target(false))).unwrap();

    assert_eq!(
        outcome,
        PasteOutcome::CopiedOnly(FallbackReason::TargetUnavailable)
    );
}

#[test]
fn hist_011_copy_puts_the_item_on_the_clipboard_without_pasting() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("só copiar", 1)).unwrap();
    f.writer.next_sequence(5);

    CopyItem::new(f.deps()).execute(id).unwrap();

    assert_eq!(
        f.writer.writes(),
        vec![ClipboardContent::Text("só copiar".into())]
    );
    assert!(f.own_writes.take(ClipboardSequence::new(5)));
    assert_eq!(f.keys.pastes(), 0);
    assert_eq!(f.focus.restored(), vec![]);
    assert_eq!(f.history.get(id).unwrap().use_count, 2);
}

#[test]
fn hist_008_get_item_returns_the_full_item() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("texto completo", 1)).unwrap();

    let item = GetItem::new(f.history.clone()).execute(id).unwrap();

    assert!(matches!(item.content, ClipContent::Text(ref t) if t.as_str() == "texto completo"));
    assert_eq!(
        GetItem::new(f.history.clone()).execute(ClipId::new(99)),
        Err(AppError::Port(PortError::NotFound))
    );
}

#[test]
fn cap_008_pausing_and_resuming_capture_is_persisted_and_notified() {
    let settings = Arc::new(FixedSettings::default());
    let events = Arc::new(RecordingEvents::default());
    let use_case = SetCaptureState::new(settings.clone(), events.clone());

    use_case.execute(CaptureState::Paused).unwrap();
    assert!(settings.load().unwrap().capture.paused);

    use_case.execute(CaptureState::Active).unwrap();
    assert!(!settings.load().unwrap().capture.paused);
    assert_eq!(events.recorded(), vec![AppEvent::CaptureStateChanged; 2]);
}

#[test]
fn cap_008_capture_state_is_read_from_settings() {
    let settings = Arc::new(FixedSettings::default());
    let use_case = SetCaptureState::new(settings.clone(), Arc::new(RecordingEvents::default()));

    assert_eq!(use_case.current().unwrap(), CaptureState::Active);
    settings.update(|s| s.capture.paused = true);
    assert_eq!(use_case.current().unwrap(), CaptureState::Paused);
}
