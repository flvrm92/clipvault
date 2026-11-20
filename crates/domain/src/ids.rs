//! Entity identifiers (database row ids).

macro_rules! entity_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(i64);

        impl $name {
            /// Wraps a raw id.
            pub const fn new(raw: i64) -> Self {
                Self(raw)
            }

            /// The raw id.
            pub const fn get(self) -> i64 {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

entity_id!(
    /// Identifier of a history item.
    ClipId
);
entity_id!(
    /// Identifier of a snippet.
    SnippetId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_wrap_and_display_the_raw_value() {
        assert_eq!(ClipId::new(7).get(), 7);
        assert_eq!(SnippetId::new(9).to_string(), "9");
    }
}
