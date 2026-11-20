//! Process queries shared by the clipboard-owner lookup and the elevation check.

use clipvault_domain::app_name::AppName;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::core::PWSTR;

/// An owned kernel handle, closed on drop.
pub struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by this value and is closed exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Process and thread ids of the window's creator; `None` for invalid windows.
pub fn window_owner_ids(hwnd: HWND) -> Option<(u32, u32)> {
    let mut pid = 0_u32;
    // SAFETY: `pid` is a valid out-pointer for the duration of the call.
    let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&raw mut pid)) };
    (pid != 0 && thread != 0).then_some((pid, thread))
}

/// Opens a process with limited query rights (enough for the image name and the token).
pub fn open_process(pid: u32) -> Option<OwnedHandle> {
    // SAFETY: requests the least-privileged access right; the result is owned immediately.
    unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
        .ok()
        .map(OwnedHandle)
}

/// Executable name of a process.
pub fn image_name(process: &OwnedHandle) -> Option<AppName> {
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
    AppName::parse(&String::from_utf16_lossy(buffer.get(..len as usize)?)).ok()
}

fn token_is_elevated(process: HANDLE) -> Option<bool> {
    let mut token = HANDLE::default();
    // SAFETY: `token` receives a new handle that is wrapped and closed below.
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &raw mut token) }.ok()?;
    let token = OwnedHandle(token);
    let mut elevation = TOKEN_ELEVATION::default();
    let mut returned = 0_u32;
    // SAFETY: the output buffer is a TOKEN_ELEVATION of the size passed.
    unsafe {
        GetTokenInformation(
            token.0,
            TokenElevation,
            Some((&raw mut elevation).cast()),
            u32::try_from(size_of::<TOKEN_ELEVATION>()).ok()?,
            &raw mut returned,
        )
    }
    .ok()?;
    Some(elevation.TokenIsElevated != 0)
}

/// Whether `process` runs elevated while ClipVault does not: Windows (UIPI) then blocks the
/// synthetic input ClipVault would send to it (PST-006). Unknown counts as not elevated.
pub fn is_more_privileged(process: &OwnedHandle) -> bool {
    // SAFETY: the pseudo handle of the current process needs no closing.
    let current = unsafe { GetCurrentProcess() };
    let ours = token_is_elevated(current).unwrap_or(false);
    let theirs = token_is_elevated(process.0).unwrap_or(false);
    theirs && !ours
}
