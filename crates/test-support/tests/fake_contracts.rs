//! The fakes must honour the same contracts as the real adapters (specs/04 §P-11).

mod history {
    clipvault_test_support::history_repository_contract!(
        clipvault_test_support::InMemoryHistoryRepository::default()
    );
}

mod images {
    clipvault_test_support::image_store_contract!(
        clipvault_test_support::InMemoryImageStore::default()
    );
}

mod settings {
    clipvault_test_support::settings_repository_contract!(
        clipvault_test_support::FixedSettings::default()
    );
}

mod snippets {
    clipvault_test_support::snippet_repository_contract!(
        clipvault_test_support::InMemorySnippetRepository::default()
    );
}
