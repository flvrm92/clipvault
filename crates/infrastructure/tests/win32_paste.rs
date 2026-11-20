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

mod win32_support;

use std::sync::Arc;

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
use win32_support::{EditWindow, ensure_foreground, wait_until};
use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

#[test]
#[ignore = "touches the system clipboard, keyboard focus and input"]
fn pst_001_end_to_end_an_item_is_pasted_from_the_popup_into_the_target_window() {
    let clipboard = Win32Clipboard::new();
    let saved_text = clipboard.snapshot().ok().and_then(|s| s.text);

    let target_editor = EditWindow::open(120);
    let popup = EditWindow::open(520);
    let _ = unsafe { SetForegroundWindow(popup.handle()) };
    assert!(
        ensure_foreground(&popup),
        "precondition: the popup must be in the foreground"
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
