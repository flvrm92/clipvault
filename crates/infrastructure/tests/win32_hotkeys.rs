//! Global shortcuts with `RegisterHotKey` and snippets pasted by shortcut (HK-001..004,
//! SNP-003..005), against the real system.
//!
//! Its own test binary with a single test: it must own the foreground window (see
//! `win32_paste.rs`). Uses Ctrl+Alt+Shift+F22/F23, combinations no other app binds.
//! Ignored by default: `cargo test -p clipvault-infrastructure --test win32_hotkeys -- --ignored`
#![cfg(windows)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod win32_support;

use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use clipvault_application::capture::OwnWriteRegistry;
use clipvault_application::ports::{
    ClipboardContent, ClipboardReader, ClipboardWriter, HotkeyRegistrar, PortError, Suspension,
};
use clipvault_application::use_cases::{
    PasteOutcome, PasteSnippet, PasteSnippetDependencies, SnippetTarget,
};
use clipvault_domain::hotkey::{Hotkey, HotkeyOwner};
use clipvault_domain::ids::SnippetId;
use clipvault_infrastructure::clock::ThreadDelay;
use clipvault_infrastructure::win32::{
    Win32Clipboard, Win32FocusTracker, Win32HotkeyRegistrar, Win32KeystrokeSender,
};
use clipvault_test_support::{FixedSettings, InMemorySnippetRepository};
use win32_support::{
    EditWindow, VK_CONTROL, VK_F22, VK_F23, VK_MENU, VK_SHIFT, ensure_foreground, press_chord,
    send_keys, wait_until,
};

const NOTHING_WITHIN: Duration = Duration::from_millis(300);
const EVENT_WITHIN: Duration = Duration::from_secs(2);

#[test]
#[ignore = "touches global shortcuts, the clipboard, focus and keyboard input"]
fn snp_004_shortcuts_fire_and_snippets_paste_once_modifiers_are_released() {
    let clipboard = Win32Clipboard::new();
    let saved_text = clipboard.snapshot().ok().and_then(|s| s.text);
    let editor = EditWindow::open(200);
    assert!(
        ensure_foreground(&editor),
        "precondition: the editor must be in the foreground"
    );

    let (pressed_tx, pressed) = mpsc::channel();
    let registrar = Win32HotkeyRegistrar::start(move |owner| {
        let _ = pressed_tx.send(owner);
    })
    .unwrap();
    let open = Hotkey::parse("Ctrl+Alt+Shift+F22").unwrap();

    // HK-001: a registered shortcut fires.
    registrar.register(HotkeyOwner::OpenHistory, open).unwrap();
    press_chord(VK_F22);
    assert_eq!(
        pressed.recv_timeout(EVENT_WITHIN),
        Ok(HotkeyOwner::OpenHistory)
    );

    // HK-004: the same combination cannot be registered twice (another "app").
    let other = Win32HotkeyRegistrar::start(|_| {}).unwrap();
    assert!(matches!(
        other.register(HotkeyOwner::OpenHistory, open),
        Err(PortError::InUse(_))
    ));
    drop(other);

    // HK-002: suspended shortcuts do not fire; resumed ones do.
    registrar.set_suspension(Suspension::Suspended).unwrap();
    press_chord(VK_F22);
    assert!(pressed.recv_timeout(NOTHING_WITHIN).is_err());
    registrar.set_suspension(Suspension::Active).unwrap();
    press_chord(VK_F22);
    assert_eq!(
        pressed.recv_timeout(EVENT_WITHIN),
        Ok(HotkeyOwner::OpenHistory)
    );

    // SNP-003..005: the snippet shortcut pastes into the foreground window once the user
    // releases the modifiers, then the previous clipboard text comes back.
    clipboard
        .write(&ClipboardContent::Text("antes do snippet".into()))
        .unwrap();
    let snippets = Arc::new(InMemorySnippetRepository::default());
    let id: SnippetId = snippets.insert_body_for_tests("Teste", "Teste de texto");
    registrar
        .register(
            HotkeyOwner::Snippet(id),
            Hotkey::parse("Ctrl+Alt+Shift+F23").unwrap(),
        )
        .unwrap();
    let paste = PasteSnippet::new(PasteSnippetDependencies {
        snippets,
        reader: Arc::new(Win32Clipboard::new()),
        writer: Arc::new(Win32Clipboard::new()),
        focus: Arc::new(Win32FocusTracker),
        keys: Arc::new(Win32KeystrokeSender),
        own_writes: Arc::new(OwnWriteRegistry::default()),
        settings: Arc::new(FixedSettings::default()),
        delay: Arc::new(ThreadDelay),
    });

    send_keys(&[VK_CONTROL, VK_MENU, VK_SHIFT, VK_F23], false);
    send_keys(&[VK_F23], true);
    let releaser = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(200)); // SNP-004: modifiers still held
        send_keys(&[VK_SHIFT, VK_MENU, VK_CONTROL], true);
    });
    assert_eq!(
        pressed.recv_timeout(EVENT_WITHIN),
        Ok(HotkeyOwner::Snippet(id))
    );
    let outcome = paste.execute(id, SnippetTarget::Foreground).unwrap();
    releaser.join().unwrap();

    let pasted = wait_until(|| editor.text() == "Teste de texto");
    let restored = clipboard.snapshot().unwrap().text;
    let editor_text = editor.text();

    registrar.stop();
    drop(editor);
    if let Some(text) = saved_text {
        let _ = clipboard.write(&ClipboardContent::Text(text));
    }
    assert_eq!(outcome, PasteOutcome::Pasted);
    assert!(pasted, "editor text: {editor_text:?}");
    assert_eq!(restored.as_deref(), Some("antes do snippet"));
}
