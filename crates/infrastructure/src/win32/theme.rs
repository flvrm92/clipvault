//! System appearance from the registry (THM-001..003).

use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};
use windows::core::{PCWSTR, w};

const PERSONALIZE: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
const CURRENT_VERSION: PCWSTR = w!(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
/// First Windows 11 build.
const WINDOWS_11_BUILD: u32 = 22_000;

/// How Windows is configured to look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // one flag per Windows setting
pub struct SystemAppearance {
    /// Apps use the light theme (`AppsUseLightTheme`).
    pub apps_light: bool,
    /// The taskbar uses the light theme (`SystemUsesLightTheme`) — drives the tray icon.
    pub taskbar_light: bool,
    /// Transparency effects are enabled (`EnableTransparency`).
    pub transparency: bool,
    /// Windows 11 or later (Mica/Acrylic backdrops available).
    pub windows_11: bool,
}

fn dword(root: HKEY, key: PCWSTR, value: PCWSTR) -> Option<u32> {
    let mut data = 0_u32;
    let mut size = u32::try_from(size_of::<u32>()).ok()?;
    // SAFETY: `data` and `size` describe a writable DWORD buffer.
    let status = unsafe {
        RegGetValueW(
            root,
            key,
            value,
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut data).cast()),
            Some(&raw mut size),
        )
    };
    status.is_ok().then_some(data)
}

fn build_number() -> Option<u32> {
    let mut buffer = [0_u16; 32];
    let mut size = u32::try_from(size_of_val(&buffer)).ok()?;
    // SAFETY: `buffer` and `size` describe a writable wide-string buffer.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            CURRENT_VERSION,
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&raw mut size),
        )
    };
    if status.is_err() {
        return None;
    }
    let text = String::from_utf16_lossy(&buffer);
    text.trim_end_matches('\0').trim().parse().ok()
}

/// Reads the current appearance; missing values use the Windows defaults (light, transparent).
pub fn system_appearance() -> SystemAppearance {
    let flag = |value, default: bool| {
        dword(HKEY_CURRENT_USER, PERSONALIZE, value).map_or(default, |v| v != 0)
    };
    SystemAppearance {
        apps_light: flag(w!("AppsUseLightTheme"), true),
        taskbar_light: flag(w!("SystemUsesLightTheme"), false),
        transparency: flag(w!("EnableTransparency"), true),
        windows_11: build_number().is_some_and(|b| b >= WINDOWS_11_BUILD),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_running_system_reports_a_build_number() {
        // Any supported Windows (10 22H2 is build 19045) exposes its build.
        assert!(build_number().is_some_and(|b| b >= 19_041));
        let _ = system_appearance();
    }
}
