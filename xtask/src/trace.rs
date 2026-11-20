//! Requirement ↔ test traceability (specs/08 §4).
//!
//! Requirements are read from the specs; references are discovered in test code:
//! - Rust: test function names prefixed with the lowercase ID, e.g. `fn cap_004_promotes_duplicate()`.
//! - TypeScript: `[CAP-004]` inside `*.test.ts` / `*.spec.ts` files.
//!
//! NOTE: fixtures in this module deliberately use fictitious prefixes (`FOO`, `BAR`, `X11Y`)
//! so the scanner never mistakes them for real coverage.

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;
use std::sync::LazyLock;

use anyhow::Context;
use regex::Regex;
use serde::Deserialize;

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| unreachable!("invalid built-in regex {pattern}: {e}"))
}

static FUNCTIONAL_HEADING: LazyLock<Regex> =
    LazyLock::new(|| regex(r"^###\s+([A-Z][A-Z0-9]*)-(\d{3})\s+·"));
static ANY_HEADING: LazyLock<Regex> = LazyLock::new(|| regex(r"^#{1,3}\s"));
static TABLE_ROW: LazyLock<Regex> = LazyLock::new(|| regex(r"^\|\s*([A-Z][A-Z0-9]*)-(\d{3})\s*\|"));
static RUST_TEST_FN: LazyLock<Regex> =
    LazyLock::new(|| regex(r"\bfn\s+([a-z][a-z0-9]*)_(\d{3})(?:_|\b)"));
static TS_REFERENCE: LazyLock<Regex> = LazyLock::new(|| regex(r"\[([A-Z][A-Z0-9]*)-(\d{3})\]"));
const MANUAL_MARKER: &str = "[manual]";

fn ids_from(re: &Regex, source: &str) -> BTreeSet<ReqId> {
    re.captures_iter(source)
        .map(|c| ReqId::new(&c[1], &c[2]))
        .collect()
}

/// A requirement identifier such as `CAP-004`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReqId(String);

impl ReqId {
    /// Builds an ID from a prefix (any case) and a 3-digit number.
    pub fn new(prefix: &str, number: &str) -> Self {
        Self(format!("{}-{number}", prefix.to_uppercase()))
    }

    /// The area prefix, e.g. `CAP`.
    pub fn prefix(&self) -> &str {
        self.0.split('-').next().unwrap_or_default()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ReqId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A requirement declared in the specs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub id: ReqId,
    /// Verified manually (`[manual]` marker) — not expected to have automated tests.
    pub manual: bool,
}

/// Parses functional requirements: `### CAP-001 · Title — M` headings.
pub fn parse_functional(markdown: &str) -> Vec<Requirement> {
    let mut requirements: Vec<Requirement> = Vec::new();
    let mut in_requirement = false;
    for line in markdown.lines() {
        if let Some(c) = FUNCTIONAL_HEADING.captures(line) {
            requirements.push(Requirement {
                id: ReqId::new(&c[1], &c[2]),
                manual: false,
            });
            in_requirement = true;
        } else if ANY_HEADING.is_match(line) {
            in_requirement = false;
        } else if in_requirement
            && line.contains(MANUAL_MARKER)
            && let Some(current) = requirements.last_mut()
        {
            current.manual = true;
        }
    }
    requirements
}

/// Parses non-functional requirements: table rows starting with `| PERF-001 |`.
pub fn parse_nonfunctional(markdown: &str) -> Vec<Requirement> {
    markdown
        .lines()
        .filter_map(|line| {
            TABLE_ROW.captures(line).map(|c| Requirement {
                id: ReqId::new(&c[1], &c[2]),
                manual: line.contains(MANUAL_MARKER),
            })
        })
        .collect()
}

/// Requirement IDs referenced by Rust test function names.
pub fn references_in_rust(source: &str) -> BTreeSet<ReqId> {
    source
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _comment)| code))
        .flat_map(|code| ids_from(&RUST_TEST_FN, code))
        .collect()
}

/// Requirement IDs referenced as `[ID]` in TypeScript test files.
pub fn references_in_ts(source: &str) -> BTreeSet<ReqId> {
    ids_from(&TS_REFERENCE, source)
}

/// Which requirements must be covered by tests (grows with each milestone).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Scope {
    /// Exact IDs (`CAP-004`) or whole areas (`CAP-*`).
    pub enforced: Vec<String>,
}

impl Scope {
    fn matches(&self, id: &ReqId) -> bool {
        self.enforced.iter().any(|entry| entry_matches(entry, id))
    }
}

fn entry_matches(entry: &str, id: &ReqId) -> bool {
    match entry.strip_suffix("-*") {
        Some(prefix) => prefix == id.prefix(),
        None => entry == id.as_str(),
    }
}

/// Result of a traceability evaluation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Enforced, automatable requirements without any test.
    pub missing: Vec<ReqId>,
    /// Test references to IDs that do not exist in the specs (typos, removed requirements).
    pub unknown: Vec<ReqId>,
    /// Scope entries that match no requirement.
    pub dangling_scope: Vec<String>,
    /// Requirements with at least one test.
    pub covered: Vec<ReqId>,
}

impl Report {
    pub fn is_ok(&self) -> bool {
        self.missing.is_empty() && self.unknown.is_empty() && self.dangling_scope.is_empty()
    }
}

/// Compares requirements, test references and the enforced scope.
pub fn evaluate(
    requirements: &[Requirement],
    references: &BTreeSet<ReqId>,
    scope: &Scope,
) -> Report {
    let known: BTreeSet<&ReqId> = requirements.iter().map(|r| &r.id).collect();
    let known_prefixes: BTreeSet<&str> = known.iter().map(|id| id.prefix()).collect();

    let covered: BTreeSet<ReqId> = known
        .iter()
        .filter(|id| references.contains(id))
        .map(|id| (*id).clone())
        .collect();
    let missing: BTreeSet<ReqId> = requirements
        .iter()
        .filter(|r| !r.manual && scope.matches(&r.id) && !covered.contains(&r.id))
        .map(|r| r.id.clone())
        .collect();
    let unknown = references
        .iter()
        .filter(|id| known_prefixes.contains(id.prefix()) && !known.contains(id))
        .cloned()
        .collect();
    let dangling_scope = scope
        .enforced
        .iter()
        .filter(|entry| !known.iter().any(|id| entry_matches(entry, id)))
        .cloned()
        .collect();

    Report {
        missing: missing.into_iter().collect(),
        unknown,
        dangling_scope,
        covered: covered.into_iter().collect(),
    }
}

/// Scans the repository at `root` and evaluates traceability.
///
/// # Errors
/// Fails when the spec files or the scope file cannot be read or parsed.
pub fn run(root: &Path) -> anyhow::Result<Report> {
    let read = |rel: &str| {
        std::fs::read_to_string(root.join(rel)).with_context(|| format!("reading {rel}"))
    };
    let mut requirements = parse_functional(&read("specs/01-requisitos-funcionais.md")?);
    requirements.extend(parse_nonfunctional(&read(
        "specs/02-requisitos-nao-funcionais.md",
    )?));
    let scope: Scope =
        toml::from_str(&read("specs/trace-scope.toml")?).context("parsing trace-scope.toml")?;

    let mut references = BTreeSet::new();
    let skipped = ["target", "node_modules", ".git", "dist", "specs", "docs"];
    let walker = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !skipped.iter().any(|s| e.file_name() == *s));
    for entry in walker {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy();
        let is_ts_test = [".test.ts", ".spec.ts"].iter().any(|s| name.ends_with(s));
        let is_rust = name.ends_with(".rs");
        if !(is_ts_test || is_rust) {
            continue;
        }
        let source = std::fs::read_to_string(entry.path())?;
        if is_rust {
            references.extend(references_in_rust(&source));
        } else {
            references.extend(references_in_ts(&source));
        }
    }
    Ok(evaluate(&requirements, &references, &scope))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> ReqId {
        let (p, n) = s.split_once('-').unwrap();
        ReqId::new(p, n)
    }

    fn ids(list: &[&str]) -> BTreeSet<ReqId> {
        list.iter().map(|s| id(s)).collect()
    }

    fn req(s: &str) -> Requirement {
        Requirement {
            id: id(s),
            manual: false,
        }
    }

    #[test]
    fn parses_functional_headings_and_manual_marker() {
        let md = "\
## FOO — Area

### FOO-001 · First — M
Some text.

### FOO-002 · Second — S
Behaviour that is checked by hand. `[manual]`

### Not a requirement
### X11Y-003 · Digits in prefix — C
";
        assert_eq!(
            parse_functional(md),
            vec![
                Requirement {
                    id: id("FOO-001"),
                    manual: false
                },
                Requirement {
                    id: id("FOO-002"),
                    manual: true
                },
                Requirement {
                    id: id("X11Y-003"),
                    manual: false
                },
            ]
        );
    }

    #[test]
    fn parses_nonfunctional_table_rows() {
        let md = "\
| ID | Requisito | Meta |
|----|-----------|------|
| BAR-001 | Fast | 50 ms |
| BAR-002 | Smooth | `[manual]` |
Text mentioning BAR-009 outside a table.
";
        assert_eq!(
            parse_nonfunctional(md),
            vec![
                Requirement {
                    id: id("BAR-001"),
                    manual: false
                },
                Requirement {
                    id: id("BAR-002"),
                    manual: true
                },
            ]
        );
    }

    #[test]
    fn finds_rust_references_in_test_function_names_only() {
        let src = "
            #[test]
            fn foo_004_promotes_duplicate() {}
            #[test]
            fn x11y_002() {}
            fn helper_without_id() {}
            // FOO-009 in a comment does not count
            fn foo_12_not_three_digits() {}
        ";
        assert_eq!(references_in_rust(src), ids(&["FOO-004", "X11Y-002"]));
    }

    #[test]
    fn rust_references_inside_comments_are_ignored() {
        let src = "
            //! e.g. `fn foo_001_example()` in module docs
            /// See `fn foo_002_other()`
            // fn foo_003_commented_out() {}
            fn bar_001_real() {} // fn foo_004_trailing
        ";
        assert_eq!(references_in_rust(src), ids(&["BAR-001"]));
    }

    #[test]
    fn finds_ts_references_in_brackets() {
        let src = "it('[FOO-003] filters', () => {}); test('[X11Y-001][BAR-010] both', fn); it('FOO-004 no brackets')";
        assert_eq!(
            references_in_ts(src),
            ids(&["FOO-003", "X11Y-001", "BAR-010"])
        );
    }

    #[test]
    fn scope_supports_exact_ids_and_area_wildcards() {
        let scope = Scope {
            enforced: vec!["FOO-001".into(), "BAR-*".into()],
        };

        assert!(scope.matches(&id("FOO-001")));
        assert!(!scope.matches(&id("FOO-002")));
        assert!(scope.matches(&id("BAR-123")));
        assert!(!scope.matches(&id("BARX-001")));
    }

    #[test]
    fn evaluate_reports_missing_enforced_requirements_but_not_manual_ones() {
        let reqs = vec![
            req("FOO-001"),
            req("FOO-002"),
            Requirement {
                id: id("FOO-003"),
                manual: true,
            },
        ];
        let scope = Scope {
            enforced: vec!["FOO-*".into()],
        };

        let report = evaluate(&reqs, &ids(&["FOO-001"]), &scope);

        assert_eq!(report.missing, vec![id("FOO-002")]);
        assert_eq!(report.covered, vec![id("FOO-001")]);
        assert!(!report.is_ok());
    }

    #[test]
    fn evaluate_ignores_unenforced_requirements() {
        let reqs = vec![req("FOO-001"), req("BAR-001")];
        let scope = Scope {
            enforced: vec!["FOO-001".into()],
        };

        let report = evaluate(&reqs, &ids(&["FOO-001"]), &scope);

        assert!(report.is_ok(), "{report:?}");
    }

    #[test]
    fn evaluate_flags_references_to_unknown_ids_of_known_areas() {
        let reqs = vec![req("FOO-001")];

        let report = evaluate(
            &reqs,
            &ids(&["FOO-001", "FOO-099", "ZZZ-001"]),
            &Scope::default(),
        );

        assert_eq!(report.unknown, vec![id("FOO-099")]);
    }

    #[test]
    fn evaluate_flags_scope_entries_matching_nothing() {
        let reqs = vec![req("FOO-001")];
        let scope = Scope {
            enforced: vec!["FOO-001".into(), "FOO-777".into(), "QUX-*".into()],
        };

        let report = evaluate(&reqs, &ids(&["FOO-001"]), &scope);

        assert_eq!(
            report.dangling_scope,
            vec!["FOO-777".to_string(), "QUX-*".to_string()]
        );
    }

    #[test]
    fn repository_traceability_is_satisfied() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");

        let report = run(&root).unwrap();

        assert!(report.is_ok(), "{report:#?}");
    }
}
