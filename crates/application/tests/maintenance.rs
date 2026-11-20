//! Retention purge and image reconciliation (RET-001..005).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use clipvault_application::ports::{AppEvent, HistoryRepository, ImageStore};
use clipvault_application::use_cases::{
    PurgeExpired, ReconcileImages, ReconcileReport, RetentionDependencies,
};
use clipvault_domain::clip::{ClipContent, PinState};
use clipvault_domain::hash::ContentHash;
use clipvault_domain::ids::ClipId;
use clipvault_domain::image::FileKey;
use clipvault_domain::retention::{MaxItems, RetentionPeriod, RetentionPolicy};
use clipvault_domain::time::Timestamp;
use clipvault_test_support::contracts::history::{image_item, text_item};
use clipvault_test_support::snapshot::pixels;
use clipvault_test_support::{
    FixedClock, FixedSettings, InMemoryHistoryRepository, InMemoryImageStore, RecordingEvents,
};

const DAY_MS: i64 = 24 * 60 * 60 * 1_000;

struct Fixture {
    history: Arc<InMemoryHistoryRepository>,
    images: Arc<InMemoryImageStore>,
    settings: Arc<FixedSettings>,
    clock: Arc<FixedClock>,
    events: Arc<RecordingEvents>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            history: Arc::new(InMemoryHistoryRepository::default()),
            images: Arc::new(InMemoryImageStore::default()),
            settings: Arc::new(FixedSettings::default()),
            clock: Arc::new(FixedClock::at(Timestamp::from_millis(100 * DAY_MS))),
            events: Arc::new(RecordingEvents::default()),
        }
    }

    fn purge(&self) -> PurgeExpired {
        PurgeExpired::new(RetentionDependencies {
            history: self.history.clone(),
            images: self.images.clone(),
            settings: self.settings.clone(),
            clock: self.clock.clone(),
            events: self.events.clone(),
        })
    }

    fn text(&self, text: &str, days_ago: i64) -> ClipId {
        self.history
            .insert(text_item(text, (100 - days_ago) * DAY_MS))
            .unwrap()
    }

    fn image(&self, seed: &str, days_ago: i64) -> (ClipId, FileKey) {
        let item = image_item(seed, (100 - days_ago) * DAY_MS);
        let ClipContent::Image(meta) = &item.content else {
            unreachable!()
        };
        let key = meta.file_key.clone();
        self.images.save(&key, &pixels(2, 2)).unwrap();
        (self.history.insert(item).unwrap(), key)
    }

    fn remaining(&self) -> Vec<ClipId> {
        self.history.items().into_iter().map(|i| i.id).collect()
    }
}

#[test]
fn ret_004_purge_applies_the_configured_policy_and_removes_files() {
    let f = Fixture::new(); // default policy: 30 days
    let fresh = f.text("fresh", 1);
    f.text("old", 31);
    let (_, old_image_key) = f.image("old image", 45);

    let removed = f.purge().execute().unwrap();

    assert_eq!(removed, 2);
    assert_eq!(f.remaining(), [fresh]);
    assert!(!f.images.contains(&old_image_key));
    assert_eq!(f.events.recorded(), vec![AppEvent::HistoryChanged]);
}

#[test]
fn ret_003_purge_keeps_pinned_items() {
    let f = Fixture::new();
    let pinned = f.text("pinned", 365);
    f.history.set_pin_state(pinned, PinState::Pinned).unwrap();

    assert_eq!(f.purge().execute().unwrap(), 0);
    assert_eq!(f.remaining(), [pinned]);
    assert_eq!(
        f.events.recorded(),
        vec![],
        "nothing removed, nothing to notify"
    );
}

#[test]
fn ret_002_purge_enforces_the_item_limit() {
    let f = Fixture::new();
    f.settings.update(|s| {
        s.retention.policy = RetentionPolicy {
            period: RetentionPeriod::Forever,
            max_items: MaxItems::Hundred,
        };
    });
    let ids: Vec<_> = (0..105)
        .map(|i| f.text(&format!("item {i}"), 105 - i))
        .collect();

    assert_eq!(f.purge().execute().unwrap(), 5);
    assert_eq!(f.remaining(), ids[5..]);
}

#[test]
fn ret_004_preview_counts_without_removing() {
    let f = Fixture::new();
    f.text("a", 2);
    f.text("b", 10);
    let one_week = RetentionPolicy {
        period: RetentionPeriod::SevenDays,
        max_items: MaxItems::Unlimited,
    };

    assert_eq!(f.purge().preview(one_week).unwrap(), 1);
    assert_eq!(f.remaining().len(), 2);
}

#[test]
fn ret_004_purge_uses_the_current_time() {
    let f = Fixture::new();
    let id = f.text("a", 29);
    assert_eq!(f.purge().execute().unwrap(), 0);

    f.clock.advance(Duration::from_hours(48));

    assert_eq!(f.purge().execute().unwrap(), 1);
    assert_eq!(f.history.get(id), None);
}

#[test]
fn ret_005_reconcile_removes_orphan_files_and_items_without_files() {
    let f = Fixture::new();
    let (kept, kept_key) = f.image("kept", 1);
    let (broken, broken_key) = f.image("broken", 1);
    f.images.remove(&broken_key).unwrap();
    let orphan = FileKey::from_hash(&ContentHash::of_text("orphan"));
    f.images.save(&orphan, &pixels(2, 2)).unwrap();
    let text = f.text("text", 1);

    let report = ReconcileImages::new(f.history.clone(), f.images.clone())
        .execute()
        .unwrap();

    assert_eq!(
        report,
        ReconcileReport {
            orphan_files_removed: 1,
            broken_items_removed: 1
        }
    );
    assert!(!f.images.contains(&orphan));
    assert!(f.images.contains(&kept_key));
    assert_eq!(f.history.get(broken), None);
    assert_eq!(f.remaining(), [kept, text]);
}

#[test]
fn ret_005_reconcile_on_a_consistent_store_changes_nothing() {
    let f = Fixture::new();
    f.image("a", 1);

    let report = ReconcileImages::new(f.history.clone(), f.images.clone())
        .execute()
        .unwrap();

    assert_eq!(report, ReconcileReport::default());
}
