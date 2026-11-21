//! Static checks of the MSIX manifest template (specs/09 §4, CMP-001).
//!
//! Catches drift between the template and the rest of the repository: generated asset names,
//! the executable name, the `StartupTask` id used by the adapter and the supported Windows range.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

/// CMP-001: Windows 10 2004 (19041) is the oldest build the package installs on.
pub const MIN_VERSION: &str = "10.0.19041.0";

/// Placeholders `packaging/msix/build.ps1` knows how to fill.
pub const PLACEHOLDERS: [&str; 5] = [
    "NAME",
    "PUBLISHER",
    "PUBLISHER_DISPLAY_NAME",
    "VERSION",
    "ARCH",
];

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| unreachable!("invalid built-in regex {pattern}: {e}"))
}

static MIN_VERSION_ATTR: LazyLock<Regex> = LazyLock::new(|| {
    regex(r#"<TargetDeviceFamily\s+Name="Windows\.Desktop"\s+MinVersion="([^"]+)""#)
});
static FULL_TRUST: LazyLock<Regex> =
    LazyLock::new(|| regex(r#"<rescap:Capability\s+Name="runFullTrust"\s*/>"#));
static TASK_ID_ATTR: LazyLock<Regex> = LazyLock::new(|| regex(r#"TaskId="([^"]+)""#));
static EXECUTABLE_ATTR: LazyLock<Regex> = LazyLock::new(|| regex(r#"Executable="([^"]+)""#));
static ASSET_REF: LazyLock<Regex> = LazyLock::new(|| regex(r"Assets\\([A-Za-z0-9]+)\.png"));
static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| regex(r"\{\{([A-Z_]+)\}\}"));
static TASK_ID_CONST: LazyLock<Regex> =
    LazyLock::new(|| regex(r#"const\s+TASK_ID:\s*&str\s*=\s*"([^"]+)""#));

/// What the template is checked against.
#[derive(Debug)]
pub struct Expectations<'a> {
    /// File names in `packaging/msix/Assets/`.
    pub asset_files: &'a BTreeSet<String>,
    /// The `StartupTask` id the infrastructure adapter asks Windows for.
    pub task_id: &'a str,
    /// The app binary name (`[[bin]]` in `app/Cargo.toml`) plus `.exe`.
    pub executable: &'a str,
}

fn captures<'t>(re: &Regex, text: &'t str) -> Vec<&'t str> {
    re.captures_iter(text)
        .filter_map(|c| c.get(1).map(|m| m.as_str()))
        .collect()
}

/// A logical asset (`Square44x44Logo`) resolves to `Name.png` or to an MRT-qualified variant
/// (`Name.scale-100.png`), which is how `gen-assets` names them.
fn asset_exists(name: &str, files: &BTreeSet<String>) -> bool {
    files.contains(&format!("{name}.png")) || files.contains(&format!("{name}.scale-100.png"))
}

/// Problems found in `template`; empty when it is consistent.
pub fn check(template: &str, expected: &Expectations<'_>) -> Vec<String> {
    let mut problems = Vec::new();

    match captures(&MIN_VERSION_ATTR, template).as_slice() {
        [MIN_VERSION] => {}
        [other] => problems.push(format!("MinVersion is {other}, expected {MIN_VERSION}")),
        _ => problems.push("missing Windows.Desktop TargetDeviceFamily".into()),
    }
    if !FULL_TRUST.is_match(template) {
        problems.push("missing runFullTrust capability".into());
    }
    let task_ids = captures(&TASK_ID_ATTR, template);
    if task_ids != [expected.task_id] {
        problems.push(format!(
            "StartupTask ids {task_ids:?}, expected [{:?}]",
            expected.task_id
        ));
    }
    let executables = captures(&EXECUTABLE_ATTR, template);
    if executables != [expected.executable] {
        problems.push(format!(
            "Executable {executables:?}, expected [{:?}]",
            expected.executable
        ));
    }
    let assets: BTreeSet<&str> = captures(&ASSET_REF, template).into_iter().collect();
    for asset in assets {
        if !asset_exists(asset, expected.asset_files) {
            problems.push(format!("asset {asset} not found in packaging/msix/Assets"));
        }
    }
    let placeholders: BTreeSet<&str> = captures(&PLACEHOLDER, template).into_iter().collect();
    for placeholder in placeholders {
        if !PLACEHOLDERS.contains(&placeholder) {
            problems.push(format!("unknown placeholder {{{{{placeholder}}}}}"));
        }
    }
    problems
}

/// The `TASK_ID` constant declared in the startup adapter's source.
pub fn task_id_in(source: &str) -> Option<&str> {
    captures(&TASK_ID_CONST, source).first().copied()
}

/// Checks the repository's template against its assets, adapter and binary.
///
/// # Errors
/// Fails if a source file cannot be read.
pub fn run(root: &Path) -> anyhow::Result<Vec<String>> {
    let msix = root.join("packaging").join("msix");
    let template = std::fs::read_to_string(msix.join("AppxManifest.template.xml"))?;
    let startup = std::fs::read_to_string(root.join("crates/infrastructure/src/win32/startup.rs"))?;
    let asset_files = std::fs::read_dir(msix.join("Assets"))?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    let task_id =
        task_id_in(&startup).ok_or_else(|| anyhow::anyhow!("TASK_ID not found in startup.rs"))?;
    Ok(check(
        &template,
        &Expectations {
            asset_files: &asset_files,
            task_id,
            executable: "ClipVault.exe",
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
        <Identity Name="{{NAME}}" Version="{{VERSION}}" />
        <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0" />
        <Application Id="ClipVault" Executable="ClipVault.exe">
          <uap:VisualElements Square44x44Logo="Assets\Square44x44Logo.png" />
          <uap:SplashScreen Image="Assets\SplashScreen.png" />
          <desktop:StartupTask TaskId="ClipVaultStartup" Enabled="false" />
        </Application>
        <rescap:Capability Name="runFullTrust" />
    "#;

    fn files() -> BTreeSet<String> {
        ["Square44x44Logo.scale-100.png", "SplashScreen.png"]
            .into_iter()
            .map(String::from)
            .collect()
    }

    fn check_with(template: &str) -> Vec<String> {
        check(
            template,
            &Expectations {
                asset_files: &files(),
                task_id: "ClipVaultStartup",
                executable: "ClipVault.exe",
            },
        )
    }

    #[test]
    fn a_consistent_template_has_no_problems() {
        assert_eq!(check_with(GOOD), Vec::<String>::new());
    }

    #[test]
    fn cmp_001_an_older_or_missing_min_version_is_reported() {
        let older = GOOD.replace("10.0.19041.0", "10.0.17763.0");
        assert_eq!(
            check_with(&older),
            ["MinVersion is 10.0.17763.0, expected 10.0.19041.0"]
        );
        let missing = GOOD.replace("Windows.Desktop", "Windows.Universal");
        assert_eq!(
            check_with(&missing),
            ["missing Windows.Desktop TargetDeviceFamily"]
        );
    }

    #[test]
    fn missing_assets_capability_task_and_executable_drift_are_reported() {
        let broken = GOOD
            .replace("SplashScreen.png", "Splash.png")
            .replace(r#"<rescap:Capability Name="runFullTrust" />"#, "")
            .replace("ClipVaultStartup", "Other")
            .replace("ClipVault.exe", "clipvault-app.exe")
            .replace("{{VERSION}}", "{{BUILD}}");

        assert_eq!(
            check_with(&broken),
            [
                "missing runFullTrust capability",
                r#"StartupTask ids ["Other"], expected ["ClipVaultStartup"]"#,
                r#"Executable ["clipvault-app.exe"], expected ["ClipVault.exe"]"#,
                "asset Splash not found in packaging/msix/Assets",
                "unknown placeholder {{BUILD}}",
            ]
        );
    }

    #[test]
    fn the_task_id_is_read_from_the_adapter_source() {
        assert_eq!(
            task_id_in(r#"const TASK_ID: &str = "ClipVaultStartup";"#),
            Some("ClipVaultStartup")
        );
        assert_eq!(task_id_in("fn main() {}"), None);
    }

    #[test]
    fn cmp_001_the_repository_manifest_targets_windows_10_19041_and_matches_the_repo() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        assert_eq!(run(&root).unwrap(), Vec::<String>::new());
    }
}
