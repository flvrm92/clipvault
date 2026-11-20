//! Contract tests shared by fakes and real adapters (specs/04 §P-11, specs/08 §2).
//!
//! Each case is a function taking the port as `&dyn Trait`; the `*_contract!` macros expand
//! to one `#[test]` per case, each with a fresh instance:
//!
//! ```ignore
//! clipvault_test_support::history_repository_contract!(SqliteHistoryRepository::in_memory());
//! ```

pub mod history;
pub mod images;
pub mod settings;
pub mod snippets;

/// Expands to a `#[test]` per case of a contract module.
#[doc(hidden)]
#[macro_export]
macro_rules! __contract_tests {
    ($module:ident, $make:expr, $($case:ident),+ $(,)?) => {
        $(
            #[test]
            fn $case() {
                let subject = $make;
                $crate::contracts::$module::$case(&subject);
            }
        )+
    };
}
