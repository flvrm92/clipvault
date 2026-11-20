//! End-to-end paste with the real clipboard, focus and keyboard (PST-001, HK-005).
//!
//! Its own test binary on purpose: Windows lets a process change the foreground window only
//! while it owns the foreground (in the app: the popup the user just pressed Enter in). A fresh
//! process gets that right for its first window, so this binary holds a single test that
//! reproduces the production sequence: our "popup" is in the foreground and pastes into
//! another window.
//!
//! Ignored by default: `cargo test -p clipvault-infrastructure --test win32_paste -- --ignored`
#![cfg(windows)]
// Integration-test helpers are test code; raw Win32 calls create the windows involved.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, unsafe_code)]

use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use clipvault_application::capture::OwnWriteRegistry;
use clipvault_application::ports::{
    ClipboardContent, ClipboardReader, ClipboardWriter, FocusTracker, HistoryRepository,
    ScreenPoint, TargetWindow, WindowHandle,
};
use clipvault_application::use_cases::{PasteDependencies, PasteItem, PasteMode, PasteOutcome};
use clipvault_infrastructure::clock::SystemClock;
use clipvault_infrastructure::sqlite::{Database, SqliteHistoryRepository};
use clipvault_infrastructure::win32::{Win32Clipboard, Win32FocusTracker, Win32KeystrokeSender};
use clipvault_test_support::contracts::history::text_item;
use clipvault_test_support::{InMemoryImageStore, RecordingEvents};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DispatchMessageW, ES_MULTILINE, GetForegroundWindow, GetMessageW, MSG,
    PostThreadMessageW, SendMessageW, SetForegroundWindow, TranslateMessage, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_CLOSE, WM_GETTEXT, WM_QUIT, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};
use windows::core::w;

/// A visible top-level EDIT control on its own UI thread.
struct EditWindow {
    hwnd: isize,
    thread_id: u32,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl EditWindow {
    fn open(x: i32) -> Self {
        let (tx, rx) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let style = WINDOW_STYLE(WS_OVERLAPPEDWINDOW.0 | WS_VISIBLE.0 | ES_MULTILINE as u32);
            let hwnd = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("EDIT"),
                    w!(""),
                    style,
                    x,
                    200,
                    360,
                    200,
                    None,
                    None,
                    None,
                    None,
                )
            }
            .unwrap();
            tx.send((hwnd.0 as isize, unsafe { GetCurrentThreadId() }))
                .unwrap();
            let mut msg = MSG::default();
            while unsafe { GetMessageW(&raw mut msg, None, 0, 0) }.0 > 0 {
                unsafe {
                    let _ = TranslateMessage(&raw const msg);
                    DispatchMessageW(&raw const msg);
                }
            }
        });
        let (hwnd, thread_id) = rx.recv().unwrap();
        Self {
            hwnd,
            thread_id,
            thread: Some(thread),
        }
    }

    fn handle(&self) -> HWND {
        HWND(self.hwnd as *mut _)
    }

    fn text(&self) -> String {
        let mut buffer = [0_u16; 512];
        let len = unsafe {
            SendMessageW(
                self.handle(),
                WM_GETTEXT,
                Some(WPARAM(buffer.len())),
                Some(LPARAM(buffer.as_mut_ptr() as isize)),
            )
        };
        String::from_utf16_lossy(&buffer[..usize::try_from(len.0).unwrap()])
    }
}

impl Drop for EditWindow {
    fn drop(&mut self) {
        unsafe {
            SendMessageW(self.handle(), WM_CLOSE, None, None);
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
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

fn foreground_is(window: &EditWindow) -> bool {
    (unsafe { GetForegroundWindow() }) == window.handle()
}

#[test]
#[ignore = "touches the system clipboard, keyboard focus and input"]
fn pst_001_end_to_end_an_item_is_pasted_from_the_popup_into_the_target_window() {
    let clipboard = Win32Clipboard::new();
    let saved_text = clipboard.snapshot().ok().and_then(|s| s.text);

    let target_editor = EditWindow::open(120);
    let popup = EditWindow::open(520);
    let _ = unsafe { SetForegroundWindow(popup.handle()) };
    assert!(
        wait_until(|| foreground_is(&popup)),
        "precondition: this process must own the foreground (run the binary on its own)"
    );

    // HK-005: ClipVault's own windows are never paste targets.
    assert_eq!(Win32FocusTracker.capture_target(), None);

    let history = Arc::new(SqliteHistoryRepository::new(
        Database::open_in_memory().unwrap(),
    ));
    let id = history
        .insert(text_item("colado pelo ClipVault ✓", 1))
        .unwrap();
    let paste = PasteItem::new(PasteDependencies {
        history,
        images: Arc::new(InMemoryImageStore::default()),
        writer: Arc::new(Win32Clipboard::new()),
        focus: Arc::new(Win32FocusTracker),
        keys: Arc::new(Win32KeystrokeSender),
        own_writes: Arc::new(OwnWriteRegistry::default()),
        clock: Arc::new(SystemClock),
        events: Arc::new(RecordingEvents::default()),
    });
    let target = TargetWindow {
        handle: WindowHandle::new(target_editor.hwnd),
        caret: None,
        cursor: ScreenPoint::default(),
        elevated: false,
    };

    let outcome = paste.execute(id, PasteMode::AsIs, Some(&target)).unwrap();
    let pasted = wait_until(|| target_editor.text() == "colado pelo ClipVault ✓");
    let editor_text = target_editor.text();

    drop(popup);
    drop(target_editor);
    if let Some(text) = saved_text {
        let _ = clipboard.write(&ClipboardContent::Text(text));
    }
    assert_eq!(outcome, PasteOutcome::Pasted);
    assert!(pasted, "target text: {editor_text:?}");
}
