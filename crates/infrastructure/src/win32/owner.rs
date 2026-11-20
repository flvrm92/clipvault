//! Executable that owns the clipboard (CAP-009, CAP-011).

use clipvault_domain::app_name::AppName;
use windows::Win32::System::DataExchange::GetClipboardOwner;

use super::process::{image_name, open_process, window_owner_ids};

/// The executable of the window that last emptied the clipboard, when identifiable.
/// `None` for anonymous owners, exited processes or protected processes.
pub fn clipboard_owner() -> Option<AppName> {
    // SAFETY: no preconditions; returns a null handle when there is no owner.
    let hwnd = unsafe { GetClipboardOwner() }.ok()?;
    if hwnd.is_invalid() {
        return None;
    }
    let (pid, _) = window_owner_ids(hwnd)?;
    image_name(&open_process(pid)?)
}
