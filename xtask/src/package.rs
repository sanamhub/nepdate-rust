//! `cargo xtask package` (W4-03, ADR-0012): builds the widget in the `release-widget` profile
//! through `apps/mitibar/Packager.toml` and runs `cargo-packager` with the formats for this OS.
//! Windows also gets the portable `.zip` that ADR-0012 lists next to the NSIS installer.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::codegen::workspace_root;

pub(crate) fn run() -> Result<()> {
    let root = workspace_root();
    let config = root.join("apps/mitibar/Packager.toml");
    if !config.is_file() {
        bail!("{} is missing", config.display());
    }

    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(
        || root.join("target"),
        |dir| {
            let dir = PathBuf::from(dir);
            if dir.is_absolute() {
                dir
            } else {
                root.join(dir)
            }
        },
    );
    // The macOS workflow joins both arches with lipo into target/universal/release-widget and
    // points here; every other build uses the profile directory.
    let binaries = std::env::var_os("MITIBAR_BINARIES_DIR").map_or_else(
        || target.join("release-widget"),
        |dir| {
            let dir = PathBuf::from(dir);
            if dir.is_absolute() {
                dir
            } else {
                root.join(dir)
            }
        },
    );
    let binary = binaries.join(exe_name());
    if !binary.is_file() {
        bail!(
            "{} is missing; run `cargo build -p mitibar --profile release-widget` first",
            binary.display()
        );
    }

    let out = root.join("dist");
    std::fs::create_dir_all(&out).with_context(|| format!("create {}", out.display()))?;
    // cargo-packager 0.11 ignores `--binaries-dir` in this build and reads the binary from the
    // output directory, so put the release binary there. The packages are written next to it.
    let packaged_binary = out.join(exe_name());
    std::fs::copy(&binary, &packaged_binary)
        .with_context(|| format!("copy {} to {}", binary.display(), packaged_binary.display()))?;
    let formats = formats();
    println!(
        "packaging {} for {formats} into {}",
        packaged_binary.display(),
        out.display()
    );
    let status = Command::new("cargo")
        .args(["packager", "--config"])
        .arg(&config)
        .arg("--out-dir")
        .arg(&out)
        .args(["--formats", formats])
        .current_dir(&root)
        .status()
        .context("run cargo-packager (install it with `cargo install cargo-packager --locked`)")?;
    if !status.success() {
        bail!("cargo packager exited with {status}");
    }

    #[cfg(windows)]
    portable_zip(&root, &out, &binary)?;

    // Keep only the release assets: the copied input binary and cargo-packager's scratch folder
    // must not be uploaded to the release.
    let _ = std::fs::remove_file(&packaged_binary);
    let _ = std::fs::remove_dir_all(out.join(".cargo-packager"));

    println!("packages in {}", out.display());
    Ok(())
}

/// The package formats for this OS, per the ADR-0012 table. cargo-packager gets one list, and
/// passing only this OS's formats keeps it from trying another OS's tools.
fn formats() -> &'static str {
    if cfg!(target_os = "windows") {
        "nsis"
    } else if cfg!(target_os = "macos") {
        "app,dmg"
    } else {
        "appimage,deb"
    }
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "mitibar.exe"
    } else {
        "mitibar"
    }
}

/// The portable Windows build: one `.zip` with `MitiBar.exe`, next to the NSIS installer. `tar`
/// on Windows 10 and 11 is bsdtar, whose `-a` picks the zip format from the extension.
#[cfg(windows)]
fn portable_zip(root: &Path, out: &Path, binary: &Path) -> Result<()> {
    let version = package_version(&root.join("apps/mitibar/Cargo.toml"))?;
    let temp = std::env::temp_dir().join(format!("mitibar-portable-{}", std::process::id()));
    if temp.exists() {
        std::fs::remove_dir_all(&temp).with_context(|| format!("remove {}", temp.display()))?;
    }
    std::fs::create_dir_all(&temp).with_context(|| format!("create {}", temp.display()))?;
    let named = temp.join("MitiBar.exe");
    std::fs::copy(binary, &named)
        .with_context(|| format!("copy {} to {}", binary.display(), named.display()))?;
    let zip = out.join(format!("MitiBar-{version}-windows-{}-portable.zip", arch()));
    let status = Command::new("tar")
        .arg("-a")
        .arg("-c")
        .arg("-f")
        .arg(&zip)
        .arg("-C")
        .arg(&temp)
        .arg("MitiBar.exe")
        .status()
        .context("run tar for the portable zip")?;
    let _ = std::fs::remove_dir_all(&temp);
    if !status.success() {
        bail!("tar exited with {status}");
    }
    println!("portable {}", zip.display());
    Ok(())
}

/// The app version from `apps/mitibar/Cargo.toml`, for the portable zip name.
#[cfg(windows)]
fn package_version(manifest: &Path) -> Result<String> {
    let text = std::fs::read_to_string(manifest)
        .with_context(|| format!("read {}", manifest.display()))?;
    text.lines()
        .skip_while(|line| line.trim() != "[package]")
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['))
        .find_map(|line| line.trim().strip_prefix("version = \""))
        .and_then(|rest| rest.split_once('"'))
        .map(|(version, _)| version.to_owned())
        .with_context(|| format!("{} has no package version", manifest.display()))
}

/// `x64` or `arm64`, the names the release assets use.
#[cfg(windows)]
fn arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    }
}
