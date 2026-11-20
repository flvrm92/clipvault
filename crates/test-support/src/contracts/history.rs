//! Contract of [`HistoryRepository`] (HIST-001..007, CAP-004, RET-001..005).

use clipvault_application::ports::{
    ClearScope, HistoryQuery, HistoryRepository, KindFilter, PortError, Removal,
};
use clipvault_domain::clip::{ClipContent, NewClipItem, PinState};
use clipvault_domain::hash::ContentHash;
use clipvault_domain::ids::ClipId;
use clipvault_domain::image::{FileKey, ImageMeta};
use clipvault_domain::text::ClipText;
use clipvault_domain::time::Timestamp;

fn at(ms: i64) -> Timestamp {
    Timestamp::from_millis(ms)
}

/// A text item captured at `ms`.
pub fn text_item(text: &str, ms: i64) -> NewClipItem {
    NewClipItem {
        content: ClipContent::Text(ClipText::new(text).unwrap()),
        hash: ContentHash::of_text(text),
        source_app: None,
        captured_at: at(ms),
    }
}

/// An image item (identified by `seed`) captured at `ms`.
pub fn image_item(seed: &str, ms: i64) -> NewClipItem {
    let hash = ContentHash::of_text(&format!("image:{seed}"));
    NewClipItem {
        content: ClipContent::Image(ImageMeta {
            file_key: FileKey::from_hash(&hash),
            width: 1920,
            height: 1080,
            file_size: 4_096,
            thumb_width: 320,
            thumb_height: 180,
        }),
        hash,
        source_app: None,
        captured_at: at(ms),
    }
}

fn key_of(item: &NewClipItem) -> FileKey {
    match &item.content {
        ClipContent::Image(meta) => meta.file_key.clone(),
        ClipContent::Text(_) => panic!("not an image"),
    }
}

fn texts(repo: &dyn HistoryRepository, query: &HistoryQuery) -> Vec<String> {
    repo.list(query)
        .unwrap()
        .items
        .into_iter()
        .map(|item| match item.content {
            ClipContent::Text(text) => text.as_str().to_owned(),
            ClipContent::Image(meta) => format!("<image {}>", &meta.file_key.as_str()[..6]),
        })
        .collect()
}

fn search(term: &str) -> HistoryQuery {
    HistoryQuery {
        search: Some(term.into()),
        ..HistoryQuery::first_page()
    }
}

/// Inserted items are returned with initial counters.
pub fn insert_then_get_returns_the_item_with_initial_counters(repo: &dyn HistoryRepository) {
    let mut new = text_item("olá", 1_000);
    new.source_app = Some(clipvault_domain::app_name::AppName::parse("code.exe").unwrap());
    let id = repo.insert(new.clone()).unwrap();

    let item = repo.get(id).unwrap().unwrap();

    assert_eq!(item.content, new.content);
    assert_eq!(item.hash, new.hash);
    assert_eq!(item.source_app, new.source_app);
    assert!(!item.pinned);
    assert_eq!(item.use_count, 1);
    assert_eq!((item.created_at, item.last_used_at), (at(1_000), at(1_000)));
    assert_eq!(repo.get(ClipId::new(9_999)).unwrap(), None);
}

/// Image metadata round-trips.
pub fn image_items_round_trip_their_metadata(repo: &dyn HistoryRepository) {
    let new = image_item("a", 1);
    let id = repo.insert(new.clone()).unwrap();

    assert_eq!(repo.get(id).unwrap().unwrap().content, new.content);
}

/// CAP-004: items are found by content hash; duplicates are rejected.
pub fn cap_004_find_by_hash_and_unique_hashes(repo: &dyn HistoryRepository) {
    let id = repo.insert(text_item("abc", 1)).unwrap();

    assert_eq!(
        repo.find_by_hash(&ContentHash::of_text("abc")).unwrap(),
        Some(id)
    );
    assert_eq!(
        repo.find_by_hash(&ContentHash::of_text("xyz")).unwrap(),
        None
    );
    assert!(matches!(
        repo.insert(text_item("abc", 2)),
        Err(PortError::Storage(_))
    ));
}

/// CAP-004: touching updates recency and use count.
pub fn cap_004_touch_updates_last_used_and_use_count(repo: &dyn HistoryRepository) {
    let id = repo.insert(text_item("abc", 1)).unwrap();

    repo.touch(id, at(50)).unwrap();

    let item = repo.get(id).unwrap().unwrap();
    assert_eq!(
        (item.last_used_at, item.use_count, item.created_at),
        (at(50), 2, at(1))
    );
    assert_eq!(
        repo.touch(ClipId::new(9_999), at(1)),
        Err(PortError::NotFound)
    );
}

/// HIST-001: pinned first, then most recently used first.
pub fn hist_001_list_orders_pinned_first_then_most_recent(repo: &dyn HistoryRepository) {
    let old = repo.insert(text_item("old", 1)).unwrap();
    repo.insert(text_item("middle", 2)).unwrap();
    let pinned = repo.insert(text_item("pinned", 0)).unwrap();
    repo.insert(text_item("new", 3)).unwrap();
    repo.set_pin_state(pinned, PinState::Pinned).unwrap();
    repo.touch(old, at(10)).unwrap();

    assert_eq!(
        texts(repo, &HistoryQuery::first_page()),
        ["pinned", "old", "new", "middle"]
    );
}

/// HIST-002: pages with `has_more`.
pub fn hist_002_list_paginates(repo: &dyn HistoryRepository) {
    for i in 0..5 {
        repo.insert(text_item(&format!("item {i}"), i)).unwrap();
    }
    let page = |offset, limit| {
        repo.list(&HistoryQuery {
            offset,
            limit,
            ..HistoryQuery::first_page()
        })
        .unwrap()
    };

    let first = page(0, 2);
    let last = page(4, 2);

    assert_eq!(first.items.len(), 2);
    assert!(first.has_more);
    assert_eq!(last.items.len(), 1);
    assert!(!last.has_more);
    assert_eq!(page(2, 3).items.len(), 3);
    assert!(!page(2, 3).has_more);
}

/// HIST-003: search ignores case and accents and matches substrings of text items only.
pub fn hist_003_search_ignores_case_and_accents(repo: &dyn HistoryRepository) {
    repo.insert(text_item("Café com leite", 1)).unwrap();
    repo.insert(text_item("Chá", 2)).unwrap();
    repo.insert(image_item("cafe", 3)).unwrap();

    assert_eq!(texts(repo, &search("cafe")), ["Café com leite"]);
    assert_eq!(texts(repo, &search("COM LEI")), ["Café com leite"]);
    assert_eq!(texts(repo, &search("chá")), ["Chá"]);
    assert_eq!(texts(repo, &search("nada")), Vec::<String>::new());
}

/// HIST-003: wildcard characters in the search are literal.
pub fn hist_003_search_treats_wildcards_literally(repo: &dyn HistoryRepository) {
    repo.insert(text_item("desconto de 100%", 1)).unwrap();
    repo.insert(text_item("1000 unidades", 2)).unwrap();
    repo.insert(text_item("snake_case", 3)).unwrap();
    repo.insert(text_item("snakeXcase", 4)).unwrap();

    assert_eq!(texts(repo, &search("100%")), ["desconto de 100%"]);
    assert_eq!(texts(repo, &search("e_c")), ["snake_case"]);
}

/// HIST-004: kind filters.
pub fn hist_004_list_filters_by_kind(repo: &dyn HistoryRepository) {
    let text = repo.insert(text_item("texto", 1)).unwrap();
    repo.insert(image_item("img", 2)).unwrap();
    repo.set_pin_state(text, PinState::Pinned).unwrap();
    let only = |filter| {
        repo.list(&HistoryQuery {
            filter,
            ..HistoryQuery::first_page()
        })
        .unwrap()
        .items
        .len()
    };

    assert_eq!(only(KindFilter::All), 2);
    assert_eq!(only(KindFilter::Text), 1);
    assert_eq!(only(KindFilter::Images), 1);
    assert_eq!(only(KindFilter::Pinned), 1);
}

/// HIST-005: pin and unpin.
pub fn hist_005_pin_and_unpin(repo: &dyn HistoryRepository) {
    let id = repo.insert(text_item("x", 1)).unwrap();

    repo.set_pin_state(id, PinState::Pinned).unwrap();
    assert!(repo.get(id).unwrap().unwrap().pinned);
    repo.set_pin_state(id, PinState::Unpinned).unwrap();
    assert!(!repo.get(id).unwrap().unwrap().pinned);
    assert_eq!(
        repo.set_pin_state(ClipId::new(9_999), PinState::Pinned),
        Err(PortError::NotFound)
    );
}

/// HIST-006: deleting returns the image key so the file can be removed.
pub fn hist_006_delete_returns_the_image_key(repo: &dyn HistoryRepository) {
    let image = image_item("img", 1);
    let image_id = repo.insert(image.clone()).unwrap();
    let text_id = repo.insert(text_item("t", 2)).unwrap();

    assert_eq!(repo.delete(image_id).unwrap(), Some(key_of(&image)));
    assert_eq!(repo.delete(text_id).unwrap(), None);
    assert_eq!(repo.get(image_id).unwrap(), None);
    assert_eq!(repo.find_by_hash(&image.hash).unwrap(), None);
    assert_eq!(repo.delete(text_id), Err(PortError::NotFound));
}

/// HIST-007: clearing keeps pinned items unless everything is requested.
pub fn hist_007_clear_keeps_pinned_items_by_default(repo: &dyn HistoryRepository) {
    let image = image_item("img", 1);
    repo.insert(image.clone()).unwrap();
    let pinned = repo.insert(text_item("fixado", 2)).unwrap();
    repo.insert(text_item("comum", 3)).unwrap();
    repo.set_pin_state(pinned, PinState::Pinned).unwrap();

    assert_eq!(
        repo.clear(ClearScope::UnpinnedOnly).unwrap(),
        Removal {
            count: 2,
            image_keys: vec![key_of(&image)]
        }
    );
    assert_eq!(texts(repo, &HistoryQuery::first_page()), ["fixado"]);
    assert_eq!(
        repo.clear(ClearScope::Everything).unwrap(),
        Removal {
            count: 1,
            image_keys: vec![]
        }
    );
    assert_eq!(
        repo.list(&HistoryQuery::first_page()).unwrap().items.len(),
        0
    );
}

/// RET-001..003: the retention view mirrors the items.
pub fn retention_candidates_mirror_the_items(repo: &dyn HistoryRepository) {
    let a = repo.insert(text_item("a", 10)).unwrap();
    let b = repo.insert(text_item("b", 20)).unwrap();
    repo.set_pin_state(b, PinState::Pinned).unwrap();

    let mut candidates = repo.retention_candidates().unwrap();
    candidates.sort_by_key(|c| c.id);

    assert_eq!(candidates.len(), 2);
    assert_eq!(
        (
            candidates[0].id,
            candidates[0].last_used_at,
            candidates[0].pinned
        ),
        (a, at(10), false)
    );
    assert_eq!((candidates[1].id, candidates[1].pinned), (b, true));
}

/// Bulk deletion ignores unknown ids and returns image keys.
pub fn delete_many_removes_the_given_items(repo: &dyn HistoryRepository) {
    let image = image_item("img", 1);
    let image_id = repo.insert(image.clone()).unwrap();
    let keep = repo.insert(text_item("keep", 2)).unwrap();
    let drop = repo.insert(text_item("drop", 3)).unwrap();

    let removal = repo
        .delete_many(&[image_id, drop, ClipId::new(9_999)])
        .unwrap();

    assert_eq!(
        removal,
        Removal {
            count: 2,
            image_keys: vec![key_of(&image)]
        }
    );
    assert!(repo.get(keep).unwrap().is_some());
    assert_eq!(repo.get(drop).unwrap(), None);
    assert_eq!(repo.delete_many(&[]).unwrap(), Removal::default());
}

/// RET-005: image entries list every image item.
pub fn image_entries_list_image_items(repo: &dyn HistoryRepository) {
    let image = image_item("img", 1);
    let id = repo.insert(image.clone()).unwrap();
    repo.insert(text_item("t", 2)).unwrap();

    assert_eq!(repo.image_entries().unwrap(), vec![(id, key_of(&image))]);
}

/// Generates one `#[test]` per [`HistoryRepository`] contract case.
/// `$make` is an expression producing a fresh, empty repository.
#[macro_export]
macro_rules! history_repository_contract {
    ($make:expr) => {
        $crate::__contract_tests!(
            history,
            $make,
            insert_then_get_returns_the_item_with_initial_counters,
            image_items_round_trip_their_metadata,
            cap_004_find_by_hash_and_unique_hashes,
            cap_004_touch_updates_last_used_and_use_count,
            hist_001_list_orders_pinned_first_then_most_recent,
            hist_002_list_paginates,
            hist_003_search_ignores_case_and_accents,
            hist_003_search_treats_wildcards_literally,
            hist_004_list_filters_by_kind,
            hist_005_pin_and_unpin,
            hist_006_delete_returns_the_image_key,
            hist_007_clear_keeps_pinned_items_by_default,
            retention_candidates_mirror_the_items,
            delete_many_removes_the_given_items,
            image_entries_list_image_items,
        );
    };
}
