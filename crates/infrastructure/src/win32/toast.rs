//! Native toast notifications (PST-005, PST-006, HK-004 — ADR-0010).

use std::sync::OnceLock;

use clipvault_application::ports::PortError;
use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
use windows::Win32::Storage::Packaging::Appx::GetCurrentApplicationUserModelId;
use windows::Win32::System::Com::CoIncrementMTAUsage;
use windows::core::{HSTRING, PWSTR};

/// Registered AUMID of Windows PowerShell: the fallback without package identity.
pub const POWERSHELL_APP_ID: &str =
    r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe";

/// The AUMID a toast must carry: the package's own, or the PowerShell fallback.
pub fn app_id(package_app_id: Option<String>) -> String {
    package_app_id.unwrap_or_else(|| POWERSHELL_APP_ID.to_owned())
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

/// Toast content with `title` and `body` escaped as XML text.
pub fn toast_xml(title: &str, body: &str) -> String {
    format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        escape(title),
        escape(body)
    )
}

/// The process's AUMID (`<PFN>!<AppId>`) when it runs with package identity.
fn package_app_id() -> Option<String> {
    let mut length = 0_u32;
    // SAFETY: a zero length with no buffer only asks for the required size.
    let status = unsafe { GetCurrentApplicationUserModelId(&raw mut length, None) };
    if status != ERROR_INSUFFICIENT_BUFFER {
        return None; // APPMODEL_ERROR_NO_APPLICATION / _NO_PACKAGE: not packaged
    }
    let mut buffer = vec![0_u16; length as usize];
    // SAFETY: `buffer` holds `length` writable UTF-16 units, as the first call requested.
    let status = unsafe {
        GetCurrentApplicationUserModelId(&raw mut length, Some(PWSTR(buffer.as_mut_ptr())))
    };
    if status.is_err() {
        return None;
    }
    let chars = (length as usize).saturating_sub(1).min(buffer.len()); // drop the NUL
    Some(String::from_utf16_lossy(buffer.get(..chars)?))
}

/// `WinRT` activation needs COM on the calling thread; the hotkey and command threads have
/// none, so the process keeps the implicit MTA alive once (the cookie is never released).
fn ensure_mta() {
    static MTA: OnceLock<()> = OnceLock::new();
    MTA.get_or_init(|| {
        // SAFETY: no preconditions; failure only means toasts may fail and get logged.
        if let Err(error) = unsafe { CoIncrementMTAUsage() } {
            tracing::warn!(%error, "COM MTA usage could not be incremented");
        }
    });
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(format!("toast notification: {error}"))
}

/// Shows a toast; Windows drops it silently if the AUMID is not a registered app.
///
/// # Errors
/// `PortError::Unavailable` when the toast XML or the `WinRT` notifier cannot be created.
pub fn show_toast(title: &str, body: &str) -> Result<(), PortError> {
    ensure_mta();
    let document = XmlDocument::new().map_err(unavailable)?;
    document
        .LoadXml(&HSTRING::from(toast_xml(title, body)))
        .map_err(unavailable)?;
    let toast = ToastNotification::CreateToastNotification(&document).map_err(unavailable)?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id(
        package_app_id(),
    )))
    .map_err(unavailable)?;
    notifier.Show(&toast).map_err(unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pst_005_packaged_process_uses_its_package_app_id() {
        let package = "flvrm92.Clip-Vault_2nrc79jzqdzh4!ClipVault".to_owned();
        assert_eq!(app_id(Some(package.clone())), package);
    }

    #[test]
    fn pst_005_unpackaged_process_uses_the_powershell_app_id() {
        assert_eq!(
            app_id(None),
            r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe"
        );
    }

    #[test]
    fn pst_005_tests_run_without_package_identity() {
        assert_eq!(app_id(package_app_id()), POWERSHELL_APP_ID);
    }

    #[test]
    fn pst_005_toast_xml_escapes_translated_text() {
        let xml = toast_xml("ClipVault", r#"Tom & Jerry <"copied"> 'ok'"#);
        assert_eq!(
            xml,
            "<toast><visual><binding template=\"ToastGeneric\">\
             <text>ClipVault</text>\
             <text>Tom &amp; Jerry &lt;&quot;copied&quot;&gt; &apos;ok&apos;</text>\
             </binding></visual></toast>"
        );
    }
}
