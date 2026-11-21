//! WebView2 Evergreen Runtime detection and the native error box shown without it (CMP-003).

use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW,
};
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW};
use windows::core::{HSTRING, PCWSTR, w};

/// The runtime's `EdgeUpdate` client, per machine (64-bit Windows registers it under
/// `WOW6432Node`) and per user.
const LOCATIONS: [(HKEY, PCWSTR); 3] = [
    (
        HKEY_LOCAL_MACHINE,
        w!(
            r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
        ),
    ),
    (
        HKEY_LOCAL_MACHINE,
        w!(r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"),
    ),
    (
        HKEY_CURRENT_USER,
        w!(r"Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"),
    ),
];

/// A registered runtime version; empty or `0.0.0.0` means "not installed".
pub fn parse_version(raw: &str) -> Option<String> {
    let version = raw.trim_end_matches('\0').trim();
    (!version.is_empty() && version != "0.0.0.0").then(|| version.to_owned())
}

fn read_pv(root: HKEY, key: PCWSTR) -> Option<String> {
    let mut buffer = [0_u16; 64];
    let mut size = u32::try_from(size_of_val(&buffer)).ok()?;
    // SAFETY: `buffer` and `size` describe a writable wide-string buffer.
    let status = unsafe {
        RegGetValueW(
            root,
            key,
            w!("pv"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&raw mut size),
        )
    };
    if status.is_err() {
        return None;
    }
    let chars = (size as usize / 2).min(buffer.len());
    parse_version(&String::from_utf16_lossy(buffer.get(..chars)?))
}

/// The installed WebView2 runtime version, if any.
pub fn webview2_version() -> Option<String> {
    LOCATIONS
        .iter()
        .find_map(|(root, key)| read_pv(*root, *key))
}

/// Shows a blocking error box with no owner window.
pub fn show_error(title: &str, message: &str) {
    // SAFETY: both strings are NUL-terminated `HSTRING`s that outlive the call.
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(message),
            &HSTRING::from(title),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmp_003_a_registered_version_is_reported() {
        assert_eq!(
            parse_version("131.0.2903.70\0"),
            Some("131.0.2903.70".to_owned())
        );
    }

    #[test]
    fn cmp_003_empty_or_zero_versions_mean_not_installed() {
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("\0"), None);
        assert_eq!(parse_version("  "), None);
        assert_eq!(parse_version("0.0.0.0"), None);
    }

    #[test]
    fn cmp_003_the_development_machine_has_the_runtime() {
        // Windows 11 and updated Windows 10 ship the Evergreen runtime.
        assert!(webview2_version().is_some());
    }
}
