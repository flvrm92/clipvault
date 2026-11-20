use super::*;
use crate::hotkey::Hotkey;

#[test]
fn snp_001_name_is_trimmed_and_required() {
    assert_eq!(
        SnippetName::parse("  Assinatura ").unwrap().as_str(),
        "Assinatura"
    );
    assert_eq!(SnippetName::parse("   "), Err(SnippetError::NameBlank));
}

#[test]
fn snp_001_name_has_at_most_60_characters() {
    assert!(SnippetName::parse(&"é".repeat(60)).is_ok());
    assert_eq!(
        SnippetName::parse(&"é".repeat(61)),
        Err(SnippetError::NameTooLong {
            max: SNIPPET_NAME_MAX_CHARS
        })
    );
}

#[test]
fn snp_001_names_are_compared_case_insensitively_for_uniqueness() {
    let a = SnippetName::parse("Assinatura").unwrap();
    let b = SnippetName::parse("ASSINATURA").unwrap();

    assert!(a.is_same_as(&b));
    assert!(!a.is_same_as(&SnippetName::parse("Endereço").unwrap()));
}

#[test]
fn snp_001_body_is_required_and_kept_verbatim() {
    assert_eq!(SnippetBody::new(" \n "), Err(SnippetError::BodyBlank));
    assert_eq!(
        SnippetBody::new("Atenciosamente,\n").unwrap().as_str(),
        "Atenciosamente,\n"
    );
}

#[test]
fn snp_001_body_is_limited_to_100_kib() {
    assert!(SnippetBody::new(&"a".repeat(SNIPPET_BODY_MAX_BYTES)).is_ok());
    assert_eq!(
        SnippetBody::new(&"a".repeat(SNIPPET_BODY_MAX_BYTES + 1)),
        Err(SnippetError::BodyTooLarge {
            max: SNIPPET_BODY_MAX_BYTES
        })
    );
}

#[test]
fn snp_001_a_draft_holds_name_body_optional_hotkey_and_enabled_flag() {
    let draft = NewSnippet {
        name: SnippetName::parse("Teste").unwrap(),
        body: SnippetBody::new("Teste de texto").unwrap(),
        hotkey: Some(Hotkey::parse("Ctrl+Shift+V").unwrap()),
        enabled: true,
    };

    assert_eq!(
        draft.hotkey.map(|h| h.to_string()).as_deref(),
        Some("Ctrl+Shift+V")
    );
}

#[test]
fn priv_003_snippet_body_debug_is_redacted() {
    let body = SnippetBody::new("Atenciosamente, Fulano").unwrap();

    assert!(!format!("{body:?}").contains("Fulano"));
}
