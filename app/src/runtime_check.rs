//! CMP-003: the app needs the WebView2 runtime; without it, a native translated message explains
//! how to install it instead of failing silently.

use crate::i18n::{Locale, Translator};

/// Title and body of the missing-runtime box, in `locale`.
pub fn missing_runtime_message(locale: Locale) -> (String, String) {
    match Translator::new(locale) {
        Ok(translator) => (
            translator.t("webview2.title"),
            translator.t("webview2.missing"),
        ),
        // The catalogs are embedded; this only guards against a broken build.
        Err(_) => (String::from("ClipVault"), String::from("WebView2 Runtime")),
    }
}

/// Whether the WebView2 runtime is installed; shows the explanation when it is not.
pub fn webview2_available() -> bool {
    if clipvault_infrastructure::win32::webview2_version().is_some() {
        return true;
    }
    tracing::error!("WebView2 runtime not found");
    let (title, body) = missing_runtime_message(Locale::system());
    clipvault_infrastructure::win32::show_error(&title, &body);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOWNLOAD: &str = "https://developer.microsoft.com/microsoft-edge/webview2/";

    #[test]
    fn cmp_003_the_missing_runtime_message_is_translated_and_says_where_to_download() {
        let (pt_title, pt_body) = missing_runtime_message(Locale::PtBr);
        let (en_title, en_body) = missing_runtime_message(Locale::En);

        assert_ne!(pt_body, en_body);
        for (title, body) in [(&pt_title, &pt_body), (&en_title, &en_body)] {
            assert!(!title.starts_with("webview2."), "{title}");
            assert!(body.contains(DOWNLOAD), "{body}");
        }
    }

    #[test]
    fn cmp_003_the_development_machine_starts_normally() {
        assert!(webview2_available());
    }
}
