//! `cargo xtask <task>` — project checks used locally and in CI.
//!
//! Tasks:
//! - `check-arch`  — enforces the layer dependency rule (specs/02 MNT-001).
//! - `trace-reqs`  — requirement ↔ test traceability (specs/08 §4).
//! - `check-version` — the version is above the latest release tag (specs/02 MNT-006).
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use xtask::{arch, trace, version};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn check_arch() -> anyhow::Result<bool> {
    let (graph, members) = arch::load_workspace(&workspace_root().join("Cargo.toml"))?;
    let violations = arch::check(&graph, &members, &arch::layer_rules());
    for v in &violations {
        eprintln!("error: {v}");
    }
    if violations.is_empty() {
        println!(
            "check-arch: OK — dependency rule respected by {} rules",
            arch::layer_rules().len()
        );
    }
    Ok(violations.is_empty())
}

fn trace_reqs() -> anyhow::Result<bool> {
    let report = trace::run(&workspace_root())?;
    println!(
        "trace-reqs: {} requirement(s) covered by tests",
        report.covered.len()
    );
    for id in &report.covered {
        println!("  ✔ {id}");
    }
    for id in &report.missing {
        eprintln!("error: enforced requirement {id} has no test");
    }
    for id in &report.unknown {
        eprintln!("error: tests reference unknown requirement {id}");
    }
    for entry in &report.dangling_scope {
        eprintln!("error: specs/trace-scope.toml entry `{entry}` matches no requirement");
    }
    Ok(report.is_ok())
}

fn check_version() -> anyhow::Result<bool> {
    let problems = version::run(&workspace_root())?;
    for p in &problems {
        eprintln!("error: {p}");
    }
    if problems.is_empty() {
        println!("check-version: OK — version is above the latest release tag");
    }
    Ok(problems.is_empty())
}

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    let result = match task.as_deref() {
        Some("check-arch") => check_arch(),
        Some("trace-reqs") => trace_reqs(),
        Some("check-version") => check_version(),
        _ => {
            eprintln!("usage: cargo xtask <check-arch|trace-reqs|check-version>");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
