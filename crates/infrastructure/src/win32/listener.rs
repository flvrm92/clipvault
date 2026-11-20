//! Clipboard change notifications (pattern: Active Object — specs/04 §P-13).
//!
//! A dedicated thread owns a message-only window registered with
//! `AddClipboardFormatListener` and forwards every `WM_CLIPBOARDUPDATE` to the capture worker.
//! Debouncing happens in the worker (CAP-012).

use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;

use clipvault_application::capture::WorkerMessage;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, RemoveClipboardFormatListener,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, PostMessageW, TranslateMessage, WM_APP,
};

use super::window::{MessageWindow, notify_on_this_thread};

/// Posted to the listener window to end its message loop.
const WM_STOP_LISTENER: u32 = WM_APP + 1;

/// A running clipboard listener; stops (and unregisters) on [`Self::stop`] or drop.
pub struct ClipboardListener {
    /// Window handle as an integer: `HWND` is not `Send`, but posting to it from any thread is
    /// allowed.
    hwnd: isize,
    thread: Option<JoinHandle<()>>,
}

impl ClipboardListener {
    /// Starts listening; every clipboard change sends [`WorkerMessage::ClipboardChanged`].
    ///
    /// # Errors
    /// The window could not be created or registered as a clipboard listener.
    pub fn start(sender: Sender<WorkerMessage>) -> std::io::Result<Self> {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<isize, String>>();
        let thread = std::thread::Builder::new()
            .name("clipboard-listener".into())
            .spawn(move || {
                notify_on_this_thread(sender);
                let window = match MessageWindow::create() {
                    Ok(window) => window,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.to_string()));
                        return;
                    }
                };
                // SAFETY: the window is alive for the whole message loop below.
                if let Err(error) = unsafe { AddClipboardFormatListener(window.hwnd()) } {
                    let _ = ready_tx.send(Err(error.to_string()));
                    return;
                }
                let _ = ready_tx.send(Ok(window.hwnd().0 as isize));
                run_message_loop();
                // SAFETY: registered above with the same, still alive, window.
                let _ = unsafe { RemoveClipboardFormatListener(window.hwnd()) };
            })?;
        match ready_rx.recv() {
            Ok(Ok(hwnd)) => Ok(Self {
                hwnd,
                thread: Some(thread),
            }),
            Ok(Err(message)) => {
                let _ = thread.join();
                Err(std::io::Error::other(message))
            }
            Err(_) => {
                let _ = thread.join();
                Err(std::io::Error::other("clipboard listener thread exited"))
            }
        }
    }

    /// Stops listening and waits for the thread to finish.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        if let Some(thread) = self.thread.take() {
            // SAFETY: posting a message to a window of another thread is allowed; if the
            // window is already gone the call fails harmlessly.
            let posted = unsafe {
                PostMessageW(
                    Some(HWND(self.hwnd as *mut _)),
                    WM_STOP_LISTENER,
                    WPARAM(0),
                    LPARAM(0),
                )
            };
            if posted.is_ok() {
                let _ = thread.join();
            }
        }
    }
}

impl Drop for ClipboardListener {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_message_loop() {
    let mut msg = MSG::default();
    loop {
        // SAFETY: `msg` is a valid out-parameter; retrieves messages of this thread.
        let result = unsafe { GetMessageW(&raw mut msg, None, 0, 0) };
        if result.0 <= 0 || msg.message == WM_STOP_LISTENER {
            break;
        }
        // SAFETY: standard dispatch of a message retrieved by GetMessageW.
        unsafe {
            let _ = TranslateMessage(&raw const msg);
            DispatchMessageW(&raw const msg);
        }
    }
}
