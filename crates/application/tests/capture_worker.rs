//! Capture worker: debounced reads of the clipboard (CAP-012, TRAY-005, REL-003 handling).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use clipvault_application::capture::{Candidate, CaptureWorker, OwnWriteRegistry, WorkerMessage};
use clipvault_application::ports::ClipboardContent;
use clipvault_application::use_cases::{CaptureClipboard, CaptureDependencies};
use clipvault_domain::image::ImagePixels;
use clipvault_test_support::{
    FakeClipboardReader, FixedSettings, InMemoryHistoryRepository, InMemoryImageStore, RealClock,
    RecordingEvents, snapshot,
};

const QUIET: Duration = Duration::from_millis(20);
const SETTLE: Duration = Duration::from_millis(150);

struct Running {
    inbox: Sender<WorkerMessage>,
    handle: JoinHandle<()>,
    reader: Arc<FakeClipboardReader>,
    history: Arc<InMemoryHistoryRepository>,
}

impl Running {
    fn notify(&self) {
        self.inbox.send(WorkerMessage::ClipboardChanged).unwrap();
    }

    fn stop(self) -> (Arc<FakeClipboardReader>, Arc<InMemoryHistoryRepository>) {
        self.inbox.send(WorkerMessage::Shutdown).unwrap();
        self.handle.join().unwrap();
        (self.reader, self.history)
    }
}

fn start() -> Running {
    let reader = Arc::new(FakeClipboardReader::default());
    let history = Arc::new(InMemoryHistoryRepository::default());
    let capture = CaptureClipboard::new(CaptureDependencies {
        history: history.clone(),
        images: Arc::new(InMemoryImageStore::default()),
        settings: Arc::new(FixedSettings::default()),
        clock: Arc::new(RealClock),
        events: Arc::new(RecordingEvents::default()),
        own_writes: Arc::new(OwnWriteRegistry::default()),
    });
    let worker = CaptureWorker::new(reader.clone(), capture, Arc::new(RealClock), QUIET);
    let (inbox, receiver) = mpsc::channel();
    let handle = thread::spawn(move || worker.run(&receiver));
    Running {
        inbox,
        handle,
        reader,
        history,
    }
}

#[test]
fn cap_012_a_burst_of_notifications_reads_the_clipboard_once() {
    let w = start();
    w.reader.set(snapshot::text("final").sequence(1).build());

    for _ in 0..5 {
        w.notify();
    }
    thread::sleep(SETTLE);

    let (reader, history) = w.stop();
    assert_eq!(reader.reads(), 1);
    assert_eq!(history.len(), 1);
}

#[test]
fn cap_001_separate_changes_are_each_captured() {
    let w = start();

    w.reader.set(snapshot::text("primeiro").sequence(1).build());
    w.notify();
    thread::sleep(SETTLE);
    w.reader.set(snapshot::text("segundo").sequence(2).build());
    w.notify();
    thread::sleep(SETTLE);

    let (reader, history) = w.stop();
    assert_eq!(reader.reads(), 2);
    assert_eq!(history.len(), 2);
}

#[test]
fn rel_003_a_failed_read_is_skipped_and_the_worker_keeps_running() {
    let w = start();
    w.reader.set(snapshot::text("ok").sequence(1).build());
    w.reader.fail_next();

    w.notify();
    thread::sleep(SETTLE);
    w.notify();
    thread::sleep(SETTLE);

    let (reader, history) = w.stop();
    assert_eq!(reader.reads(), 2);
    assert_eq!(history.len(), 1);
}

#[test]
fn tray_005_a_pending_change_is_captured_before_shutting_down() {
    let w = start();
    w.reader.set(snapshot::text("pendente").sequence(1).build());

    w.notify();
    let (reader, history) = w.stop(); // well before the quiet period ends

    assert_eq!(reader.reads(), 1);
    assert_eq!(history.len(), 1);
}

#[test]
fn the_worker_stops_when_every_sender_is_dropped() {
    let w = start();

    drop(w.inbox);

    w.handle.join().unwrap();
}

#[test]
fn priv_003_snapshot_and_candidate_debug_output_hide_the_content() {
    let snap = snapshot::text("senha secreta").owner("keepass.exe").build();
    let candidate = Candidate::Text("senha secreta".into());
    let content = ClipboardContent::Text("senha secreta".into());
    let image = ClipboardContent::Image(ImagePixels::new(1, 1, vec![0; 4]).unwrap());

    for debug in [
        format!("{snap:?}"),
        format!("{candidate:?}"),
        format!("{content:?}"),
        format!("{image:?}"),
    ] {
        assert!(!debug.contains("secreta"), "{debug}");
    }
    assert!(format!("{snap:?}").contains("13 bytes"));
}
