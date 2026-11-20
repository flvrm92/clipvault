//! [`StartupManager`] adapters (CFG-003, ADR-0008): the MSIX `StartupTask` when the process has
//! package identity, otherwise the user's `Run` registry key.

use std::sync::Arc;

use clipvault_application::ports::{PortError, StartupManager, StartupState};
use windows::ApplicationModel::{StartupTask, StartupTaskState};
use windows::Win32::Foundation::{APPMODEL_ERROR_NO_PACKAGE, ERROR_FILE_NOT_FOUND, WIN32_ERROR};
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW,
    RegSetKeyValueW,
};
use windows::core::HSTRING;

/// `TaskId` of the manifest's `windows.startupTask` extension (specs/09 §4).
const TASK_ID: &str = "ClipVaultStartup";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const VALUE_NAME: &str = "ClipVault";

fn unavailable(what: &str, error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(format!("{what}: {error}"))
}

/// Whether the process runs with MSIX package identity.
pub fn has_package_identity() -> bool {
    let mut length = 0_u32;
    // SAFETY: a zero length with no buffer only asks for the required size.
    let status = unsafe { GetCurrentPackageFullName(&raw mut length, None) };
    status != APPMODEL_ERROR_NO_PACKAGE
}

/// The adapter for this process (ADR-0008).
pub fn startup_manager() -> Arc<dyn StartupManager> {
    if has_package_identity() {
        // A Run entry left by an unpackaged build would start a second copy (ADR-0008).
        if let Some(run) = RunKeyStartup::for_current_exe() {
            let _ = run.disable();
        }
        Arc::new(MsixStartupTask)
    } else {
        match RunKeyStartup::for_current_exe() {
            Some(run) => Arc::new(run),
            None => Arc::new(Unsupported),
        }
    }
}

/// The MSIX `StartupTask` declared in the manifest.
#[derive(Debug, Clone, Copy, Default)]
pub struct MsixStartupTask;

impl MsixStartupTask {
    fn task() -> Result<StartupTask, PortError> {
        StartupTask::GetAsync(&HSTRING::from(TASK_ID))
            .and_then(|operation| operation.join())
            .map_err(|e| unavailable("startup task", e))
    }
}

fn from_task_state(state: StartupTaskState) -> StartupState {
    match state {
        StartupTaskState::Enabled => StartupState::Enabled,
        StartupTaskState::DisabledByUser => StartupState::DisabledByUser,
        StartupTaskState::DisabledByPolicy => StartupState::DisabledByPolicy,
        StartupTaskState::EnabledByPolicy => StartupState::EnabledByPolicy,
        _ => StartupState::Disabled,
    }
}

impl StartupManager for MsixStartupTask {
    fn state(&self) -> Result<StartupState, PortError> {
        let state = Self::task()?
            .State()
            .map_err(|e| unavailable("startup task state", e))?;
        Ok(from_task_state(state))
    }

    fn enable(&self) -> Result<StartupState, PortError> {
        let state = Self::task()?
            .RequestEnableAsync()
            .and_then(|operation| operation.join())
            .map_err(|e| unavailable("startup task enable", e))?;
        Ok(from_task_state(state))
    }

    fn disable(&self) -> Result<StartupState, PortError> {
        Self::task()?
            .Disable()
            .map_err(|e| unavailable("startup task disable", e))?;
        self.state()
    }
}

/// The user's `Run` key, for processes without package identity.
#[derive(Debug, Clone)]
pub struct RunKeyStartup {
    run_key: HSTRING,
    approved_key: HSTRING,
    value_name: HSTRING,
    command: String,
}

impl RunKeyStartup {
    /// Starts the running executable, quoted.
    pub fn for_current_exe() -> Option<Self> {
        let exe = std::env::current_exe().ok()?;
        Some(Self::with_keys(
            RUN_KEY,
            APPROVED_KEY,
            VALUE_NAME,
            format!("\"{}\"", exe.display()),
        ))
    }

    /// Uses other keys under `HKEY_CURRENT_USER` (tests).
    pub fn with_keys(run_key: &str, approved_key: &str, value_name: &str, command: String) -> Self {
        Self {
            run_key: HSTRING::from(run_key),
            approved_key: HSTRING::from(approved_key),
            value_name: HSTRING::from(value_name),
            command,
        }
    }

    fn has_run_value(&self) -> bool {
        let mut size = 0_u32;
        // SAFETY: asks only for the size of the value; no buffer is written.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                &self.run_key,
                &self.value_name,
                RRF_RT_REG_SZ,
                None,
                None,
                Some(&raw mut size),
            )
        };
        status.is_ok()
    }

    /// The Task Manager / Settings switch: an odd first byte means "disabled by the user".
    fn disabled_by_user(&self) -> bool {
        let mut data = [0_u8; 12];
        let mut size = u32::try_from(data.len()).unwrap_or(0);
        // SAFETY: `data` and `size` describe a writable byte buffer.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                &self.approved_key,
                &self.value_name,
                RRF_RT_REG_BINARY,
                None,
                Some(data.as_mut_ptr().cast()),
                Some(&raw mut size),
            )
        };
        status.is_ok() && size > 0 && data[0] & 1 == 1
    }
}

fn check(status: WIN32_ERROR, what: &str) -> Result<(), PortError> {
    if status.is_ok() || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(unavailable(what, status.0))
    }
}

impl StartupManager for RunKeyStartup {
    fn state(&self) -> Result<StartupState, PortError> {
        Ok(if !self.has_run_value() {
            StartupState::Disabled
        } else if self.disabled_by_user() {
            StartupState::DisabledByUser
        } else {
            StartupState::Enabled
        })
    }

    fn enable(&self) -> Result<StartupState, PortError> {
        if self.disabled_by_user() && self.has_run_value() {
            return Ok(StartupState::DisabledByUser);
        }
        let data: Vec<u16> = self.command.encode_utf16().chain([0]).collect();
        let bytes = u32::try_from(data.len() * 2).map_err(|e| unavailable("run key", e))?;
        // SAFETY: `data` is a NUL-terminated UTF-16 string of `bytes` bytes.
        let status = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                &self.run_key,
                &self.value_name,
                REG_SZ.0,
                Some(data.as_ptr().cast()),
                bytes,
            )
        };
        check(status, "run key write")?;
        self.state()
    }

    fn disable(&self) -> Result<StartupState, PortError> {
        // SAFETY: deletes one value under a key owned by the current user.
        let status =
            unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, &self.run_key, &self.value_name) };
        check(status, "run key delete")?;
        self.state()
    }
}

/// No executable path: starting with Windows cannot be offered.
#[derive(Debug, Clone, Copy)]
struct Unsupported;

impl StartupManager for Unsupported {
    fn state(&self) -> Result<StartupState, PortError> {
        Ok(StartupState::DisabledByPolicy)
    }

    fn enable(&self) -> Result<StartupState, PortError> {
        self.state()
    }

    fn disable(&self) -> Result<StartupState, PortError> {
        self.state()
    }
}

#[cfg(test)]
mod tests {
    use windows::Win32::System::Registry::{REG_BINARY, RegDeleteTreeW};

    use super::*;

    /// A throwaway key tree under `HKCU\Software\ClipVaultTests`, removed on drop.
    struct TestKeys {
        root: String,
    }

    impl TestKeys {
        fn new(name: &str) -> Self {
            Self {
                root: format!(r"Software\ClipVaultTests\{name}-{}", std::process::id()),
            }
        }

        fn startup(&self) -> RunKeyStartup {
            RunKeyStartup::with_keys(
                &format!(r"{}\Run", self.root),
                &format!(r"{}\Approved", self.root),
                "ClipVault",
                r#""C:\Apps\ClipVault.exe""#.into(),
            )
        }

        fn approve(&self, first_byte: u8) {
            let data = [first_byte, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            // SAFETY: writes a 12-byte binary value under the test key.
            let status = unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    &HSTRING::from(format!(r"{}\Approved", self.root)),
                    &HSTRING::from("ClipVault"),
                    REG_BINARY.0,
                    Some(data.as_ptr().cast()),
                    12,
                )
            };
            assert!(status.is_ok());
        }
    }

    impl Drop for TestKeys {
        fn drop(&mut self) {
            // SAFETY: removes the test tree created by this value.
            let _ = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(&self.root)) };
        }
    }

    #[test]
    fn cfg_003_run_key_enable_and_disable_round_trip() {
        let keys = TestKeys::new("round-trip");
        let startup = keys.startup();

        assert_eq!(startup.state().unwrap(), StartupState::Disabled);
        assert_eq!(startup.enable().unwrap(), StartupState::Enabled);
        assert_eq!(startup.state().unwrap(), StartupState::Enabled);
        assert_eq!(startup.disable().unwrap(), StartupState::Disabled);
        assert_eq!(
            startup.disable().unwrap(),
            StartupState::Disabled,
            "idempotent"
        );
    }

    #[test]
    fn cfg_003_run_key_reports_and_keeps_a_switch_turned_off_by_the_user() {
        let keys = TestKeys::new("by-user");
        let startup = keys.startup();
        startup.enable().unwrap();

        keys.approve(0x03);
        assert_eq!(startup.state().unwrap(), StartupState::DisabledByUser);
        assert_eq!(startup.enable().unwrap(), StartupState::DisabledByUser);

        keys.approve(0x02);
        assert_eq!(startup.state().unwrap(), StartupState::Enabled);
    }

    #[test]
    fn cfg_003_task_states_map_one_to_one() {
        assert_eq!(
            from_task_state(StartupTaskState::Enabled),
            StartupState::Enabled
        );
        assert_eq!(
            from_task_state(StartupTaskState::DisabledByUser),
            StartupState::DisabledByUser
        );
        assert_eq!(
            from_task_state(StartupTaskState::DisabledByPolicy),
            StartupState::DisabledByPolicy
        );
        assert_eq!(
            from_task_state(StartupTaskState::EnabledByPolicy),
            StartupState::EnabledByPolicy
        );
        assert_eq!(
            from_task_state(StartupTaskState::Disabled),
            StartupState::Disabled
        );
    }

    #[test]
    fn tests_run_without_package_identity() {
        assert!(!has_package_identity());
    }
}
