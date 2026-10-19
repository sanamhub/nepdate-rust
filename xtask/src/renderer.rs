//! ADR-0018: the default `MitiBar` build must not contain `femtovg` or an `OpenGL` crate.
//!
//! `cargo xtask renderer-check` reads `cargo tree -p mitibar -e features --target <t>` for every
//! release target and fails on a renderer crate name. Without `--target`, `cargo tree` shows only
//! the host's target-specific dependencies, so a GPU crate pulled in on Linux or macOS alone
//! would pass on a Windows machine. It is a CI step, so a later `slint` default feature cannot
//! bring the GPU renderer back without a red build. The negative control adds `renderer-femtovg` to the
//! app manifest, runs this command and expects the femtovg line to fail the check.

use std::process::Command;

use anyhow::{Context, Result, bail};

/// Crate-name prefixes that mean a GPU renderer is in the tree. The crate name is matched, not
/// any text on the line: on macOS `winit` itself turns on the `NSOpenGLView` binding feature of
/// `objc2-app-kit`, which is a type declaration, not an OpenGL renderer.
const BANNED_CRATES: [&str; 6] = [
    "femtovg",
    "glow",
    "glutin",
    "i-slint-renderer-femtovg",
    "i-slint-renderer-skia",
    "skia-",
];

/// Slint feature names that select a GPU renderer.
const BANNED_FEATURES: [&str; 2] = ["\"renderer-femtovg", "\"renderer-skia"];

/// The crate name of one `cargo tree` line: the first word after the tree drawing.
fn crate_name(line: &str) -> &str {
    line.trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
        .split_whitespace()
        .next()
        .unwrap_or("")
}

fn is_banned(line: &str) -> bool {
    let name = crate_name(line);
    BANNED_CRATES.iter().any(|banned| name.starts_with(banned))
        || BANNED_FEATURES.iter().any(|feature| line.contains(feature))
}

/// The targets `release-mitibar.yml` builds, with `universal-apple-darwin` as its two halves.
/// A workflow test keeps this list equal to the release matrix.
pub(crate) const TARGETS: [&str; 5] = [
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
];

/// Fails when `tree` contains a banned crate or renderer feature, or when it is not a `mitibar` tree with the
/// software renderer in it (an empty or wrong tree must not pass by containing nothing).
fn check_tree(tree: &str) -> Result<()> {
    if !tree
        .lines()
        .next()
        .is_some_and(|line| line.starts_with("mitibar v"))
    {
        bail!("`cargo tree` output does not start with the mitibar package");
    }
    if !tree.contains("i-slint-renderer-software") {
        bail!("the mitibar feature tree has no software renderer (ADR-0018)");
    }
    let hits: Vec<&str> = tree.lines().filter(|line| is_banned(line)).collect();
    if !hits.is_empty() {
        bail!(
            "the default mitibar build contains a GPU renderer (ADR-0018):\n{}",
            hits.join("\n")
        );
    }
    Ok(())
}

/// Runs the check against the real feature tree of every release target.
pub(crate) fn run() -> Result<()> {
    for target in TARGETS {
        let output = Command::new("cargo")
            .args([
                "tree", "-p", "mitibar", "-e", "features", "--target", target,
            ])
            .output()
            .context("could not run `cargo tree`")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("`cargo tree -p mitibar -e features --target {target}` failed: {stderr}");
        }
        let tree = String::from_utf8_lossy(&output.stdout);
        check_tree(&tree).with_context(|| format!("target {target}"))?;
    }
    println!(
        "renderer-check: no femtovg or OpenGL crate in mitibar's feature tree for {} targets",
        TARGETS.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn a_software_only_tree_passes() {
        check_tree(CLEAN).expect("clean tree");
    }

    const CLEAN: &str = "mitibar v0.1.0\n  slint\n    i-slint-renderer-software\n";

    #[test]
    fn an_empty_tree_fails() {
        let err = check_tree("").expect_err("empty");
        assert!(err.to_string().contains("mitibar package"));
    }

    #[test]
    fn a_tree_without_the_software_renderer_fails() {
        let err = check_tree("mitibar v0.1.0\n  slint\n").expect_err("no renderer");
        assert!(err.to_string().contains("software renderer"));
    }

    #[test]
    fn a_femtovg_line_fails() {
        let tree = format!("{CLEAN}  i-slint-renderer-femtovg\n");
        let err = check_tree(&tree).expect_err("femtovg");
        assert!(err.to_string().contains("femtovg"));
    }

    #[test]
    fn a_skia_renderer_feature_fails() {
        let tree = format!("{CLEAN}  slint feature \"renderer-skia\"\n");
        let err = check_tree(&tree).expect_err("skia");
        assert!(err.to_string().contains("renderer-skia"));
    }

    #[test]
    fn the_macos_open_gl_binding_feature_of_winit_passes() {
        let tree = format!("{CLEAN}  ├── objc2-app-kit feature \"NSOpenGLView\"\n");
        check_tree(&tree).expect("a binding feature is not a renderer");
    }

    #[test]
    fn an_opengl_line_fails() {
        let tree = format!("{CLEAN}  glutin-winit feature \"wgl\"\n");
        let err = check_tree(&tree).expect_err("opengl");
        assert!(err.to_string().contains("glutin"));
    }
}
