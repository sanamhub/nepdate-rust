//! ADR-0018: the default `MitiBar` build must not contain `femtovg` or an `OpenGL` crate.
//!
//! `cargo xtask renderer-check` reads `cargo tree -p mitibar -e features` and fails on a
//! renderer crate name. It is a CI step, so a later `slint` default feature cannot bring the
//! GPU renderer back without a red build. The negative control adds `renderer-femtovg` to the
//! app manifest, runs this command and expects the femtovg line to fail the check.

use std::process::Command;

use anyhow::{Context, Result, bail};

/// Crate or feature names that mean a GPU renderer is in the tree.
const BANNED: [&str; 4] = ["femtovg", "glow", "glutin", "opengl"];

/// Fails when `tree` contains any [`BANNED`] name.
fn check_tree(tree: &str) -> Result<()> {
    let hits: Vec<&str> = tree
        .lines()
        .filter(|line| {
            let line = line.to_ascii_lowercase();
            BANNED.iter().any(|name| line.contains(name))
        })
        .collect();
    if !hits.is_empty() {
        bail!(
            "the default mitibar build contains a GPU renderer (ADR-0018):\n{}",
            hits.join("\n")
        );
    }
    Ok(())
}

/// Runs the check against the real feature tree.
pub(crate) fn run() -> Result<()> {
    let output = Command::new("cargo")
        .args(["tree", "-p", "mitibar", "-e", "features"])
        .output()
        .context("could not run `cargo tree`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`cargo tree -p mitibar -e features` failed: {stderr}");
    }
    let tree = String::from_utf8_lossy(&output.stdout);
    check_tree(&tree)?;
    println!("renderer-check: no femtovg or OpenGL crate in mitibar's feature tree");
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn a_software_only_tree_passes() {
        check_tree("mitibar\n  slint\n    i-slint-renderer-software\n").expect("clean tree");
    }

    #[test]
    fn a_femtovg_line_fails() {
        let err = check_tree("mitibar\n  i-slint-renderer-femtovg\n").expect_err("femtovg");
        assert!(err.to_string().contains("femtovg"));
    }

    #[test]
    fn an_opengl_line_fails() {
        let err = check_tree("mitibar\n  glutin-winit feature \"wgl\"\n").expect_err("opengl");
        assert!(err.to_string().contains("glutin"));
    }
}
