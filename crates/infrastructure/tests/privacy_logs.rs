//! PRIV-003: clipboard content never reaches the logs, even at TRACE level (specs/08 §3).
//!
//! A unique sentinel text goes through every flow — capture, duplicate, failures, search,
//! deletion, the capture worker — while all log output is collected; the sentinel must not
//! appear in it.
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use clipvault_application::capture::{CaptureWorker, OwnWriteRegistry, WorkerMessage};
use clipvault_application::ports::{HistoryQuery, HistoryRepository};
use clipvault_application::use_cases::{
    CaptureClipboard, CaptureDependencies, CaptureOutcome, DeleteItem, ListHistory,
};
use clipvault_infrastructure::clock::SystemClock;
use clipvault_infrastructure::images::FsImageStore;
use clipvault_infrastructure::sqlite::{
    Database, SqliteHistoryRepository, SqliteSettingsRepository,
};
use clipvault_test_support::{FakeClipboardReader, RecordingEvents, snapshot};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct SharedLog(Arc<Mutex<Vec<u8>>>);

impl SharedLog {
    fn contents(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap_or_else(PoisonError::into_inner)).into_owned()
    }
}

impl Write for SharedLog {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for SharedLog {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[test]
fn priv_003_the_sentinel_text_never_appears_in_trace_logs() {
    let log = SharedLog::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(log.clone())
        .with_ansi(false)
        .finish();
    let sentinel = format!("SENTINEL-{}", std::process::id() ^ 0x5EED_CAFE);

    tracing::subscriber::with_default(subscriber, || {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("history.db"))
            .unwrap()
            .database;
        let history = Arc::new(SqliteHistoryRepository::new(db.clone()));
        let images = Arc::new(FsImageStore::open(dir.path().join("images")).unwrap());
        let events = Arc::new(RecordingEvents::default());
        let deps = || CaptureDependencies {
            history: history.clone(),
            images: images.clone(),
            settings: Arc::new(SqliteSettingsRepository::new(db.clone())),
            clock: Arc::new(SystemClock),
            events: events.clone(),
            own_writes: Arc::new(OwnWriteRegistry::default()),
        };
        let capture = CaptureClipboard::new(deps());

        // Capture, duplicate (promotion), oversized and private variants.
        let text = format!("conteúdo secreto {sentinel}");
        let CaptureOutcome::Stored(id) = capture.execute(snapshot::text(&text).build()).unwrap()
        else {
            panic!("expected Stored");
        };
        capture.execute(snapshot::text(&text).build()).unwrap();
        capture
            .execute(snapshot::text(&text).exclude_from_monitor().build())
            .unwrap();
        capture
            .execute(snapshot::text(&sentinel.repeat(100_000)).build())
            .unwrap();

        // Search and deletion.
        let page = ListHistory::new(history.clone())
            .execute(HistoryQuery {
                search: Some(sentinel.clone()),
                ..HistoryQuery::first_page()
            })
            .unwrap();
        assert_eq!(page.items.len(), 1);
        DeleteItem::new(history.clone(), images.clone(), events.clone())
            .execute(id)
            .unwrap();
        assert!(history.get(id).unwrap().is_none());

        // The worker on this thread: a failed read (warning) and a successful capture (debug).
        let reader = Arc::new(FakeClipboardReader::default());
        reader.set(
            snapshot::text(&format!("worker {sentinel}"))
                .sequence(9)
                .build(),
        );
        reader.fail_next();
        let worker = CaptureWorker::new(
            reader,
            CaptureClipboard::new(deps()),
            Arc::new(SystemClock),
            Duration::ZERO,
        );
        let (inbox, receiver) = mpsc::channel();
        for message in [
            WorkerMessage::ClipboardChanged,
            WorkerMessage::ClipboardChanged,
            WorkerMessage::Shutdown,
        ] {
            inbox.send(message).unwrap();
        }
        worker.run(&receiver);
    });

    let output = log.contents();
    assert!(
        output.contains("clipboard could not be read"),
        "logging must be active:\n{output}"
    );
    assert!(
        output.contains("clipboard change processed"),
        "debug events must be captured:\n{output}"
    );
    assert!(
        !output.contains(&sentinel),
        "clipboard content leaked into logs:\n{output}"
    );
}
