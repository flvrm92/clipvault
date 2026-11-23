//! Version bump check (specs/02 MNT-006): every change merged into `main` declares a version
//! above the latest one published, because the Store refuses a package whose version it has
//! already seen. The published versions are the `vX.Y.Z` git tags.

use std::path::Path;

/// A `major.minor.patch` version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u64, u64, u64);

impl Version {
    /// Parses `X.Y.Z`, or `vX.Y.Z` when `tag` is true.
    pub fn parse(text: &str, tag: bool) -> Option<Self> {
        let text = if tag { text.strip_prefix('v')? } else { text };
        let mut parts = text.split('.').map(|p| {
            // Leading `+` or `-` would parse; only plain digits are versions.
            p.bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| p.parse::<u64>().ok())
                .flatten()
        });
        let version = Self(parts.next()??, parts.next()??, parts.next()??);
        parts.next().is_none().then_some(version)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Problems with the declared `version`, given the release tags of the repository, the tags
/// on `HEAD` and the changelog.
pub fn check(version: &str, tags: &[&str], head_tags: &[&str], changelog: &str) -> Vec<String> {
    let Some(declared) = Version::parse(version, false) else {
        return vec![format!(
            "workspace version `{version}` is not X.Y.Z (Cargo.toml [workspace.package])"
        )];
    };
    let mut problems = Vec::new();
    let released = |t: &&str| Version::parse(t, true);
    let is_release_commit = head_tags.iter().filter_map(released).any(|v| v == declared);
    if let Some(latest) = tags.iter().filter_map(released).max()
        && declared <= latest
        && !is_release_commit
    {
        problems.push(format!(
            "workspace version {declared} is not above the latest release {latest}: bump [workspace.package] version in Cargo.toml (specs/09 §6)"
        ));
    }
    let heading = format!("## [{declared}]");
    if !changelog.lines().any(|line| line.starts_with(&heading)) {
        problems.push(format!("CHANGELOG.md has no `{heading}` section"));
    }
    problems
}

/// Runs the check on the repository at `root`.
///
/// # Errors
/// Fails when `Cargo.toml`, `CHANGELOG.md` or the git tags cannot be read.
pub fn run(root: &Path) -> anyhow::Result<Vec<String>> {
    let manifest: toml::Table = std::fs::read_to_string(root.join("Cargo.toml"))?.parse()?;
    let version = manifest
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Cargo.toml has no [workspace.package] version"))?;
    let changelog = std::fs::read_to_string(root.join("CHANGELOG.md"))?;
    let tags = git_tags(root, &["tag", "--list", "v*"])?;
    let head_tags = git_tags(root, &["tag", "--list", "v*", "--points-at", "HEAD"])?;
    let tags: Vec<&str> = tags.lines().collect();
    let head_tags: Vec<&str> = head_tags.lines().collect();
    Ok(check(version, &tags, &head_tags, &changelog))
}

fn git_tags(root: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()?;
    anyhow::ensure!(output.status.success(), "git {} failed", args.join(" "));
    Ok(String::from_utf8(output.stdout)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Vec<String> = Vec::new();
    const CHANGELOG: &str = "# Changelog\n\n## [1.1.0] - Unreleased\n\n## [1.0.0] - 2026-11-22\n";

    #[test]
    fn mnt_006_a_version_above_the_latest_release_passes() {
        assert_eq!(check("1.1.0", &["v1.0.0", "v0.9.0"], &[], CHANGELOG), NONE);
    }

    #[test]
    fn mnt_006_a_version_already_published_is_reported() {
        let problems = check("1.0.0", &["v1.0.0"], &[], "## [1.0.0] - 2026-11-22\n");
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("1.0.0"), "{problems:?}");
    }

    #[test]
    fn mnt_006_a_version_below_the_latest_release_is_reported() {
        let problems = check("1.0.5", &["v1.0.0", "v1.1.0"], &[], "## [1.0.5]\n");
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("1.1.0"), "{problems:?}");
    }

    #[test]
    fn mnt_006_versions_compare_numerically_not_as_text() {
        assert_eq!(
            check("1.10.0", &["v1.9.0"], &[], "## [1.10.0] - Unreleased\n"),
            NONE
        );
        assert_eq!(check("1.9.0", &["v1.10.0"], &[], "## [1.9.0]\n").len(), 1);
    }

    #[test]
    fn mnt_006_the_release_commit_itself_passes() {
        let changelog = "## [1.0.0] - 2026-11-22\n";
        assert_eq!(check("1.0.0", &["v1.0.0"], &["v1.0.0"], changelog), NONE);
    }

    #[test]
    fn mnt_006_the_changelog_needs_a_section_for_the_version() {
        let problems = check("1.1.0", &["v1.0.0"], &[], "## [1.0.0] - 2026-11-22\n");
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("CHANGELOG"), "{problems:?}");
    }

    #[test]
    fn mnt_006_non_release_tags_are_ignored_and_an_invalid_version_is_reported() {
        assert_eq!(
            check(
                "1.1.0",
                &["v1.0.0", "nightly", "v2.0.0-rc1"],
                &[],
                CHANGELOG
            ),
            NONE
        );
        assert_eq!(check("1.1", &[], &[], "## [1.1]\n").len(), 1);
    }

    #[test]
    fn mnt_006_the_repository_declares_an_unpublished_version() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let problems = run(&root).unwrap_or_else(|e| vec![e.to_string()]);
        assert_eq!(problems, NONE);
    }
}
