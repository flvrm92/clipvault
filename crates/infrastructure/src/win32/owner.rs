//! Executable that owns the clipboard (CAP-009, CAP-011).

use clipvault_domain::app_name::AppName;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::DataExchange::GetClipboardOwner;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::core::PWSTR;

struct ProcessHandle(HANDLE);

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        // SAFETY: the handle was returned by OpenProcess and is closed exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// The executable of the window that last emptied the clipboard, when identifiable.
/// `None` for anonymous owners, exited processes or protected processes.
pub fn clipboard_owner() -> Option<AppName> {
    // SAFETY: no preconditions; returns a null handle when there is no owner.
    let hwnd = unsafe { GetClipboardOwner() }.ok()?;
    if hwnd.is_invalid() {
        return None;
    }
    let mut pid = 0_u32;
    // SAFETY: `pid` is a valid out-pointer for the duration of the call.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&raw mut pid)) };
    if pid == 0 {
        return None;
    }
    // SAFETY: limited query rights are enough to read the image name; the handle is wrapped
    // immediately so it is always closed.
    let process =
        ProcessHandle(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?);
    let mut buffer = [0_u16; 1024];
    let mut len = u32::try_from(buffer.len()).ok()?;
    // SAFETY: the buffer and its length describe valid writable memory; `len` is updated to
    // the number of characters written.
    unsafe {
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &raw mut len,
        )
    }
    .ok()?;
    let path = String::from_utf16_lossy(buffer.get(..len as usize)?);
    AppName::parse(&path).ok()
}
