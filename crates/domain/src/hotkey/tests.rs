use proptest::prelude::*;

use super::*;
use crate::ids::SnippetId;

fn hk(s: &str) -> Hotkey {
    Hotkey::parse(s).unwrap()
}

#[test]
fn hk_003_parses_modifiers_and_key() {
    let hotkey = hk("Ctrl+Shift+1");

    assert_eq!(hotkey.modifiers(), Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(hotkey.key().to_string(), "1");
}

#[test]
fn hk_003_parsing_is_case_insensitive_order_insensitive_and_tolerates_spaces() {
    assert_eq!(hk("shift + ctrl + 1"), hk("Ctrl+Shift+1"));
    assert_eq!("shift+ctrl+1".parse::<Hotkey>(), Ok(hk("Ctrl+Shift+1")));
    assert_eq!(hk("CONTROL+windows+v"), hk("Ctrl+Win+V"));
    assert_eq!(hk("ctrl+shift+del"), hk("Ctrl+Shift+Delete"));
}

#[test]
fn hk_003_display_is_canonical() {
    assert_eq!(hk("win+v+ctrl").to_string(), "Ctrl+Win+V");
    assert_eq!(
        hk("shift+alt+ctrl+win+pgup").to_string(),
        "Ctrl+Alt+Shift+Win+PageUp"
    );
    assert_eq!(hk("ctrl+num5").to_string(), "Ctrl+Num5");
}

#[test]
fn hk_003_requires_a_modifier_except_for_f13_to_f24() {
    assert_eq!(Hotkey::parse("V"), Err(HotkeyError::MissingModifier));
    assert_eq!(Hotkey::parse("F12"), Err(HotkeyError::MissingModifier));
    assert!(Hotkey::parse("F13").is_ok());
    assert!(Hotkey::parse("F24").is_ok());
}

#[test]
fn hk_003_requires_exactly_one_non_modifier_key() {
    assert_eq!(Hotkey::parse("Ctrl+Shift"), Err(HotkeyError::MissingKey));
    assert_eq!(Hotkey::parse("Ctrl+A+B"), Err(HotkeyError::MultipleKeys));
}

#[test]
fn hk_003_rejects_empty_unknown_and_duplicate_tokens() {
    assert_eq!(Hotkey::parse(""), Err(HotkeyError::Empty));
    assert_eq!(Hotkey::parse("  "), Err(HotkeyError::Empty));
    assert_eq!(
        Hotkey::parse("Ctrl+Banana"),
        Err(HotkeyError::UnknownToken("Banana".into()))
    );
    assert_eq!(
        Hotkey::parse("Ctrl++V"),
        Err(HotkeyError::UnknownToken(String::new()))
    );
    assert_eq!(
        Hotkey::parse("Ctrl+Control+V"),
        Err(HotkeyError::DuplicateModifier)
    );
}

#[test]
fn hk_003_rejects_reserved_combinations() {
    for reserved in [
        "Ctrl+C",
        "Ctrl+V",
        "Ctrl+X",
        "Ctrl+Z",
        "Win+L",
        "Win+V",
        "Ctrl+Alt+Delete",
        "Alt+F4",
        "Alt+Tab",
    ] {
        assert_eq!(
            Hotkey::parse(reserved),
            Err(HotkeyError::Reserved),
            "{reserved}"
        );
    }
    assert!(
        Hotkey::parse("Ctrl+Shift+V").is_ok(),
        "only exact reserved combinations are rejected"
    );
}

#[test]
fn hk_001_default_open_hotkey_is_ctrl_win_v_and_valid() {
    let default = Hotkey::default_open();

    assert_eq!(default.to_string(), "Ctrl+Win+V");
    assert_eq!(Hotkey::parse(&default.to_string()), Ok(default));
}

#[test]
fn hk_003_detects_conflicts_with_other_clipvault_bindings() {
    let open = Hotkey::default_open();
    let snippet = hk("Ctrl+Shift+1");
    let bindings = [
        (HotkeyOwner::OpenHistory, &open),
        (HotkeyOwner::Snippet(SnippetId::new(1)), &snippet),
    ];

    assert_eq!(
        find_conflict(&hk("shift+ctrl+1"), bindings, HotkeyOwner::OpenHistory),
        Some(HotkeyOwner::Snippet(SnippetId::new(1)))
    );
    assert_eq!(
        find_conflict(&hk("Ctrl+Shift+2"), bindings, HotkeyOwner::OpenHistory),
        None
    );
}

#[test]
fn hk_003_a_binding_does_not_conflict_with_itself() {
    let snippet = hk("Ctrl+Shift+1");
    let owner = HotkeyOwner::Snippet(SnippetId::new(1));

    assert_eq!(find_conflict(&snippet, [(owner, &snippet)], owner), None);
}

#[test]
fn every_listed_key_parses_from_its_display_name() {
    for key in Key::all() {
        assert_eq!(key.to_string().parse::<Key>(), Ok(key));
    }
    assert!(Key::all().count() > 80);
}

fn hotkey_strategy() -> impl Strategy<Value = Hotkey> {
    let keys: Vec<Key> = Key::all().collect();
    (1_u8..16, prop::sample::select(keys)).prop_filter_map("reserved combination", |(bits, key)| {
        Hotkey::new(Modifiers::from_bits(bits), key).ok()
    })
}

proptest! {
    #[test]
    fn hk_003_display_then_parse_round_trips(hotkey in hotkey_strategy()) {
        prop_assert_eq!(Hotkey::parse(&hotkey.to_string()), Ok(hotkey));
    }

    #[test]
    fn hk_003_parse_is_insensitive_to_case(hotkey in hotkey_strategy()) {
        let text = hotkey.to_string();
        prop_assert_eq!(Hotkey::parse(&text.to_lowercase()), Ok(hotkey));
        prop_assert_eq!(Hotkey::parse(&text.to_uppercase()), Ok(hotkey));
    }
}
