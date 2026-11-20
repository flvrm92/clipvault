//! History use cases: list, pin, delete, clear (HIST-001..007).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use clipvault_application::AppError;
use clipvault_application::ports::{
    AppEvent, ClearScope, HistoryQuery, HistoryRepository, ImageStore, KindFilter, PortError,
};
use clipvault_application::use_cases::{ClearHistory, DeleteItem, ListHistory, TogglePin};
use clipvault_domain::clip::{ClipContent, PinState};
use clipvault_domain::ids::ClipId;
use clipvault_domain::image::FileKey;
use clipvault_test_support::contracts::history::{image_item, text_item};
use clipvault_test_support::snapshot::pixels;
use clipvault_test_support::{InMemoryHistoryRepository, InMemoryImageStore, RecordingEvents};

struct Fixture {
    history: Arc<InMemoryHistoryRepository>,
    images: Arc<InMemoryImageStore>,
    events: Arc<RecordingEvents>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            history: Arc::new(InMemoryHistoryRepository::default()),
            images: Arc::new(InMemoryImageStore::default()),
            events: Arc::new(RecordingEvents::default()),
        }
    }

    /// Inserts an image item and its file.
    fn image(&self, seed: &str, ms: i64) -> (ClipId, FileKey) {
        let item = image_item(seed, ms);
        let ClipContent::Image(meta) = &item.content else {
            unreachable!()
        };
        let key = meta.file_key.clone();
        self.images.save(&key, &pixels(2, 2)).unwrap();
        (self.history.insert(item).unwrap(), key)
    }

    fn list(&self, query: HistoryQuery) -> Vec<ClipId> {
        ListHistory::new(self.history.clone())
            .execute(query)
            .unwrap()
            .items
            .into_iter()
            .map(|i| i.id)
            .collect()
    }
}

#[test]
fn hist_001_lists_the_first_page_most_recent_first() {
    let f = Fixture::new();
    let a = f.history.insert(text_item("a", 1)).unwrap();
    let b = f.history.insert(text_item("b", 2)).unwrap();

    assert_eq!(f.list(HistoryQuery::first_page()), [b, a]);
}

#[test]
fn hist_002_page_size_is_clamped_to_the_maximum() {
    let f = Fixture::new();
    for i in 0..(HistoryQuery::MAX_PAGE_SIZE + 5) {
        f.history
            .insert(text_item(&format!("item {i}"), i64::try_from(i).unwrap()))
            .unwrap();
    }

    let page = ListHistory::new(f.history.clone())
        .execute(HistoryQuery {
            limit: 10_000,
            ..HistoryQuery::first_page()
        })
        .unwrap();

    assert_eq!(page.items.len(), HistoryQuery::MAX_PAGE_SIZE);
    assert!(page.has_more);
}

#[test]
fn hist_002_zero_limit_returns_at_least_one_item() {
    let f = Fixture::new();
    f.history.insert(text_item("a", 1)).unwrap();

    assert_eq!(
        f.list(HistoryQuery {
            limit: 0,
            ..HistoryQuery::first_page()
        })
        .len(),
        1
    );
}

#[test]
fn hist_003_blank_search_is_treated_as_no_search() {
    let f = Fixture::new();
    f.history.insert(text_item("a", 1)).unwrap();
    f.history.insert(text_item("b", 2)).unwrap();

    let all = f.list(HistoryQuery {
        search: Some("   ".into()),
        ..HistoryQuery::first_page()
    });

    assert_eq!(all.len(), 2);
}

#[test]
fn hist_003_search_is_trimmed() {
    let f = Fixture::new();
    let cafe = f.history.insert(text_item("Café", 1)).unwrap();
    f.history.insert(text_item("Chá", 2)).unwrap();

    assert_eq!(
        f.list(HistoryQuery {
            search: Some("  cafe ".into()),
            ..HistoryQuery::first_page()
        }),
        [cafe]
    );
}

#[test]
fn hist_004_filters_are_forwarded() {
    let f = Fixture::new();
    f.history.insert(text_item("t", 1)).unwrap();
    let (image, _) = f.image("i", 2);

    assert_eq!(
        f.list(HistoryQuery {
            filter: KindFilter::Images,
            ..HistoryQuery::first_page()
        }),
        [image]
    );
}

#[test]
fn hist_005_toggle_pin_flips_the_state_and_notifies() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("x", 1)).unwrap();
    let toggle = TogglePin::new(f.history.clone(), f.events.clone());

    assert_eq!(toggle.execute(id).unwrap(), PinState::Pinned);
    assert!(f.history.get(id).unwrap().pinned);
    assert_eq!(toggle.execute(id).unwrap(), PinState::Unpinned);
    assert_eq!(f.events.recorded(), vec![AppEvent::HistoryChanged; 2]);
}

#[test]
fn hist_005_toggling_a_missing_item_fails() {
    let f = Fixture::new();

    let result = TogglePin::new(f.history.clone(), f.events.clone()).execute(ClipId::new(7));

    assert_eq!(result, Err(AppError::Port(PortError::NotFound)));
    assert_eq!(f.events.recorded(), vec![]);
}

#[test]
fn hist_006_deleting_an_image_also_removes_its_file() {
    let f = Fixture::new();
    let (id, key) = f.image("i", 1);

    DeleteItem::new(f.history.clone(), f.images.clone(), f.events.clone())
        .execute(id)
        .unwrap();

    assert_eq!(f.history.get(id), None);
    assert!(!f.images.contains(&key));
    assert_eq!(f.events.recorded(), vec![AppEvent::HistoryChanged]);
}

#[test]
fn hist_006_deleting_a_text_item_succeeds() {
    let f = Fixture::new();
    let id = f.history.insert(text_item("t", 1)).unwrap();

    DeleteItem::new(f.history.clone(), f.images.clone(), f.events.clone())
        .execute(id)
        .unwrap();

    assert!(f.history.is_empty());
}

#[test]
fn hist_006_a_file_removal_failure_does_not_fail_the_deletion() {
    let f = Fixture::new();
    let (id, _) = f.image("i", 1);
    f.images.fail_next_remove();

    let result = DeleteItem::new(f.history.clone(), f.images.clone(), f.events.clone()).execute(id);

    assert_eq!(
        result,
        Ok(()),
        "the orphan file is cleaned up by reconciliation (RET-005)"
    );
    assert_eq!(f.history.get(id), None);
}

#[test]
fn hist_007_clear_removes_unpinned_items_and_their_files() {
    let f = Fixture::new();
    let (_, key) = f.image("i", 1);
    let pinned = f.history.insert(text_item("fixado", 2)).unwrap();
    f.history.insert(text_item("comum", 3)).unwrap();
    f.history.set_pin_state(pinned, PinState::Pinned).unwrap();
    let clear = ClearHistory::new(f.history.clone(), f.images.clone(), f.events.clone());

    let removed = clear.execute(ClearScope::UnpinnedOnly).unwrap();

    assert_eq!(removed, 2);
    assert_eq!(
        f.history.items().iter().map(|i| i.id).collect::<Vec<_>>(),
        [pinned]
    );
    assert!(!f.images.contains(&key));
    assert_eq!(f.events.recorded(), vec![AppEvent::HistoryChanged]);
}

#[test]
fn hist_007_clear_everything_includes_pinned_items() {
    let f = Fixture::new();
    let pinned = f.history.insert(text_item("fixado", 1)).unwrap();
    f.history.set_pin_state(pinned, PinState::Pinned).unwrap();

    let removed = ClearHistory::new(f.history.clone(), f.images.clone(), f.events.clone())
        .execute(ClearScope::Everything)
        .unwrap();

    assert_eq!(removed, 1);
    assert!(f.history.is_empty());
}

#[test]
fn hist_007_clearing_an_empty_history_does_not_notify() {
    let f = Fixture::new();

    let removed = ClearHistory::new(f.history.clone(), f.images.clone(), f.events.clone())
        .execute(ClearScope::UnpinnedOnly)
        .unwrap();

    assert_eq!(removed, 0);
    assert_eq!(f.events.recorded(), vec![]);
}
