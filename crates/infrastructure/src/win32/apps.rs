//! [`RunningApps`] over the visible top-level windows (CAP-009).

use clipvault_application::ports::{PortError, RunningApps};
use clipvault_domain::app_name::AppName;
use windows::Win32::System::Threading::GetCurrentProcessId;

use super::focus::{is_user_window, top_level_windows};
use super::process::{image_name, open_process, window_owner_ids};

/// Lists the executables of the windows the user can see (ClipVault excluded).
#[derive(Debug, Clone, Copy, Default)]
pub struct Win32RunningApps;

impl RunningApps for Win32RunningApps {
    fn list(&self) -> Result<Vec<AppName>, PortError> {
        // SAFETY: no preconditions.
        let own_pid = unsafe { GetCurrentProcessId() };
        Ok(top_level_windows()
            .filter(|hwnd| is_user_window(*hwnd, own_pid))
            .filter_map(window_owner_ids)
            .filter_map(|(pid, _)| open_process(pid))
            .filter_map(|process| image_name(&process))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_009_running_apps_never_include_clipvault_itself() {
        let own = std::env::current_exe()
            .ok()
            .and_then(|p| AppName::parse(&p.to_string_lossy()).ok());

        let apps = Win32RunningApps.list().unwrap();

        assert!(own.is_some_and(|own| !apps.contains(&own)));
    }
}
