//! File-system image store (ADR-0004, specs/05 §4, REL-002).
// Integration-test helpers are test code (REL-004 applies to production code only).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use clipvault_application::ports::{ImageStore, PortError, StoredImage};
use clipvault_domain::hash::ContentHash;
use clipvault_domain::image::{FileKey, ImagePixels};
use clipvault_infrastructure::images::FsImageStore;
use clipvault_test_support::snapshot::pixels;
use tempfile::TempDir;

/// Keeps the temporary folder alive for the store's lifetime.
struct TempFsStore {
    dir: TempDir,
    store: FsImageStore,
}

impl TempFsStore {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = FsImageStore::open(dir.path().join("images")).unwrap();
        Self { dir, store }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }
}

impl ImageStore for TempFsStore {
    fn save(&self, key: &FileKey, pixels: &ImagePixels) -> Result<StoredImage, PortError> {
        self.store.save(key, pixels)
    }

    fn remove(&self, key: &FileKey) -> Result<(), PortError> {
        self.store.remove(key)
    }

    fn keys(&self) -> Result<Vec<FileKey>, PortError> {
        self.store.keys()
    }
}

mod contract {
    use super::*;
    clipvault_test_support::image_store_contract!(TempFsStore::new());
}

fn key(seed: &str) -> FileKey {
    FileKey::from_hash(&ContentHash::of_text(seed))
}

fn decode(path: &Path) -> image::RgbaImage {
    image::open(path).unwrap().to_rgba8()
}

#[test]
fn originals_are_lossless_png_files_sharded_by_key_prefix() {
    let t = TempFsStore::new();
    let px = pixels(40, 30);

    let stored = t.store.save(&key("a"), &px).unwrap();

    let path = t.store.image_path(&key("a"));
    assert_eq!(
        path,
        t.root()
            .join("images")
            .join(key("a").shard())
            .join(format!("{}.png", key("a").as_str()))
    );
    let decoded = decode(&path);
    assert_eq!((decoded.width(), decoded.height()), (40, 30));
    assert_eq!(decoded.as_raw().as_slice(), px.rgba(), "PNG is lossless");
    assert_eq!(stored.file_size, std::fs::metadata(&path).unwrap().len());
}

#[test]
fn thumbnails_are_written_next_to_the_original() {
    let t = TempFsStore::new();

    t.store.save(&key("a"), &pixels(1280, 320)).unwrap();

    let thumb = decode(&t.store.thumbnail_path(&key("a")));
    assert_eq!((thumb.width(), thumb.height()), (320, 80));
}

#[test]
fn removing_deletes_original_and_thumbnail() {
    let t = TempFsStore::new();
    t.store.save(&key("a"), &pixels(4, 4)).unwrap();

    t.store.remove(&key("a")).unwrap();

    assert!(!t.store.image_path(&key("a")).exists());
    assert!(!t.store.thumbnail_path(&key("a")).exists());
}

#[test]
fn rel_002_no_temporary_files_remain_after_saving() {
    let t = TempFsStore::new();

    t.store.save(&key("a"), &pixels(4, 4)).unwrap();

    let leftovers: Vec<_> = walk(&t.root().join("images"))
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "tmp"))
        .collect();
    assert_eq!(leftovers, Vec::<std::path::PathBuf>::new());
}

#[test]
fn rel_002_temporary_files_from_an_interrupted_save_are_cleaned_on_open() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("images");
    let shard = root.join("ab");
    std::fs::create_dir_all(&shard).unwrap();
    let stale = shard.join(format!("{}.png.tmp", "ab".repeat(32)));
    std::fs::write(&stale, b"partial").unwrap();

    let store = FsImageStore::open(&root).unwrap();

    assert!(!stale.exists());
    assert_eq!(store.keys().unwrap(), Vec::<FileKey>::new());
}

#[test]
fn unrelated_or_malformed_files_are_not_listed() {
    let t = TempFsStore::new();
    t.store.save(&key("a"), &pixels(2, 2)).unwrap();
    let images = t.root().join("images");
    std::fs::write(images.join("readme.txt"), b"x").unwrap();
    std::fs::create_dir_all(images.join("zz")).unwrap();
    std::fs::write(images.join("zz").join("not-a-key.png"), b"x").unwrap();

    assert_eq!(t.store.keys().unwrap(), vec![key("a")]);
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(walk(&path));
        } else {
            files.push(path);
        }
    }
    files
}
