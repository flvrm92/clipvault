use super::*;

#[test]
fn app_names_are_normalized_to_lowercase_file_names() {
    assert_eq!(
        AppName::parse("KeePass.EXE").unwrap().as_str(),
        "keepass.exe"
    );
    assert_eq!(
        AppName::parse(r"C:\Program Files\KeePass\KeePass.exe")
            .unwrap()
            .as_str(),
        "keepass.exe"
    );
    assert_eq!(AppName::parse("  code.exe ").unwrap().as_str(), "code.exe");
}

#[test]
fn blank_app_names_are_rejected() {
    for blank in ["", "   ", r"C:\folder\"] {
        assert_eq!(AppName::parse(blank), Err(AppNameError::Blank), "{blank:?}");
    }
}

#[test]
fn cap_009_comparison_is_case_insensitive_through_normalization() {
    assert_eq!(AppName::parse("KEEPASS.exe"), AppName::parse("keepass.EXE"));
}
