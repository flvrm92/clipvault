//! In-memory fakes of the application ports (pattern: Fake — specs/04 §P-11).
//!
//! Each fake passes the same contract tests as its real adapter (`tests/fake_contracts.rs`).

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use clipvault_application::ports::{
    AppEvent, ClearScope, ClipboardReader, ClipboardSnapshot, Clock, EventPublisher, HistoryQuery,
    HistoryRepository, ImageStore, KindFilter, Page, PortError, Removal, SettingsRepository,
    StoredImage,
};
use clipvault_domain::clip::{ClipContent, ClipItem, NewClipItem, PinState};
use clipvault_domain::hash::ContentHash;
use clipvault_domain::ids::ClipId;
use clipvault_domain::image::{FileKey, ImagePixels, thumbnail_size};
use clipvault_domain::retention::RetentionCandidate;
use clipvault_domain::settings::Settings;
use clipvault_domain::text::{ClipText, normalize_for_search};
use clipvault_domain::time::Timestamp;

use crate::lock;

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

fn image_key(item: &ClipItem) -> Option<FileKey> {
    match &item.content {
        ClipContent::Image(meta) => Some(meta.file_key.clone()),
        ClipContent::Text(_) => None,
    }
}

fn matches_query(item: &ClipItem, query: &HistoryQuery) -> bool {
    let kind_ok = match query.filter {
        KindFilter::All => true,
        KindFilter::Text => matches!(item.content, ClipContent::Text(_)),
        KindFilter::Images => matches!(item.content, ClipContent::Image(_)),
        KindFilter::Pinned => item.pinned,
    };
    let search_ok = query
        .search
        .as_deref()
        .is_none_or(|term| match &item.content {
            ClipContent::Text(text) => text.search_form().contains(&normalize_for_search(term)),
            ClipContent::Image(_) => false,
        });
    kind_ok && search_ok
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

    fn with_item<T>(
        &self,
        id: ClipId,
        change: impl FnOnce(&mut ClipItem) -> T,
    ) -> Result<T, PortError> {
        let mut state = lock(&self.state);
        let item = state
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(PortError::NotFound)?;
        Ok(change(item))
    }

    fn remove_where(&self, condition: impl Fn(&ClipItem) -> bool) -> Removal {
        let mut state = lock(&self.state);
        let (removed, kept): (Vec<_>, Vec<_>) = state.items.drain(..).partition(condition);
        state.items = kept;
        Removal {
            count: removed.len(),
            image_keys: removed.iter().filter_map(image_key).collect(),
        }
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

    fn get(&self, id: ClipId) -> Result<Option<ClipItem>, PortError> {
        Ok(Self::get(self, id))
    }

    fn list(&self, query: &HistoryQuery) -> Result<Page<ClipItem>, PortError> {
        let mut matching: Vec<ClipItem> = lock(&self.state)
            .items
            .iter()
            .filter(|i| matches_query(i, query))
            .cloned()
            .collect();
        matching.sort_by(|a, b| {
            (b.pinned, b.last_used_at, b.id).cmp(&(a.pinned, a.last_used_at, a.id))
        });
        let has_more = matching.len() > query.offset.saturating_add(query.limit);
        let items = matching
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect();
        Ok(Page { items, has_more })
    }

    fn touch(&self, id: ClipId, at: Timestamp) -> Result<(), PortError> {
        self.with_item(id, |item| {
            item.last_used_at = at;
            item.use_count += 1;
        })
    }

    fn set_pin_state(&self, id: ClipId, state: PinState) -> Result<(), PortError> {
        self.with_item(id, |item| item.pinned = state.is_pinned())
    }

    fn delete(&self, id: ClipId) -> Result<Option<FileKey>, PortError> {
        let mut state = lock(&self.state);
        let index = state
            .items
            .iter()
            .position(|i| i.id == id)
            .ok_or(PortError::NotFound)?;
        Ok(image_key(&state.items.remove(index)))
    }

    fn delete_many(&self, ids: &[ClipId]) -> Result<Removal, PortError> {
        Ok(self.remove_where(|i| ids.contains(&i.id)))
    }

    fn clear(&self, scope: ClearScope) -> Result<Removal, PortError> {
        Ok(self.remove_where(|i| scope == ClearScope::Everything || !i.pinned))
    }

    fn retention_candidates(&self) -> Result<Vec<RetentionCandidate>, PortError> {
        Ok(lock(&self.state)
            .items
            .iter()
            .map(ClipItem::retention_candidate)
            .collect())
    }

    fn image_entries(&self) -> Result<Vec<(ClipId, FileKey)>, PortError> {
        Ok(lock(&self.state)
            .items
            .iter()
            .filter_map(|i| image_key(i).map(|key| (i.id, key)))
            .collect())
    }
}

#[derive(Debug, Default)]
struct ImageState {
    files: BTreeMap<FileKey, u64>,
    fail_next_save: bool,
    fail_next_remove: bool,
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

    /// Makes the next [`ImageStore::remove`] fail with a storage error.
    pub fn fail_next_remove(&self) {
        lock(&self.state).fail_next_remove = true;
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
        let (thumb_width, thumb_height) = thumbnail_size(pixels.width(), pixels.height());
        Ok(StoredImage {
            file_size,
            thumb_width,
            thumb_height,
        })
    }

    fn remove(&self, key: &FileKey) -> Result<(), PortError> {
        let mut state = lock(&self.state);
        if std::mem::take(&mut state.fail_next_remove) {
            return Err(PortError::Storage("simulated remove failure".into()));
        }
        state.files.remove(key);
        Ok(())
    }

    fn keys(&self) -> Result<Vec<FileKey>, PortError> {
        Ok(lock(&self.state).files.keys().cloned().collect())
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

    fn save(&self, settings: &Settings) -> Result<(), PortError> {
        lock(&self.state).settings = settings.clone();
        Ok(())
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

/// The wall clock, for tests that involve real waiting (e.g. the capture worker).
#[derive(Debug, Clone, Copy, Default)]
pub struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> Timestamp {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
        Timestamp::from_millis(millis)
    }
}

#[derive(Debug, Default)]
struct ReaderState {
    current: Option<ClipboardSnapshot>,
    reads: usize,
    fail_next: bool,
}

/// A clipboard whose content is set by the test.
#[derive(Debug, Default)]
pub struct FakeClipboardReader {
    state: Mutex<ReaderState>,
}

impl FakeClipboardReader {
    /// Sets what the next reads return.
    pub fn set(&self, snapshot: ClipboardSnapshot) {
        lock(&self.state).current = Some(snapshot);
    }

    /// Makes the next read fail as if the clipboard stayed locked.
    pub fn fail_next(&self) {
        lock(&self.state).fail_next = true;
    }

    /// Number of read attempts so far.
    pub fn reads(&self) -> usize {
        lock(&self.state).reads
    }
}

impl ClipboardReader for FakeClipboardReader {
    fn snapshot(&self) -> Result<ClipboardSnapshot, PortError> {
        let mut state = lock(&self.state);
        state.reads += 1;
        if std::mem::take(&mut state.fail_next) {
            return Err(PortError::Unavailable("clipboard locked".into()));
        }
        state
            .current
            .clone()
            .ok_or_else(|| PortError::Unavailable("empty fake clipboard".into()))
    }
}
