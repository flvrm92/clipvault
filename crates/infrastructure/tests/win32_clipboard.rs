//! Win32 clipboard adapters against the real system clipboard (specs/08 §2 "Integração Win32").
//!
//! Ignored by default: they replace the clipboard content (the previous text is restored).
//! Run serially: `cargo test -p clipvault-infrastructure --test win32_clipboard -- --ignored --test-threads=1`
#![cfg(windows)]
// Integration-test helpers are test code; raw Win32 calls build clipboard states that the
// adapter itself never writes (privacy formats, DIB-only images).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, unsafe_code)]

use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use clipvault_application::capture::{CaptureWorker, Debouncer, OwnWriteRegistry, WorkerMessage};
use clipvault_application::ports::{
    ClipboardContent, ClipboardReader, ClipboardWriter, HistoryQuery, HistoryRepository,
};
use clipvault_application::use_cases::{CaptureClipboard, CaptureDependencies};
use clipvault_domain::app_name::AppName;
use clipvault_domain::clip::ClipContent;
use clipvault_domain::image::ImagePixels;
use clipvault_infrastructure::clock::SystemClock;
use clipvault_infrastructure::dib::encode_dib;
use clipvault_infrastructure::sqlite::{Database, SqliteHistoryRepository};
use clipvault_infrastructure::win32::{ClipboardListener, Win32Clipboard};
use clipvault_test_support::{FixedSettings, InMemoryImageStore, RecordingEvents};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::{CF_DIB, CF_UNICODETEXT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
};
use windows::core::{PCWSTR, w};

static CLIPBOARD: Mutex<()> = Mutex::new(());

/// Serializes access to the global clipboard and restores the user's text afterwards.
struct Session {
    _lock: MutexGuard<'static, ()>,
    saved: Option<String>,
}

impl Session {
    fn start() -> Self {
        let lock = CLIPBOARD.lock().unwrap_or_else(PoisonError::into_inner);
        let saved = Win32Clipboard::new().snapshot().ok().and_then(|s| s.text);
        Self { _lock: lock, saved }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(text) = self.saved.take() {
            let _ = Win32Clipboard::new().write(&ClipboardContent::Text(text));
        }
    }
}

/// A hidden window used as clipboard owner by the raw helper; destroyed on drop.
struct RawOwner(HWND);

impl RawOwner {
    fn new() -> Self {
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                PCWSTR::null(),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
        }
        .unwrap();
        Self(hwnd)
    }
}

impl Drop for RawOwner {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

fn format(name: PCWSTR) -> u32 {
    unsafe { RegisterClipboardFormatW(name) }
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// Writes arbitrary formats, keeping `owner` as the clipboard owner.
fn write_raw(owner: &RawOwner, formats: &[(u32, Vec<u8>)]) {
    unsafe {
        OpenClipboard(Some(owner.0)).unwrap();
        EmptyClipboard().unwrap();
        for (format, bytes) in formats {
            let memory = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)).unwrap();
            let ptr = GlobalLock(memory).cast::<u8>();
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            let _ = GlobalUnlock(memory);
            if SetClipboardData(*format, Some(HANDLE(memory.0))).is_err() {
                let _ = GlobalFree(Some(memory));
                panic!("SetClipboardData failed");
            }
        }
        CloseClipboard().unwrap();
    }
}

fn translucent_image() -> ImagePixels {
    ImagePixels::new(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 100]).unwrap()
}

#[test]
#[ignore = "touches the system clipboard"]
fn cap_001_written_text_is_read_back_with_the_same_sequence() {
    let _session = Session::start();
    let clipboard = Win32Clipboard::new();

    let sequence = clipboard
        .write(&ClipboardContent::Text("ClipVault: olá, 世界".into()))
        .unwrap();
    let snapshot = clipboard.snapshot().unwrap();

    assert_eq!(snapshot.text.as_deref(), Some("ClipVault: olá, 世界"));
    assert_eq!(snapshot.sequence, sequence);
    assert!(!snapshot.privacy.any());
    assert_eq!(snapshot.image, None);
}

#[test]
#[ignore = "touches the system clipboard"]
fn pst_003_written_images_are_read_back_losslessly() {
    let _session = Session::start();
    let clipboard = Win32Clipboard::new();

    clipboard
        .write(&ClipboardContent::Image(translucent_image()))
        .unwrap();

    assert_eq!(
        clipboard.snapshot().unwrap().image,
        Some(translucent_image())
    );
}

#[test]
#[ignore = "touches the system clipboard"]
fn cap_002_bitmap_only_images_are_decoded() {
    let _session = Session::start();
    let owner = RawOwner::new();
    let opaque = ImagePixels::new(3, 2, vec![200; 24]).unwrap();

    write_raw(&owner, &[(u32::from(CF_DIB.0), encode_dib(&opaque))]);

    let image = Win32Clipboard::new().snapshot().unwrap().image.unwrap();
    assert_eq!((image.width(), image.height()), (3, 2));
}

#[test]
#[ignore = "touches the system clipboard"]
fn cap_007_password_manager_content_is_flagged_and_not_read() {
    let _session = Session::start();
    let owner = RawOwner::new();

    write_raw(
        &owner,
        &[
            (u32::from(CF_UNICODETEXT.0), utf16("hunter2")),
            (
                format(w!("ExcludeClipboardContentFromMonitorProcessing")),
                vec![0; 4],
            ),
        ],
    );
    let snapshot = Win32Clipboard::new().snapshot().unwrap();

    assert!(snapshot.privacy.exclude_from_monitor);
    assert_eq!(snapshot.text, None, "private content is never read");
}

#[test]
#[ignore = "touches the system clipboard"]
fn cap_007_can_include_in_clipboard_history_zero_is_detected() {
    let _session = Session::start();
    let owner = RawOwner::new();
    let history_flag = format(w!("CanIncludeInClipboardHistory"));

    write_raw(
        &owner,
        &[
            (u32::from(CF_UNICODETEXT.0), utf16("a")),
            (history_flag, 0_u32.to_le_bytes().to_vec()),
        ],
    );
    assert!(
        Win32Clipboard::new()
            .snapshot()
            .unwrap()
            .privacy
            .history_disallowed
    );

    write_raw(
        &owner,
        &[
            (u32::from(CF_UNICODETEXT.0), utf16("a")),
            (history_flag, 1_u32.to_le_bytes().to_vec()),
        ],
    );
    assert!(
        !Win32Clipboard::new()
            .snapshot()
            .unwrap()
            .privacy
            .history_disallowed
    );
}

#[test]
#[ignore = "touches the system clipboard"]
fn cap_011_the_owner_process_is_identified() {
    let _session = Session::start();
    let owner = RawOwner::new();

    write_raw(
        &owner,
        &[(u32::from(CF_UNICODETEXT.0), utf16("from this test"))],
    );
    let snapshot = Win32Clipboard::new().snapshot().unwrap();

    let this_exe = AppName::parse(&std::env::current_exe().unwrap().to_string_lossy()).unwrap();
    assert_eq!(snapshot.owner, Some(this_exe));
}

#[test]
#[ignore = "touches the system clipboard"]
fn cap_012_the_listener_reports_clipboard_changes_until_stopped() {
    let _session = Session::start();
    let (sender, receiver) = mpsc::channel();
    let listener = ClipboardListener::start(sender).unwrap();

    Win32Clipboard::new()
        .write(&ClipboardContent::Text("change".into()))
        .unwrap();
    let message = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    listener.stop();
    while receiver.try_recv().is_ok() {} // drain notifications of the same change
    Win32Clipboard::new()
        .write(&ClipboardContent::Text("after stop".into()))
        .unwrap();

    assert_eq!(message, WorkerMessage::ClipboardChanged);
    assert!(
        receiver.recv_timeout(Duration::from_millis(300)).is_err(),
        "no events after stop"
    );
}

struct Pipeline {
    history: Arc<SqliteHistoryRepository>,
    own_writes: Arc<OwnWriteRegistry>,
    inbox: mpsc::Sender<WorkerMessage>,
    worker: std::thread::JoinHandle<()>,
    listener: ClipboardListener,
}

fn start_pipeline() -> Pipeline {
    let history = Arc::new(SqliteHistoryRepository::new(
        Database::open_in_memory().unwrap(),
    ));
    let own_writes = Arc::new(OwnWriteRegistry::default());
    let capture = CaptureClipboard::new(CaptureDependencies {
        history: history.clone(),
        images: Arc::new(InMemoryImageStore::default()),
        settings: Arc::new(FixedSettings::default()),
        clock: Arc::new(SystemClock),
        events: Arc::new(RecordingEvents::default()),
        own_writes: own_writes.clone(),
    });
    let worker = CaptureWorker::new(
        Arc::new(Win32Clipboard::new()),
        capture,
        Arc::new(SystemClock),
        Debouncer::DEFAULT_QUIET_PERIOD,
    );
    let (inbox, receiver) = mpsc::channel();
    let listener = ClipboardListener::start(inbox.clone()).unwrap();
    let worker = std::thread::spawn(move || worker.run(&receiver));
    Pipeline {
        history,
        own_writes,
        inbox,
        worker,
        listener,
    }
}

impl Pipeline {
    fn stop(self) -> Arc<SqliteHistoryRepository> {
        self.listener.stop();
        self.inbox.send(WorkerMessage::Shutdown).unwrap();
        self.worker.join().unwrap();
        self.history
    }
}

fn texts(history: &SqliteHistoryRepository) -> Vec<String> {
    history
        .list(&HistoryQuery::first_page())
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| match item.content {
            ClipContent::Text(t) => Some(t.as_str().to_owned()),
            ClipContent::Image(_) => None,
        })
        .collect()
}

fn wait_until(condition: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

#[test]
#[ignore = "touches the system clipboard"]
fn cap_001_end_to_end_copied_text_reaches_the_database() {
    let _session = Session::start();
    let pipeline = start_pipeline();
    let owner = RawOwner::new();

    write_raw(
        &owner,
        &[(u32::from(CF_UNICODETEXT.0), utf16("copiado em outro app"))],
    );
    let stored = wait_until(|| texts(&pipeline.history) == ["copiado em outro app"]);

    let history = pipeline.stop();
    assert!(stored, "history: {:?}", texts(&history));
}

#[test]
#[ignore = "touches the system clipboard"]
fn cap_010_end_to_end_clipvault_own_writes_are_not_recorded() {
    let _session = Session::start();
    let pipeline = start_pipeline();

    let sequence = Win32Clipboard::new()
        .write(&ClipboardContent::Text("colado pelo ClipVault".into()))
        .unwrap();
    pipeline.own_writes.record(sequence);
    std::thread::sleep(Duration::from_millis(500));

    let history = pipeline.stop();
    assert_eq!(texts(&history), Vec::<String>::new());
}
