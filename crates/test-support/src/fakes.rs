//! In-memory fakes of the application ports (pattern: Fake — specs/04 §P-11).
//!
//! Each fake passes the same contract tests as its real adapter (`tests/fake_contracts.rs`).

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use clipvault_application::ports::{
    AppEvent, ClearScope, ClipboardContent, ClipboardReader, ClipboardSequence, ClipboardSnapshot,
    ClipboardWriter, Clock, Delay, EventPublisher, FocusTracker, HistoryQuery, HistoryRepository,
    HotkeyRegistrar, ImageStore, KeystrokeSender, KindFilter, Page, PortError, Removal,
    RunningApps, SettingsRepository, SnippetRepository, StoredImage, Suspension, TargetWindow,
    WindowHandle,
};
use clipvault_domain::app_name::AppName;
use clipvault_domain::clip::{ClipContent, ClipItem, NewClipItem, PinState};
use clipvault_domain::hash::ContentHash;
use clipvault_domain::hotkey::{Hotkey, HotkeyOwner};
use clipvault_domain::ids::{ClipId, SnippetId};
use clipvault_domain::image::{FileKey, ImagePixels, thumbnail_size};
use clipvault_domain::retention::RetentionCandidate;
use clipvault_domain::settings::Settings;
use clipvault_domain::snippet::{NewSnippet, Snippet, SnippetBody, SnippetName};
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
    /// Reported file size and the original pixels.
    files: BTreeMap<FileKey, (u64, ImagePixels)>,
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
        lock(&self.state).files.get(key).map(|(size, _)| *size)
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
        state.files.insert(key.clone(), (file_size, pixels.clone()));
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

    fn load(&self, key: &FileKey) -> Result<ImagePixels, PortError> {
        lock(&self.state)
            .files
            .get(key)
            .map(|(_, pixels)| pixels.clone())
            .ok_or(PortError::NotFound)
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

#[derive(Debug, Default)]
struct WriterState {
    writes: Vec<ClipboardContent>,
    next_sequence: u32,
    fail_next: bool,
}

/// Records what is written to the clipboard.
#[derive(Debug, Default)]
pub struct RecordingClipboardWriter {
    state: Mutex<WriterState>,
}

impl RecordingClipboardWriter {
    /// Contents written so far.
    pub fn writes(&self) -> Vec<ClipboardContent> {
        lock(&self.state).writes.clone()
    }

    /// Sequence number returned by the next write.
    pub fn next_sequence(&self, raw: u32) {
        lock(&self.state).next_sequence = raw;
    }

    /// Makes the next write fail as if the clipboard stayed locked.
    pub fn fail_next(&self) {
        lock(&self.state).fail_next = true;
    }
}

impl ClipboardWriter for RecordingClipboardWriter {
    fn write(&self, content: &ClipboardContent) -> Result<ClipboardSequence, PortError> {
        let mut state = lock(&self.state);
        if std::mem::take(&mut state.fail_next) {
            return Err(PortError::Unavailable("clipboard locked".into()));
        }
        state.writes.push(content.clone());
        Ok(ClipboardSequence::new(state.next_sequence))
    }
}

#[derive(Debug, Default)]
struct FocusState {
    target: Option<TargetWindow>,
    restored: Vec<WindowHandle>,
    fail_restore: bool,
}

/// A foreground window controlled by the test.
#[derive(Debug, Default)]
pub struct FakeFocusTracker {
    state: Mutex<FocusState>,
}

impl FakeFocusTracker {
    /// Sets what [`FocusTracker::capture_target`] returns.
    pub fn set_target(&self, target: Option<TargetWindow>) {
        lock(&self.state).target = target;
    }

    /// Makes every restore fail (window closed).
    pub fn fail_restore(&self) {
        lock(&self.state).fail_restore = true;
    }

    /// Windows restored so far.
    pub fn restored(&self) -> Vec<WindowHandle> {
        lock(&self.state).restored.clone()
    }
}

impl FocusTracker for FakeFocusTracker {
    fn capture_target(&self) -> Option<TargetWindow> {
        lock(&self.state).target
    }

    fn restore(&self, target: &TargetWindow) -> Result<(), PortError> {
        let mut state = lock(&self.state);
        if state.fail_restore {
            return Err(PortError::Unavailable("window closed".into()));
        }
        state.restored.push(target.handle);
        Ok(())
    }
}

#[derive(Debug, Default)]
struct KeystrokeState {
    pastes: usize,
    fail_next: bool,
}

/// Counts synthetic `Ctrl+V` keystrokes.
#[derive(Debug, Default)]
pub struct RecordingKeystrokes {
    state: Mutex<KeystrokeState>,
}

impl RecordingKeystrokes {
    /// Number of pastes sent.
    pub fn pastes(&self) -> usize {
        lock(&self.state).pastes
    }

    /// Makes the next paste fail.
    pub fn fail_next(&self) {
        lock(&self.state).fail_next = true;
    }
}

impl KeystrokeSender for RecordingKeystrokes {
    fn send_paste(&self) -> Result<(), PortError> {
        let mut state = lock(&self.state);
        if std::mem::take(&mut state.fail_next) {
            return Err(PortError::Unavailable("input blocked".into()));
        }
        state.pastes += 1;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct SnippetState {
    snippets: Vec<Snippet>,
    last_id: i64,
}

/// Snippets kept in memory.
#[derive(Debug, Default)]
pub struct InMemorySnippetRepository {
    state: Mutex<SnippetState>,
}

impl InMemorySnippetRepository {
    /// Inserts a snippet with a generated body.
    pub fn insert_for_tests(&self, name: &str, hotkey: Option<Hotkey>, enabled: bool) -> SnippetId {
        let new = NewSnippet {
            name: SnippetName::parse(name).unwrap(),
            body: SnippetBody::new(&format!("corpo de {name}")).unwrap(),
            hotkey,
            enabled,
        };
        self.insert(new, Timestamp::from_millis(0)).unwrap().id
    }

    /// Inserts an enabled snippet without shortcut.
    pub fn insert_body_for_tests(&self, name: &str, body: &str) -> SnippetId {
        let new = NewSnippet {
            name: SnippetName::parse(name).unwrap(),
            body: SnippetBody::new(body).unwrap(),
            hotkey: None,
            enabled: true,
        };
        self.insert(new, Timestamp::from_millis(0)).unwrap().id
    }

    /// Enables or disables a snippet.
    pub fn set_enabled_for_tests(&self, id: SnippetId, enabled: bool) {
        if let Some(s) = lock(&self.state).snippets.iter_mut().find(|s| s.id == id) {
            s.enabled = enabled;
        }
    }

    fn check_unique(
        state: &SnippetState,
        id: Option<SnippetId>,
        name: &SnippetName,
        hotkey: Option<Hotkey>,
    ) -> Result<(), PortError> {
        let others = state.snippets.iter().filter(|s| Some(s.id) != id);
        for other in others {
            if other.name.is_same_as(name) {
                return Err(PortError::InUse("snippet name".into()));
            }
            if hotkey.is_some() && other.hotkey == hotkey {
                return Err(PortError::InUse("snippet shortcut".into()));
            }
        }
        Ok(())
    }
}

impl SnippetRepository for InMemorySnippetRepository {
    fn list(&self) -> Result<Vec<Snippet>, PortError> {
        let mut snippets = lock(&self.state).snippets.clone();
        snippets.sort_by_key(|s| (s.order, s.id));
        Ok(snippets)
    }

    fn get(&self, id: SnippetId) -> Result<Option<Snippet>, PortError> {
        Ok(lock(&self.state)
            .snippets
            .iter()
            .find(|s| s.id == id)
            .cloned())
    }

    fn insert(&self, snippet: NewSnippet, _at: Timestamp) -> Result<Snippet, PortError> {
        let mut state = lock(&self.state);
        Self::check_unique(&state, None, &snippet.name, snippet.hotkey)?;
        state.last_id += 1;
        let order = state
            .snippets
            .iter()
            .map(|s| s.order + 1)
            .max()
            .unwrap_or(0);
        let saved = Snippet {
            id: SnippetId::new(state.last_id),
            name: snippet.name,
            body: snippet.body,
            hotkey: snippet.hotkey,
            enabled: snippet.enabled,
            order,
        };
        state.snippets.push(saved.clone());
        Ok(saved)
    }

    fn update(&self, snippet: &Snippet, _at: Timestamp) -> Result<(), PortError> {
        let mut state = lock(&self.state);
        Self::check_unique(&state, Some(snippet.id), &snippet.name, snippet.hotkey)?;
        let slot = state
            .snippets
            .iter_mut()
            .find(|s| s.id == snippet.id)
            .ok_or(PortError::NotFound)?;
        *slot = snippet.clone();
        Ok(())
    }

    fn delete(&self, id: SnippetId) -> Result<(), PortError> {
        let mut state = lock(&self.state);
        let index = state
            .snippets
            .iter()
            .position(|s| s.id == id)
            .ok_or(PortError::NotFound)?;
        state.snippets.remove(index);
        Ok(())
    }
}

#[derive(Debug, Default)]
struct RegistrarState {
    registered: BTreeMap<String, (HotkeyOwner, Hotkey)>,
    blocked: Vec<Hotkey>,
    suspended: bool,
}

/// Global shortcuts kept in memory; combinations can be "taken by another app".
#[derive(Debug, Default)]
pub struct FakeHotkeyRegistrar {
    state: Mutex<RegistrarState>,
}

fn owner_key(owner: HotkeyOwner) -> String {
    format!("{owner:?}")
}

impl FakeHotkeyRegistrar {
    /// Simulates another application holding `hotkey` (HK-004).
    pub fn block(&self, hotkey: Hotkey) {
        lock(&self.state).blocked.push(hotkey);
    }

    /// The hotkey registered for `owner`.
    pub fn registered(&self, owner: HotkeyOwner) -> Option<Hotkey> {
        lock(&self.state)
            .registered
            .get(&owner_key(owner))
            .map(|(_, h)| *h)
    }

    /// Number of registered shortcuts.
    pub fn count(&self) -> usize {
        lock(&self.state).registered.len()
    }

    /// Whether shortcuts are suspended.
    pub fn is_suspended(&self) -> bool {
        lock(&self.state).suspended
    }
}

impl HotkeyRegistrar for FakeHotkeyRegistrar {
    fn register(&self, owner: HotkeyOwner, hotkey: Hotkey) -> Result<(), PortError> {
        let mut state = lock(&self.state);
        if state.blocked.contains(&hotkey) {
            return Err(PortError::InUse(format!(
                "{hotkey} is used by another application"
            )));
        }
        state.registered.insert(owner_key(owner), (owner, hotkey));
        Ok(())
    }

    fn unregister(&self, owner: HotkeyOwner) -> Result<(), PortError> {
        lock(&self.state).registered.remove(&owner_key(owner));
        Ok(())
    }

    fn set_suspension(&self, suspension: Suspension) -> Result<(), PortError> {
        lock(&self.state).suspended = suspension == Suspension::Suspended;
        Ok(())
    }
}

/// Records requested waits instead of sleeping.
#[derive(Debug, Default)]
pub struct RecordingDelay {
    waits: Mutex<Vec<Duration>>,
}

impl RecordingDelay {
    /// Waits requested so far.
    pub fn waits(&self) -> Vec<Duration> {
        lock(&self.waits).clone()
    }
}

impl Delay for RecordingDelay {
    fn wait(&self, duration: Duration) {
        lock(&self.waits).push(duration);
    }
}

/// A fixed list of running applications (CAP-009).
#[derive(Debug, Default)]
pub struct FakeRunningApps {
    apps: Vec<AppName>,
}

impl FakeRunningApps {
    /// Reports `apps`, in this order.
    pub const fn new(apps: Vec<AppName>) -> Self {
        Self { apps }
    }
}

impl RunningApps for FakeRunningApps {
    fn list(&self) -> Result<Vec<AppName>, PortError> {
        Ok(self.apps.clone())
    }
}
