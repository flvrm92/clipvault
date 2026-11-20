//! In-memory fakes of the application ports (pattern: Fake — specs/04 §P-11).
//!
//! From M2 on, each fake must pass the same contract tests as its real adapter.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use clipvault_application::ports::{
    AppEvent, Clock, EventPublisher, HistoryRepository, ImageStore, PortError, SettingsRepository,
    StoredImage,
};
use clipvault_domain::clip::{ClipContent, ClipItem, NewClipItem};
use clipvault_domain::hash::ContentHash;
use clipvault_domain::ids::ClipId;
use clipvault_domain::image::{FileKey, ImagePixels};
use clipvault_domain::settings::Settings;
use clipvault_domain::text::ClipText;
use clipvault_domain::time::Timestamp;

use crate::lock;

const THUMB_MAX_SIDE: u32 = 320;

/// A clock that only moves when told to.
#[derive(Debug)]
pub struct FixedClock {
    now: Mutex<Timestamp>,
}

impl FixedClock {
    /// A clock stopped at `now`.
    pub const fn at(now: Timestamp) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    /// Moves the clock forward.
    pub fn advance(&self, by: Duration) {
        let mut now = lock(&self.now);
        *now = *now + by;
    }

    /// Sets the time.
    pub fn set(&self, to: Timestamp) {
        *lock(&self.now) = to;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        *lock(&self.now)
    }
}

#[derive(Debug, Default)]
struct HistoryState {
    items: Vec<ClipItem>,
    last_id: i64,
    fail_next_insert: bool,
}

/// History kept in memory.
#[derive(Debug, Default)]
pub struct InMemoryHistoryRepository {
    state: Mutex<HistoryState>,
}

impl InMemoryHistoryRepository {
    /// The item with `id`.
    pub fn get(&self, id: ClipId) -> Option<ClipItem> {
        lock(&self.state).items.iter().find(|i| i.id == id).cloned()
    }

    /// All items, in insertion order.
    pub fn items(&self) -> Vec<ClipItem> {
        lock(&self.state).items.clone()
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        lock(&self.state).items.len()
    }

    /// Whether there are no items.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The most recently used item (top of the unpinned history).
    pub fn most_recent(&self) -> Option<ClipItem> {
        lock(&self.state)
            .items
            .iter()
            .max_by_key(|i| (i.last_used_at, i.id))
            .cloned()
    }

    /// Inserts a text item captured at time zero.
    ///
    /// # Panics
    /// If `text` is blank.
    pub fn seed_text(&self, text: &str) -> ClipId {
        let text = ClipText::new(text).unwrap_or_else(|e| panic!("invalid seed text: {e}"));
        let item = NewClipItem {
            hash: ContentHash::of_text(text.as_str()),
            content: ClipContent::Text(text),
            source_app: None,
            captured_at: Timestamp::from_millis(0),
        };
        self.insert(item)
            .unwrap_or_else(|e| panic!("seed failed: {e}"))
    }

    /// Makes the next [`HistoryRepository::insert`] fail with a storage error.
    pub fn fail_next_insert(&self) {
        lock(&self.state).fail_next_insert = true;
    }
}

impl HistoryRepository for InMemoryHistoryRepository {
    fn find_by_hash(&self, hash: &ContentHash) -> Result<Option<ClipId>, PortError> {
        Ok(lock(&self.state)
            .items
            .iter()
            .find(|i| i.hash == *hash)
            .map(|i| i.id))
    }

    fn insert(&self, item: NewClipItem) -> Result<ClipId, PortError> {
        let mut state = lock(&self.state);
        if std::mem::take(&mut state.fail_next_insert) {
            return Err(PortError::Storage("simulated insert failure".into()));
        }
        if state.items.iter().any(|i| i.hash == item.hash) {
            return Err(PortError::Storage("duplicate content hash".into()));
        }
        state.last_id += 1;
        let id = ClipId::new(state.last_id);
        state.items.push(ClipItem {
            id,
            content: item.content,
            hash: item.hash,
            source_app: item.source_app,
            pinned: false,
            use_count: 1,
            created_at: item.captured_at,
            last_used_at: item.captured_at,
        });
        Ok(id)
    }

    fn touch(&self, id: ClipId, at: Timestamp) -> Result<(), PortError> {
        let mut state = lock(&self.state);
        let item = state
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(PortError::NotFound)?;
        item.last_used_at = at;
        item.use_count += 1;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct ImageState {
    files: BTreeMap<FileKey, u64>,
    fail_next_save: bool,
}

/// Image files kept in memory (only their sizes).
#[derive(Debug, Default)]
pub struct InMemoryImageStore {
    state: Mutex<ImageState>,
}

impl InMemoryImageStore {
    /// Whether an image is stored under `key`.
    pub fn contains(&self, key: &FileKey) -> bool {
        lock(&self.state).files.contains_key(key)
    }

    /// The reported file size of the image under `key`.
    pub fn stored_size(&self, key: &FileKey) -> Option<u64> {
        lock(&self.state).files.get(key).copied()
    }

    /// Number of stored images.
    pub fn len(&self) -> usize {
        lock(&self.state).files.len()
    }

    /// Whether no image is stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Makes the next [`ImageStore::save`] fail with a storage error.
    pub fn fail_next_save(&self) {
        lock(&self.state).fail_next_save = true;
    }
}

impl ImageStore for InMemoryImageStore {
    fn save(&self, key: &FileKey, pixels: &ImagePixels) -> Result<StoredImage, PortError> {
        let mut state = lock(&self.state);
        if std::mem::take(&mut state.fail_next_save) {
            return Err(PortError::Storage("simulated save failure".into()));
        }
        // Pretend PNG compresses to half the raw size.
        let file_size = (pixels.byte_len() as u64).div_ceil(2);
        state.files.insert(key.clone(), file_size);

        let (w, h) = (pixels.width(), pixels.height());
        let scale = |side: u32, other: u32| {
            u32::try_from(u64::from(side) * u64::from(THUMB_MAX_SIDE) / u64::from(other))
                .unwrap_or(1)
                .max(1)
        };
        let (thumb_width, thumb_height) = match w.max(h) {
            longest if longest <= THUMB_MAX_SIDE => (w, h),
            _ if w >= h => (THUMB_MAX_SIDE, scale(h, w)),
            _ => (scale(w, h), THUMB_MAX_SIDE),
        };
        Ok(StoredImage {
            file_size,
            thumb_width,
            thumb_height,
        })
    }

    fn remove(&self, key: &FileKey) -> Result<(), PortError> {
        lock(&self.state).files.remove(key);
        Ok(())
    }
}

#[derive(Debug, Default)]
struct SettingsState {
    settings: Settings,
    fail_next_load: bool,
}

/// Settings kept in memory, starting from the defaults.
#[derive(Debug, Default)]
pub struct FixedSettings {
    state: Mutex<SettingsState>,
}

impl FixedSettings {
    /// Changes the stored settings.
    pub fn update(&self, change: impl FnOnce(&mut Settings)) {
        change(&mut lock(&self.state).settings);
    }

    /// Makes the next [`SettingsRepository::load`] fail.
    pub fn fail_next_load(&self) {
        lock(&self.state).fail_next_load = true;
    }
}

impl SettingsRepository for FixedSettings {
    fn load(&self) -> Result<Settings, PortError> {
        let mut state = lock(&self.state);
        if std::mem::take(&mut state.fail_next_load) {
            return Err(PortError::Storage("simulated settings failure".into()));
        }
        Ok(state.settings.clone())
    }
}

/// Records published events.
#[derive(Debug, Default)]
pub struct RecordingEvents {
    events: Mutex<Vec<AppEvent>>,
}

impl RecordingEvents {
    /// Events published so far.
    pub fn recorded(&self) -> Vec<AppEvent> {
        lock(&self.events).clone()
    }
}

impl EventPublisher for RecordingEvents {
    fn publish(&self, event: AppEvent) {
        lock(&self.events).push(event);
    }
}
