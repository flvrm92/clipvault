//! Contract of [`SnippetRepository`] (SNP-001..002).

use clipvault_application::ports::{PortError, SnippetRepository};
use clipvault_domain::hotkey::Hotkey;
use clipvault_domain::ids::SnippetId;
use clipvault_domain::snippet::{NewSnippet, SnippetBody, SnippetName};
use clipvault_domain::time::Timestamp;

/// A new snippet.
pub fn new_snippet(name: &str, hotkey: Option<&str>) -> NewSnippet {
    NewSnippet {
        name: SnippetName::parse(name).unwrap(),
        body: SnippetBody::new(&format!("corpo de {name}")).unwrap(),
        hotkey: hotkey.map(|h| Hotkey::parse(h).unwrap()),
        enabled: true,
    }
}

fn at(ms: i64) -> Timestamp {
    Timestamp::from_millis(ms)
}

/// Inserted snippets get an id and are read back unchanged.
pub fn insert_then_get(repo: &dyn SnippetRepository) {
    let saved = repo
        .insert(new_snippet("Assinatura", Some("Ctrl+Shift+1")), at(1))
        .unwrap();

    let read = repo.get(saved.id).unwrap().unwrap();

    assert_eq!(read, saved);
    assert_eq!(read.name.as_str(), "Assinatura");
    assert_eq!(read.body.as_str(), "corpo de Assinatura");
    assert_eq!(read.hotkey, Some(Hotkey::parse("Ctrl+Shift+1").unwrap()));
    assert!(read.enabled);
    assert_eq!(repo.get(SnippetId::new(9_999)).unwrap(), None);
}

/// SNP-002: snippets are listed in insertion order.
pub fn snp_002_list_keeps_insertion_order(repo: &dyn SnippetRepository) {
    for name in ["C", "A", "B"] {
        repo.insert(new_snippet(name, None), at(1)).unwrap();
    }

    let names: Vec<_> = repo
        .list()
        .unwrap()
        .into_iter()
        .map(|s| s.name.as_str().to_owned())
        .collect();

    assert_eq!(names, ["C", "A", "B"]);
}

/// Updates replace every field.
pub fn update_replaces_fields(repo: &dyn SnippetRepository) {
    let mut snippet = repo
        .insert(new_snippet("Antigo", Some("Ctrl+Shift+1")), at(1))
        .unwrap();
    snippet.name = SnippetName::parse("Novo").unwrap();
    snippet.body = SnippetBody::new("texto novo").unwrap();
    snippet.hotkey = None;
    snippet.enabled = false;

    repo.update(&snippet, at(2)).unwrap();

    assert_eq!(repo.get(snippet.id).unwrap().unwrap(), snippet);
}

/// Updating or deleting an unknown snippet is `NotFound`.
pub fn unknown_snippets_are_not_found(repo: &dyn SnippetRepository) {
    let mut ghost = repo.insert(new_snippet("Ghost", None), at(1)).unwrap();
    repo.delete(ghost.id).unwrap();

    assert_eq!(repo.delete(ghost.id), Err(PortError::NotFound));
    ghost.enabled = false;
    assert_eq!(repo.update(&ghost, at(2)), Err(PortError::NotFound));
    assert_eq!(repo.list().unwrap(), vec![]);
}

/// SNP-001: names (ignoring case) and shortcuts are unique.
pub fn snp_001_names_and_shortcuts_are_unique(repo: &dyn SnippetRepository) {
    repo.insert(new_snippet("Assinatura", Some("Ctrl+Shift+1")), at(1))
        .unwrap();

    assert!(matches!(
        repo.insert(new_snippet("ASSINATURA", None), at(2)),
        Err(PortError::InUse(_))
    ));
    assert!(matches!(
        repo.insert(new_snippet("Outro", Some("Ctrl+Shift+1")), at(2)),
        Err(PortError::InUse(_))
    ));
}

/// Generates one `#[test]` per [`SnippetRepository`] contract case.
#[macro_export]
macro_rules! snippet_repository_contract {
    ($make:expr) => {
        $crate::__contract_tests!(
            snippets,
            $make,
            insert_then_get,
            snp_002_list_keeps_insertion_order,
            update_replaces_fields,
            unknown_snippets_are_not_found,
            snp_001_names_and_shortcuts_are_unique,
        );
    };
}
