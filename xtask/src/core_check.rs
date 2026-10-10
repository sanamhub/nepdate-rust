//! `cargo xtask check-core-has-no-patro` (L5-06): the core crate neither names `nepdate-patro`
//! nor carries patro data (ADR-0005, hub ADR-0004). One event name stands for all of it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::codegen::workspace_root;

/// An event name that only the patro data contains (PATRO §4, 2081-04-15).
const NEEDLE: &[u8] = b"World Day Against Human Trafficking";

pub(crate) fn run() -> Result<()> {
    let root = workspace_root();
    manifest(&root)?;
    packaged_files(&root)?;
    let target = target_dir(&root);

    // The dev profile on purpose: with `lto = "fat"` a release rlib holds LLVM bitcode, where a
    // string isn't stored as plain bytes.
    cargo(&root, &["build", "-p", "nepdate"])?;
    let core = target.join("debug/libnepdate.rlib");
    if contains(&core)? {
        bail!("{} contains patro data", core.display());
    }
    // Positive control: if the patro rlib didn't match either, the search itself is broken.
    cargo(&root, &["build", "-p", "nepdate-patro"])?;
    let patro = target.join("debug/libnepdate_patro.rlib");
    if !contains(&patro)? {
        bail!(
            "{} doesn't contain the event name, so the search is broken",
            patro.display()
        );
    }
    println!("check-core-has-no-patro: core manifest, package and rlib have no patro data");
    Ok(())
}

/// Step 1: the core's manifest doesn't name the patro crate.
fn manifest(root: &Path) -> Result<()> {
    let path = root.join("crates/nepdate/Cargo.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    if text.contains("nepdate-patro") || text.contains("nepdate_patro") {
        bail!("{} names nepdate-patro", path.display());
    }
    Ok(())
}

/// Step 2: no file that `cargo package` would publish for the core holds patro data.
fn packaged_files(root: &Path) -> Result<()> {
    let out = Command::new("cargo")
        .args(["package", "-p", "nepdate", "--list", "--allow-dirty"])
        .current_dir(root)
        .output()
        .context("running cargo package --list")?;
    if !out.status.success() {
        bail!(
            "cargo package --list failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let crate_dir = root.join("crates/nepdate");
    let listed = String::from_utf8(out.stdout)?;
    let mut checked = 0;
    for name in listed.lines() {
        // `Cargo.toml` is rewritten by cargo and `.cargo_vcs_info.json` generated, so only
        // files that exist on disk are read.
        let path = crate_dir.join(name);
        if path.is_file() {
            checked += 1;
            if contains(&path)? {
                bail!(
                    "{} is in the core package and contains patro data",
                    path.display()
                );
            }
        }
    }
    if checked == 0 {
        bail!(
            "cargo package --list listed no files under {}",
            crate_dir.display()
        );
    }
    Ok(())
}

fn target_dir(root: &Path) -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

fn cargo(root: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(root)
        .status()
        .with_context(|| format!("running cargo {}", args.join(" ")))?;
    if !status.success() {
        bail!("`cargo {}` exited with {status}", args.join(" "));
    }
    Ok(())
}

fn contains(path: &Path) -> Result<bool> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(bytes.windows(NEEDLE.len()).any(|window| window == NEEDLE))
}
