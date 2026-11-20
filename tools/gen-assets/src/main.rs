//! `cargo run -p gen-assets [-- --check]` (specs/07 §6).
// A command-line tool reports to the terminal.
#![allow(clippy::print_stdout)]

use std::path::Path;

use anyhow::{Result, bail};
use gen_assets::{Masters, generate, stale, write};

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let check = match std::env::args().nth(1).as_deref() {
        None => false,
        Some("--check") => true,
        Some(other) => bail!("unknown argument {other}; usage: gen-assets [--check]"),
    };
    let assets = generate(&Masters::load(&root)?)?;
    if check {
        let outdated = stale(&root, &assets);
        if !outdated.is_empty() {
            for path in &outdated {
                println!("out of date: {path}");
            }
            bail!(
                "{} asset(s) out of date: run `cargo run -p gen-assets`",
                outdated.len()
            );
        }
        println!("gen-assets: {} assets up to date", assets.len());
    } else {
        let changed = write(&root, &assets)?;
        for path in &changed {
            println!("updated: {path}");
        }
        println!(
            "gen-assets: {} of {} assets updated",
            changed.len(),
            assets.len()
        );
    }
    Ok(())
}
