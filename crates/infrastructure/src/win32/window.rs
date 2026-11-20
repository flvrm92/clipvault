//! Hidden message-only windows: clipboard owner for writes and receiver of
//! `WM_CLIPBOARDUPDATE` for the listener.

use std::cell::RefCell;
use std::sync::OnceLock;
use std::sync::mpsc::Sender;

use clipvault_application::capture::WorkerMessage;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_MESSAGE, RegisterClassW, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_CLIPBOARDUPDATE, WNDCLASSW,
};
use windows::core::{PCWSTR, w};

const CLASS_NAME: PCWSTR = w!("ClipVaultMessageWindow");

thread_local! {
    /// Destination of clipboard notifications for windows created on this thread.
    static NOTIFY: RefCell<Option<Sender<WorkerMessage>>> = const { RefCell::new(None) };
}

/// Routes `WM_CLIPBOARDUPDATE` received by this thread's windows to `sender`.
pub fn notify_on_this_thread(sender: Sender<WorkerMessage>) {
    NOTIFY.with(|slot| *slot.borrow_mut() = Some(sender));
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_CLIPBOARDUPDATE {
        NOTIFY.with(|slot| {
            if let Some(sender) = slot.borrow().as_ref() {
                // The worker may already be gone during shutdown; nothing to do then.
                let _ = sender.send(WorkerMessage::ClipboardChanged);
            }
        });
        return LRESULT(0);
    }
    // SAFETY: forwards the unmodified arguments Windows passed to this window procedure.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn register_class() -> windows::core::Result<()> {
    static REGISTERED: OnceLock<Result<(), windows::core::Error>> = OnceLock::new();
    REGISTERED
        .get_or_init(|| {
            // SAFETY: a null module name returns the handle of the current executable.
            let instance = unsafe { GetModuleHandleW(None) }?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance.into(),
                lpszClassName: CLASS_NAME,
                ..Default::default()
            };
            // SAFETY: `class` is fully initialized and outlives the call; the class name is a
            // static string.
            if unsafe { RegisterClassW(&raw const class) } == 0 {
                return Err(windows::core::Error::from_thread());
            }
            Ok(())
        })
        .clone()
}

/// A message-only window, destroyed on drop. Must be dropped on the thread that created it.
pub struct MessageWindow {
    hwnd: HWND,
}

impl MessageWindow {
    /// Creates a window on the current thread.
    pub fn create() -> windows::core::Result<Self> {
        register_class()?;
        // SAFETY: the class is registered; HWND_MESSAGE makes it invisible and message-only.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                CLASS_NAME,
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
        }?;
        Ok(Self { hwnd })
    }

    pub const fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for MessageWindow {
    fn drop(&mut self) {
        // SAFETY: the window was created by this value on this thread and not destroyed yet.
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}
