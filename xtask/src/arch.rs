//! Layer dependency rule check (specs/02 MNT-001, ADR-0002).
//!
//! The dependency rule is: `domain` ← `application` ← `infrastructure` ← `app`.
//! Core layers must never (even transitively) depend on platform, framework or storage crates.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use anyhow::Context;

/// Directed graph of *normal* (non-dev, non-build) dependencies between packages.
#[derive(Debug, Default, Clone)]
pub struct DependencyGraph {
    edges: BTreeMap<String, BTreeSet<String>>,
}

impl DependencyGraph {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers that `from` depends on `to`.
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.edges
            .entry(from.to_owned())
            .or_default()
            .insert(to.to_owned());
    }

    /// Direct dependencies of `package`.
    pub fn direct(&self, package: &str) -> BTreeSet<String> {
        self.edges.get(package).cloned().unwrap_or_default()
    }

    /// All dependencies reachable from `package` (excluding itself).
    pub fn transitive(&self, package: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut pending: Vec<String> = self.direct(package).into_iter().collect();
        while let Some(next) = pending.pop() {
            if next != package && seen.insert(next.clone()) {
                pending.extend(self.direct(&next));
            }
        }
        seen
    }
}

/// Allowed/forbidden dependencies for one workspace package.
#[derive(Debug, Clone, Copy)]
pub struct LayerRule {
    pub package: &'static str,
    /// Workspace members this package may depend on directly.
    pub allowed_workspace_deps: &'static [&'static str],
    /// External crates that must not appear anywhere in this package's dependency tree.
    pub forbidden_crates: &'static [&'static str],
}

/// A breach of a [`LayerRule`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    MissingPackage { package: String },
    ForbiddenWorkspaceDep { package: String, dependency: String },
    ForbiddenCrate { package: String, dependency: String },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPackage { package } => {
                write!(
                    f,
                    "package `{package}` has a layer rule but is not a workspace member"
                )
            }
            Self::ForbiddenWorkspaceDep {
                package,
                dependency,
            } => {
                write!(
                    f,
                    "`{package}` must not depend on workspace crate `{dependency}`"
                )
            }
            Self::ForbiddenCrate {
                package,
                dependency,
            } => {
                write!(
                    f,
                    "`{package}` must not depend (even transitively) on `{dependency}`"
                )
            }
        }
    }
}

const PLATFORM_AND_STORAGE: &[&str] = &[
    "tauri",
    "tauri-build",
    "windows",
    "windows-sys",
    "windows-core",
    "rusqlite",
    "libsqlite3-sys",
    "image",
];

/// The project's layer rules (specs/03 §1).
pub fn layer_rules() -> Vec<LayerRule> {
    vec![
        LayerRule {
            package: "clipvault-domain",
            allowed_workspace_deps: &[],
            forbidden_crates: PLATFORM_AND_STORAGE,
        },
        LayerRule {
            package: "clipvault-application",
            allowed_workspace_deps: &["clipvault-domain"],
            forbidden_crates: PLATFORM_AND_STORAGE,
        },
        LayerRule {
            package: "clipvault-infrastructure",
            allowed_workspace_deps: &["clipvault-domain", "clipvault-application"],
            forbidden_crates: &["tauri", "tauri-build"],
        },
        LayerRule {
            package: "clipvault-test-support",
            allowed_workspace_deps: &["clipvault-domain", "clipvault-application"],
            forbidden_crates: PLATFORM_AND_STORAGE,
        },
    ]
}

/// Checks `rules` against `graph`. `members` are the workspace package names.
pub fn check(
    graph: &DependencyGraph,
    members: &BTreeSet<String>,
    rules: &[LayerRule],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for rule in rules {
        let package = rule.package.to_owned();
        if !members.contains(rule.package) {
            violations.push(Violation::MissingPackage { package });
            continue;
        }
        violations.extend(
            graph
                .direct(rule.package)
                .into_iter()
                .filter(|dep| members.contains(dep))
                .filter(|dep| !rule.allowed_workspace_deps.contains(&dep.as_str()))
                .map(|dependency| Violation::ForbiddenWorkspaceDep {
                    package: package.clone(),
                    dependency,
                }),
        );
        violations.extend(
            graph
                .transitive(rule.package)
                .into_iter()
                .filter(|dep| rule.forbidden_crates.contains(&dep.as_str()))
                .map(|dependency| Violation::ForbiddenCrate {
                    package: package.clone(),
                    dependency,
                }),
        );
    }
    violations
}

/// Builds the dependency graph of the workspace at `manifest_path` using `cargo metadata`.
///
/// # Errors
/// Fails if `cargo metadata` cannot be executed or its output cannot be parsed.
pub fn load_workspace(manifest_path: &Path) -> anyhow::Result<(DependencyGraph, BTreeSet<String>)> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(manifest_path)
        .exec()
        .context("failed to run `cargo metadata`")?;

    let members = metadata
        .workspace_packages()
        .iter()
        .map(|p| p.name.to_string())
        .collect();

    let resolve = metadata
        .resolve
        .as_ref()
        .context("`cargo metadata` returned no resolve graph")?;
    let mut graph = DependencyGraph::new();
    for node in &resolve.nodes {
        let from = metadata[&node.id].name.to_string();
        for dep in &node.deps {
            let is_normal = dep
                .dep_kinds
                .iter()
                .any(|k| k.kind == cargo_metadata::DependencyKind::Normal);
            if is_normal {
                graph.add_edge(&from, &metadata[&dep.pkg].name);
            }
        }
    }
    Ok((graph, members))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(ToString::to_string).collect()
    }

    const DOMAIN_ONLY: LayerRule = LayerRule {
        package: "core",
        allowed_workspace_deps: &[],
        forbidden_crates: &["windows"],
    };

    #[test]
    fn transitive_follows_all_paths_without_repeating() {
        let mut g = DependencyGraph::new();
        g.add_edge("a", "b");
        g.add_edge("b", "c");
        g.add_edge("a", "c");
        g.add_edge("c", "a"); // cycle must not loop forever

        assert_eq!(g.direct("a"), members(&["b", "c"]));
        assert_eq!(g.transitive("a"), members(&["b", "c"]));
        assert_eq!(g.transitive("b"), members(&["a", "c"]));
    }

    #[test]
    fn clean_graph_has_no_violations() {
        let mut g = DependencyGraph::new();
        g.add_edge("core", "serde");

        assert_eq!(check(&g, &members(&["core"]), &[DOMAIN_ONLY]), vec![]);
    }

    #[test]
    fn direct_dependency_on_disallowed_workspace_member_is_reported() {
        let mut g = DependencyGraph::new();
        g.add_edge("core", "infra");

        let violations = check(&g, &members(&["core", "infra"]), &[DOMAIN_ONLY]);

        assert_eq!(
            violations,
            vec![Violation::ForbiddenWorkspaceDep {
                package: "core".into(),
                dependency: "infra".into()
            }]
        );
    }

    #[test]
    fn transitive_dependency_on_forbidden_crate_is_reported() {
        let mut g = DependencyGraph::new();
        g.add_edge("core", "helper");
        g.add_edge("helper", "windows");

        let violations = check(&g, &members(&["core"]), &[DOMAIN_ONLY]);

        assert_eq!(
            violations,
            vec![Violation::ForbiddenCrate {
                package: "core".into(),
                dependency: "windows".into()
            }]
        );
    }

    #[test]
    fn rule_for_unknown_package_is_reported() {
        let violations = check(&DependencyGraph::new(), &members(&[]), &[DOMAIN_ONLY]);

        assert_eq!(
            violations,
            vec![Violation::MissingPackage {
                package: "core".into()
            }]
        );
    }

    #[test]
    fn mnt_001_workspace_respects_dependency_rule() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
        let (graph, members) = load_workspace(&manifest).unwrap();

        let violations = check(&graph, &members, &layer_rules());

        assert!(violations.is_empty(), "layer violations: {violations:#?}");
    }
}
