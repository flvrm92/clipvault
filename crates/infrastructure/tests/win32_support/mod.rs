//! Helpers shared by the Win32 test binaries: a real EDIT control and synthetic input.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    unsafe_code
)]

use std::sync::mpsc;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DispatchMessageW, ES_MULTILINE, GetForegroundWindow, GetMessageW, MSG,
    PostThreadMessageW, SendMessageW, TranslateMessage, WINDOW_STYLE, WM_CLOSE, WM_GETTEXT,
    WM_QUIT, WS_EX_TOPMOST, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};
use windows::core::w;

/// A visible top-level EDIT control on its own UI thread.
pub struct EditWindow {
    pub hwnd: isize,
    thread_id: u32,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl EditWindow {
    pub fn open(x: i32) -> Self {
        let (tx, rx) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let style = WINDOW_STYLE(WS_OVERLAPPEDWINDOW.0 | WS_VISIBLE.0 | ES_MULTILINE as u32);
            let hwnd = unsafe {
                CreateWindowExW(
                    // Topmost: a click-to-activate must hit this window, not one above it.
                    WS_EX_TOPMOST,
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

    pub fn handle(&self) -> HWND {
        HWND(self.hwnd as *mut _)
    }

    pub fn text(&self) -> String {
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

    pub fn is_foreground(&self) -> bool {
        (unsafe { GetForegroundWindow() }) == self.handle()
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

pub fn wait_until(condition: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

fn key(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                ..Default::default()
            },
        },
    }
}

/// Synthetic key presses (`down`) or releases (`up`), in order.
pub fn send_keys(keys: &[u16], up: bool) {
    let inputs: Vec<INPUT> = keys.iter().map(|vk| key(*vk, up)).collect();
    let sent = unsafe { SendInput(&inputs, i32::try_from(size_of::<INPUT>()).unwrap()) };
    assert_eq!(sent as usize, inputs.len(), "input blocked");
}

pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12;
pub const VK_F22: u16 = 0x85;
pub const VK_F23: u16 = 0x86;

/// Presses and releases Ctrl+Alt+Shift+`key`.
pub fn press_chord(key: u16) {
    send_keys(&[VK_CONTROL, VK_MENU, VK_SHIFT, key], false);
    send_keys(&[key, VK_SHIFT, VK_MENU, VK_CONTROL], true);
}

/// Makes `window` the foreground window. A process may only take the foreground while it owns
/// it or right after user input, so when the window did not get it on creation it is activated
/// the way a user would: a synthetic click on its title bar (the cursor is put back afterwards).
pub fn ensure_foreground(window: &EditWindow) -> bool {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect, SetCursorPos};

    if wait_until(|| window.is_foreground()) {
        return true;
    }
    let mut rect = RECT::default();
    let mut previous = POINT::default();
    unsafe {
        GetWindowRect(window.handle(), &raw mut rect).unwrap();
        let _ = GetCursorPos(&raw mut previous);
        SetCursorPos(i32::midpoint(rect.left, rect.right), rect.top + 12).unwrap();
    }
    let click = |flags| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    let inputs = [click(MOUSEEVENTF_LEFTDOWN), click(MOUSEEVENTF_LEFTUP)];
    unsafe {
        SendInput(&inputs, i32::try_from(size_of::<INPUT>()).unwrap());
    }
    let activated = wait_until(|| window.is_foreground());
    unsafe {
        let _ = SetCursorPos(previous.x, previous.y);
    }
    activated
}
