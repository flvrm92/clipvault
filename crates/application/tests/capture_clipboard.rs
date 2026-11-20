//! `CaptureClipboard` use case (specs/03 §5.1).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use clipvault_application::AppError;
use clipvault_application::capture::OwnWriteRegistry;
use clipvault_application::ports::{AppEvent, ClipboardSnapshot, PortError};
use clipvault_application::use_cases::{CaptureClipboard, CaptureDependencies, CaptureOutcome};
use clipvault_domain::app_name::AppName;
use clipvault_domain::capture::IgnoreReason;
use clipvault_domain::clip::ClipContent;
use clipvault_domain::hash::ContentHash;
use clipvault_domain::image::FileKey;
use clipvault_domain::settings::Settings;
use clipvault_domain::time::Timestamp;
use clipvault_test_support::{
    FixedClock, FixedSettings, InMemoryHistoryRepository, InMemoryImageStore, RecordingEvents,
    snapshot,
};

struct Fixture {
    history: Arc<InMemoryHistoryRepository>,
    images: Arc<InMemoryImageStore>,
    settings: Arc<FixedSettings>,
    clock: Arc<FixedClock>,
    events: Arc<RecordingEvents>,
    own_writes: Arc<OwnWriteRegistry>,
    use_case: CaptureClipboard,
}

impl Fixture {
    fn new() -> Self {
        let history = Arc::new(InMemoryHistoryRepository::default());
        let images = Arc::new(InMemoryImageStore::default());
        let settings = Arc::new(FixedSettings::default());
        let clock = Arc::new(FixedClock::at(Timestamp::from_millis(1_000)));
        let events = Arc::new(RecordingEvents::default());
        let own_writes = Arc::new(OwnWriteRegistry::default());
        let use_case = CaptureClipboard::new(CaptureDependencies {
            history: history.clone(),
            images: images.clone(),
            settings: settings.clone(),
            clock: clock.clone(),
            events: events.clone(),
            own_writes: own_writes.clone(),
        });
        Self {
            history,
            images,
            settings,
            clock,
            events,
            own_writes,
            use_case,
        }
    }

    fn capture(&self, snapshot: ClipboardSnapshot) -> Result<CaptureOutcome, AppError> {
        self.use_case.execute(snapshot)
    }
}

#[test]
fn cap_001_copied_text_is_stored_at_the_top_with_capture_time() {
    let f = Fixture::new();

    let outcome = f
        .capture(snapshot::text("olá mundo").owner("notepad.exe").build())
        .unwrap();

    let CaptureOutcome::Stored(id) = outcome else {
        panic!("expected Stored, got {outcome:?}")
    };
    let item = f.history.get(id).unwrap();
    assert!(matches!(item.content, ClipContent::Text(ref t) if t.as_str() == "olá mundo"));
    assert_eq!(item.created_at, Timestamp::from_millis(1_000));
    assert_eq!(item.last_used_at, Timestamp::from_millis(1_000));
    assert_eq!(
        item.source_app,
        Some(AppName::parse("notepad.exe").unwrap())
    );
    assert_eq!(f.events.recorded(), vec![AppEvent::HistoryChanged]);
}

#[test]
fn cap_002_copied_image_is_saved_to_disk_and_only_metadata_is_inserted() {
    let f = Fixture::new();

    let CaptureOutcome::Stored(id) = f.capture(snapshot::empty().image(4, 3).build()).unwrap()
    else {
        panic!("expected Stored");
    };

    let item = f.history.get(id).unwrap();
    let ClipContent::Image(meta) = item.content else {
        panic!("expected image")
    };
    assert_eq!((meta.width, meta.height), (4, 3));
    assert_eq!(meta.file_key, FileKey::from_hash(&item.hash));
    assert!(f.images.contains(&meta.file_key));
    assert_eq!(
        meta.file_size,
        f.images.stored_size(&meta.file_key).unwrap()
    );
}

#[test]
fn cap_004_copying_existing_content_promotes_it_instead_of_duplicating() {
    let f = Fixture::new();
    let CaptureOutcome::Stored(id) = f.capture(snapshot::text("abc").build()).unwrap() else {
        panic!("expected Stored");
    };
    f.capture(snapshot::text("outro").build()).unwrap();
    f.clock.advance(Duration::from_secs(60));

    let outcome = f.capture(snapshot::text("abc").build()).unwrap();

    assert_eq!(outcome, CaptureOutcome::Promoted(id));
    assert_eq!(f.history.len(), 2);
    let item = f.history.get(id).unwrap();
    assert_eq!(item.last_used_at, Timestamp::from_millis(61_000));
    assert_eq!(item.use_count, 2);
    assert_eq!(f.history.most_recent().map(|i| i.id), Some(id));
}

#[test]
fn cap_008_nothing_is_recorded_while_capture_is_paused() {
    let f = Fixture::new();
    f.settings.update(|s| s.capture.paused = true);

    let outcome = f.capture(snapshot::text("x").build()).unwrap();

    assert_eq!(outcome, CaptureOutcome::Ignored(IgnoreReason::Paused));
    assert_eq!(f.history.len(), 0);
    assert_eq!(
        f.events.recorded(),
        vec![],
        "ignored changes do not notify the UI"
    );
}

#[test]
fn cap_010_changes_written_by_clipvault_are_not_recorded() {
    let f = Fixture::new();
    f.own_writes.record(snapshot::sequence(9));

    let outcome = f
        .capture(snapshot::text("colado").sequence(9).build())
        .unwrap();

    assert_eq!(outcome, CaptureOutcome::Ignored(IgnoreReason::OwnWrite));
    assert_eq!(f.history.len(), 0);
}

#[test]
fn cap_007_content_flagged_by_password_managers_is_not_recorded() {
    let f = Fixture::new();

    let outcome = f
        .capture(snapshot::text("hunter2").exclude_from_monitor().build())
        .unwrap();

    assert_eq!(outcome, CaptureOutcome::Ignored(IgnoreReason::PrivacyFlag));
    assert_eq!(f.history.len(), 0);
}

#[test]
fn rel_002_image_file_is_removed_when_the_database_insert_fails() {
    let f = Fixture::new();
    f.history.fail_next_insert();
    let pixels_key = FileKey::from_hash(&ContentHash::of_image(
        snapshot::empty()
            .image(2, 2)
            .build()
            .image
            .as_ref()
            .unwrap(),
    ));

    let result = f.capture(snapshot::empty().image(2, 2).build());

    assert!(matches!(result, Err(AppError::Port(PortError::Storage(_)))));
    assert!(!f.images.contains(&pixels_key));
    assert_eq!(f.events.recorded(), vec![]);
}

#[test]
fn image_store_failure_is_reported_and_nothing_is_inserted() {
    let f = Fixture::new();
    f.images.fail_next_save();

    let result = f.capture(snapshot::empty().image(2, 2).build());

    assert!(matches!(result, Err(AppError::Port(_))));
    assert_eq!(f.history.len(), 0);
}

#[test]
fn settings_failure_is_reported() {
    let f = Fixture::new();
    f.settings.fail_next_load();

    assert!(matches!(
        f.capture(snapshot::text("x").build()),
        Err(AppError::Port(_))
    ));
}

#[test]
fn settings_are_read_on_every_capture() {
    let f = Fixture::new();
    f.capture(snapshot::text("a").build()).unwrap();
    f.settings
        .update(|s: &mut Settings| s.capture.max_text_bytes = 1);

    let outcome = f.capture(snapshot::text("long").build()).unwrap();

    assert_eq!(outcome, CaptureOutcome::Ignored(IgnoreReason::TooLarge));
}
