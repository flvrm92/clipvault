//! [`FocusTracker`] over the Win32 foreground window (specs/03 §5.2, HK-005, PST-005/006).

use std::time::{Duration, Instant};

use clipvault_application::ports::{
    FocusTracker, PortError, ScreenPoint, TargetWindow, WindowHandle,
};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GUITHREADINFO, GW_HWNDNEXT, GWL_EXSTYLE, GetClassNameW, GetCursorPos, GetForegroundWindow,
    GetGUIThreadInfo, GetTopWindow, GetWindow, GetWindowLongPtrW, GetWindowTextLengthW, IsIconic,
    IsWindow, IsWindowVisible, SW_RESTORE, SetForegroundWindow, ShowWindow, WS_EX_TOOLWINDOW,
};

use super::process::{is_more_privileged, open_process, window_owner_ids};

/// How long to wait for Windows to report the restored window as foreground.
const ACTIVATION_TIMEOUT: Duration = Duration::from_millis(300);

const fn to_hwnd(handle: WindowHandle) -> HWND {
    HWND(handle.get() as *mut _)
}

fn caret_position(thread: u32) -> Option<ScreenPoint> {
    let mut info = GUITHREADINFO {
        cbSize: u32::try_from(size_of::<GUITHREADINFO>()).ok()?,
        ..Default::default()
    };
    // SAFETY: `info` is initialized with its size as the API requires.
    unsafe { GetGUIThreadInfo(thread, &raw mut info) }.ok()?;
    if info.hwndCaret.is_invalid() {
        return None;
    }
    let mut point = POINT {
        x: info.rcCaret.left,
        y: info.rcCaret.bottom,
    };
    // SAFETY: converts a point of a live window's client area; `point` is a valid in/out pointer.
    if !unsafe { ClientToScreen(info.hwndCaret, &raw mut point) }.as_bool() {
        return None;
    }
    Some(ScreenPoint {
        x: point.x,
        y: point.y,
    })
}

fn cursor_position() -> ScreenPoint {
    let mut point = POINT::default();
    // SAFETY: `point` is a valid out-pointer.
    let _ = unsafe { GetCursorPos(&raw mut point) };
    ScreenPoint {
        x: point.x,
        y: point.y,
    }
}

fn is_foreground(hwnd: HWND) -> bool {
    // SAFETY: no preconditions.
    (unsafe { GetForegroundWindow() }) == hwnd
}

fn wait_until_foreground(hwnd: HWND) -> bool {
    let deadline = Instant::now() + ACTIVATION_TIMEOUT;
    while Instant::now() < deadline {
        if is_foreground(hwnd) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    is_foreground(hwnd)
}

/// Taskbar, notification-area and desktop windows: never paste targets.
const SHELL_CLASSES: [&str; 6] = [
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "NotifyIconOverflowWindow",
    "TopLevelWindowForOverflowXamlIsland",
    "Progman",
    "WorkerW",
];

fn class_name(hwnd: HWND) -> String {
    let mut buffer = [0_u16; 128];
    // SAFETY: the buffer is writable; the call returns the number of characters copied.
    let len = unsafe { GetClassNameW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..usize::try_from(len).unwrap_or(0)])
}

fn is_shell(hwnd: HWND) -> bool {
    SHELL_CLASSES.contains(&class_name(hwnd).as_str())
}

/// A visible, titled, non-tool, non-cloaked top-level window of another process; minimized
/// windows included. Also used to list the applications that can be excluded (CAP-009).
pub(super) fn is_user_window(hwnd: HWND, own_pid: u32) -> bool {
    let mut cloaked = 0_u32;
    // SAFETY: plain queries on a window handle obtained from the z-order walk; the cloaked
    // attribute is a DWORD written into `cloaked`.
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE).cast_unsigned();
        let _ = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&raw mut cloaked).cast(),
            u32::try_from(size_of::<u32>()).unwrap_or(4),
        );
        IsWindowVisible(hwnd).as_bool()
            && GetWindowTextLengthW(hwnd) > 0
            && ex_style & WS_EX_TOOLWINDOW.0 as usize == 0
            && cloaked == 0
            && !is_shell(hwnd)
            && window_owner_ids(hwnd).is_some_and(|(pid, _)| pid != own_pid)
    }
}

/// A user window that is not minimized: a possible paste target.
fn is_app_window(hwnd: HWND, own_pid: u32) -> bool {
    // SAFETY: plain query on a window handle.
    is_user_window(hwnd, own_pid) && !unsafe { IsIconic(hwnd) }.as_bool()
}

/// Top-level windows in z-order, top first (bounded walk).
pub(super) fn top_level_windows() -> impl Iterator<Item = HWND> {
    // SAFETY: walks the top-level z-order; each handle is only queried.
    let first = unsafe { GetTopWindow(None) }.ok();
    std::iter::successors(first, |hwnd| {
        // SAFETY: as above.
        unsafe { GetWindow(*hwnd, GW_HWNDNEXT) }.ok()
    })
    .take_while(|h| !h.is_invalid())
    .take(1_000)
}

/// The top-most application window, used when the shell (e.g. the tray icon that opened the
/// popup) owns the foreground: that is the window the user was working in.
fn topmost_app_window(own_pid: u32) -> Option<HWND> {
    top_level_windows().find(|hwnd| is_app_window(*hwnd, own_pid))
}

/// Tracks the window that receives pastes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Win32FocusTracker;

impl FocusTracker for Win32FocusTracker {
    fn capture_target(&self) -> Option<TargetWindow> {
        // SAFETY: no preconditions.
        let own_pid = unsafe { GetCurrentProcessId() };
        // SAFETY: no preconditions; may return a null handle.
        let foreground = unsafe { GetForegroundWindow() };
        let hwnd = if foreground.is_invalid() || is_shell(foreground) {
            topmost_app_window(own_pid)?
        } else {
            foreground
        };
        let (pid, thread) = window_owner_ids(hwnd)?;
        if pid == own_pid {
            return None; // the popup itself is never a paste target
        }
        let elevated = open_process(pid).is_some_and(|p| is_more_privileged(&p));
        Some(TargetWindow {
            handle: WindowHandle::new(hwnd.0 as isize),
            caret: caret_position(thread),
            cursor: cursor_position(),
            elevated,
        })
    }

    fn restore(&self, target: &TargetWindow) -> Result<(), PortError> {
        let hwnd = to_hwnd(target.handle);
        // SAFETY: IsWindow accepts any value and reports whether it is a live window.
        if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            return Err(PortError::Unavailable("target window was closed".into()));
        }
        // SAFETY: the window is alive; restoring a minimized window before activating it.
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
        if wait_until_foreground(hwnd) {
            return Ok(());
        }
        // Windows may refuse activation from a background thread; sharing input state with the
        // current foreground thread lifts that restriction for this call.
        // SAFETY: the threads are attached and detached around a single activation.
        unsafe {
            let foreground_thread = window_owner_ids(GetForegroundWindow()).map(|(_, t)| t);
            let current = GetCurrentThreadId();
            if let Some(other) = foreground_thread.filter(|t| *t != current) {
                let _ = AttachThreadInput(current, other, true);
                let _ = SetForegroundWindow(hwnd);
                let _ = AttachThreadInput(current, other, false);
            }
        }
        if wait_until_foreground(hwnd) {
            Ok(())
        } else {
            Err(PortError::Unavailable(
                "target window could not be activated".into(),
            ))
        }
    }
}
