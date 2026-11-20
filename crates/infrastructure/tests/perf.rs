//! Performance requirements PERF-002..004, measured as p95 against the real adapters.
//!
//! Ignored by default (timing-sensitive). Run in release mode on the reference machine:
//! `cargo test -p clipvault-infrastructure --release --test perf -- --ignored --nocapture`
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use clipvault_application::capture::OwnWriteRegistry;
use clipvault_application::ports::{AppEvent, EventPublisher, HistoryQuery, HistoryRepository};
use clipvault_application::use_cases::{
    CaptureClipboard, CaptureDependencies, CaptureOutcome, ListHistory,
};
use clipvault_infrastructure::clock::SystemClock;
use clipvault_infrastructure::images::FsImageStore;
use clipvault_infrastructure::sqlite::{
    Database, SqliteHistoryRepository, SqliteSettingsRepository,
};
use clipvault_test_support::contracts::history::text_item;
use clipvault_test_support::snapshot;
use tempfile::TempDir;

struct NoEvents;

impl EventPublisher for NoEvents {
    fn publish(&self, _: AppEvent) {}
}

struct Setup {
    _dir: TempDir,
    history: Arc<SqliteHistoryRepository>,
    capture: CaptureClipboard,
}

fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(&dir.path().join("history.db"))
        .unwrap()
        .database;
    let history = Arc::new(SqliteHistoryRepository::new(db.clone()));
    let capture = CaptureClipboard::new(CaptureDependencies {
        history: history.clone(),
        images: Arc::new(FsImageStore::open(dir.path().join("images")).unwrap()),
        settings: Arc::new(SqliteSettingsRepository::new(db)),
        clock: Arc::new(SystemClock),
        events: Arc::new(NoEvents),
        own_writes: Arc::new(OwnWriteRegistry::default()),
    });
    Setup {
        _dir: dir,
        history,
        capture,
    }
}

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[(samples.len() * 95).div_ceil(100) - 1]
}

fn measure(runs: usize, mut operation: impl FnMut(usize)) -> Duration {
    let samples = (0..runs)
        .map(|i| {
            let start = Instant::now();
            operation(i);
            start.elapsed()
        })
        .collect();
    p95(samples)
}

#[test]
#[ignore = "performance: run with --release --ignored"]
fn perf_002_text_capture_to_persistence_p95_under_50ms() {
    let s = setup();

    let p95 = measure(300, |i| {
        let outcome = s
            .capture
            .execute(snapshot::text(&format!("copied text number {i}")).build())
            .unwrap();
        assert!(matches!(outcome, CaptureOutcome::Stored(_)), "{outcome:?}");
    });

    println!("PERF-002 text capture p95 = {p95:?}");
    assert!(p95 <= Duration::from_millis(50), "p95 {p95:?}");
}

#[test]
#[ignore = "performance: run with --release --ignored"]
fn perf_003_4k_image_capture_to_persistence_p95_under_800ms() {
    let s = setup();
    // Pseudo-random noise is the worst case for PNG compression (real screenshots compress
    // better). Each run varies one byte so every capture is new content.
    let mut seed: u32 = 0x9E37_79B9;
    let base: Vec<u8> = (0..3840 * 2160 * 4)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed.to_le_bytes()[0]
        })
        .collect();

    let p95 = measure(10, |i| {
        let mut rgba = base.clone();
        rgba[0] = u8::try_from(i).unwrap();
        let mut snap = snapshot::empty().build();
        snap.image = Some(clipvault_domain::image::ImagePixels::new(3840, 2160, rgba).unwrap());
        let outcome = s.capture.execute(snap).unwrap();
        assert!(matches!(outcome, CaptureOutcome::Stored(_)), "{outcome:?}");
    });

    println!("PERF-003 4K image capture p95 = {p95:?}");
    assert!(p95 <= Duration::from_millis(800), "p95 {p95:?}");
}

#[test]
#[ignore = "performance: run with --release --ignored"]
fn perf_004_search_over_5000_items_p95_under_50ms() {
    let s = setup();
    for i in 0..5_000 {
        let text = format!("Item {i}: relatório de vendas {} — café {}", i % 97, i % 13);
        s.history.insert(text_item(&text, i)).unwrap();
    }
    let list = ListHistory::new(s.history.clone());
    let terms = [
        "cafe 7",
        "relatorio",
        "vendas 42",
        "inexistente",
        "item 4999",
    ];

    let p95 = measure(100, |i| {
        let query = HistoryQuery {
            search: Some(terms[i % terms.len()].into()),
            ..HistoryQuery::first_page()
        };
        list.execute(query).unwrap();
    });

    println!("PERF-004 search over 5000 items p95 = {p95:?}");
    assert!(p95 <= Duration::from_millis(50), "p95 {p95:?}");
}
