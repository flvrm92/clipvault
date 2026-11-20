use super::*;

#[test]
fn cap_005_empty_or_whitespace_text_is_rejected() {
    for blank in ["", " ", "\n\t  \r\n"] {
        assert_eq!(ClipText::new(blank), Err(TextError::Blank), "{blank:?}");
    }
}

#[test]
fn text_is_kept_verbatim() {
    let text = ClipText::new("  olá\n mundo  ").unwrap();

    assert_eq!(text.as_str(), "  olá\n mundo  ");
    assert_eq!(text.byte_len(), "  olá\n mundo  ".len());
}

#[test]
fn hist_001_preview_collapses_whitespace_and_skips_blank_lines() {
    let text = ClipText::new("  git   rebase\t-i \n\n\n   HEAD~3  ").unwrap();

    assert_eq!(text.preview(), "git rebase -i\nHEAD~3");
}

#[test]
fn hist_001_preview_keeps_at_most_three_lines_with_ellipsis() {
    let text = ClipText::new("one\ntwo\nthree\nfour").unwrap();

    assert_eq!(text.preview(), "one\ntwo\nthree…");
}

#[test]
fn hist_001_preview_is_truncated_to_the_char_limit_on_char_boundaries() {
    let long = "é".repeat(PREVIEW_MAX_CHARS + 10);
    let text = ClipText::new(&long).unwrap();
    let preview = text.preview();

    assert_eq!(preview.chars().count(), PREVIEW_MAX_CHARS + 1);
    assert!(preview.ends_with('…'));
}

#[test]
fn hist_003_search_form_is_lowercase_without_diacritics() {
    let text = ClipText::new("Café com LEITE, Ação e Pão").unwrap();

    assert_eq!(text.search_form(), "cafe com leite, acao e pao");
    assert_eq!(normalize_for_search("CAFÉ"), "cafe");
}

#[test]
fn priv_003_debug_output_never_contains_the_text() {
    let text = ClipText::new("senha super secreta").unwrap();

    let debug = format!("{text:?}");

    assert!(!debug.contains("secreta"), "{debug}");
    assert!(debug.contains("19 bytes"), "{debug}");
}
