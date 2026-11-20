//! Contract of [`ImageStore`] (ADR-0004, REL-002, RET-005).

use clipvault_application::ports::ImageStore;
use clipvault_domain::hash::ContentHash;
use clipvault_domain::image::{FileKey, thumbnail_size};

use crate::snapshot::pixels;

fn key(seed: &str) -> FileKey {
    FileKey::from_hash(&ContentHash::of_text(seed))
}

/// Saved images are listed and report a non-empty file and the specified thumbnail size.
pub fn saved_images_are_listed_with_thumbnail_size(store: &dyn ImageStore) {
    let stored = store.save(&key("a"), &pixels(640, 160)).unwrap();

    assert!(stored.file_size > 0);
    assert_eq!(
        (stored.thumb_width, stored.thumb_height),
        thumbnail_size(640, 160)
    );
    assert_eq!(store.keys().unwrap(), vec![key("a")]);
}

/// Removing deletes the image; removing a missing key is not an error.
pub fn remove_deletes_and_is_idempotent(store: &dyn ImageStore) {
    store.save(&key("a"), &pixels(2, 2)).unwrap();
    store.save(&key("b"), &pixels(2, 2)).unwrap();

    store.remove(&key("a")).unwrap();
    store.remove(&key("a")).unwrap();

    assert_eq!(store.keys().unwrap(), vec![key("b")]);
}

/// Saving the same key twice keeps a single image.
pub fn saving_twice_keeps_one_image(store: &dyn ImageStore) {
    store.save(&key("a"), &pixels(2, 2)).unwrap();
    store.save(&key("a"), &pixels(3, 3)).unwrap();

    assert_eq!(store.keys().unwrap(), vec![key("a")]);
}

/// An empty store lists nothing.
pub fn empty_store_lists_nothing(store: &dyn ImageStore) {
    assert_eq!(store.keys().unwrap(), Vec::<FileKey>::new());
}

/// Generates one `#[test]` per [`ImageStore`] contract case.
#[macro_export]
macro_rules! image_store_contract {
    ($make:expr) => {
        $crate::__contract_tests!(
            images,
            $make,
            saved_images_are_listed_with_thumbnail_size,
            remove_deletes_and_is_idempotent,
            saving_twice_keeps_one_image,
            empty_store_lists_nothing,
        );
    };
}
