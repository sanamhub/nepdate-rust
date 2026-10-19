//! `cargo xtask package` (ADR-0012): builds the widget in the `release-widget` profile
//! (unless `MITIBAR_BINARIES_DIR` points at a prebuilt one, as the macOS universal job does) and
//! runs `cargo-packager` with `apps/mitibar/Packager.toml` and the formats for this OS.
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
    // points here. Every other build compiles the release-widget profile now, so the package
    // can never hold a stale binary from an earlier build.
    let prebuilt = std::env::var_os("MITIBAR_BINARIES_DIR");
    if prebuilt.is_none() {
        let status = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .args(["build", "-p", "mitibar", "--profile", "release-widget"])
            .current_dir(&root)
            .status()
            .context("run cargo build for the release-widget profile")?;
        if !status.success() {
            bail!("cargo build -p mitibar --profile release-widget exited with {status}");
        }
    }
    let binaries = prebuilt.map_or_else(
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
            "{} is missing after the build (or in MITIBAR_BINARIES_DIR)",
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
    let formats = formats()?;
    println!(
        "packaging {} for {formats} into {}",
        packaged_binary.display(),
        out.display()
    );
    let mut command = Command::new("cargo");
    command
        .args(["packager", "--config"])
        .arg(&config)
        .arg("--out-dir")
        .arg(&out)
        .args(["--formats", &formats]);
    // The Windows ARM64 job cross-compiles, so cargo-packager must name the installer
    // arm64. The universal macOS build leaves this unset.
    if let Some(target) = std::env::var_os("MITIBAR_TARGET") {
        command.arg("--target").arg(target);
    }
    let status = command
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
///
/// the macOS workflow asks for `app` only, signs `dist/MitiBar.app`, and builds the DMG
/// itself, so the `.app` inside the DMG is signed. `MITIBAR_PACKAGE_FORMATS` is a test/release
/// override, not a user setting.
fn formats() -> Result<String> {
    let default = if cfg!(target_os = "windows") {
        "nsis"
    } else if cfg!(target_os = "macos") {
        "app,dmg"
    } else {
        "appimage,deb"
    };
    let Some(value) = std::env::var_os("MITIBAR_PACKAGE_FORMATS") else {
        return Ok(default.to_owned());
    };
    parse_formats(&value.to_string_lossy())
}

/// Validates a comma-separated `MITIBAR_PACKAGE_FORMATS` value.
fn parse_formats(value: &str) -> Result<String> {
    let allowed = ["app", "dmg", "nsis", "appimage", "deb"];
    if value.trim().is_empty() {
        bail!("MITIBAR_PACKAGE_FORMATS must not be empty");
    }
    for part in value.split(',') {
        if !allowed.contains(&part.trim()) {
            bail!(
                "MITIBAR_PACKAGE_FORMATS has unknown format `{part}`; allowed: {}",
                allowed.join(", ")
            );
        }
    }
    Ok(value.to_owned())
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

/// `x64` or `arm64`, the names the release assets use. : the cross-compiled ARM64 job sets
/// `MITIBAR_ARCH=arm64`; otherwise the host arch decides.
#[cfg(windows)]
fn arch() -> String {
    std::env::var("MITIBAR_ARCH").unwrap_or_else(|_| {
        if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x64"
        }
        .to_owned()
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    /// The NSIS template is a copy of cargo-packager's. It must keep the per-user default under
    /// `Programs`, be the one Packager.toml uses, and name the cargo-packager
    /// version the release workflow installs, so an upgrade cannot leave a stale copy behind.
    #[test]
    fn nsis_template_installs_under_programs_and_matches_the_packager_version() {
        let root = workspace_root();
        let read =
            |path: &str| std::fs::read_to_string(root.join(path)).expect("read a packaging file");
        let template = read("apps/mitibar/packaging/installer.nsi");
        let config = read("apps/mitibar/Packager.toml");
        let workflow = read(".github/workflows/release-mitibar.yml");

        assert!(config.contains("template = \"packaging/installer.nsi\""));
        assert!(!config.contains("preinstallSection"));
        assert_eq!(
            template
                .matches(r#"StrCpy $INSTDIR "$LOCALAPPDATA\Programs\${PRODUCTNAME}""#)
                .count(),
            1
        );
        assert!(!template.contains(r#"StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}""#));
        // The default is only applied when /D left $INSTDIR empty.
        let on_init = template
            .split("Function .onInit")
            .nth(1)
            .and_then(|rest| rest.split("FunctionEnd").next())
            .expect(".onInit");
        assert!(on_init.contains("${If} $INSTDIR == \"\""));
        assert!(on_init.contains("Programs"));

        let version = "0.11.8";
        assert!(template.contains(&format!("cargo-packager {version}")));
        assert!(workflow.contains(&format!("cargo install cargo-packager --version {version}")));
    }

    #[test]
    fn package_formats_accept_the_release_lists_and_reject_typos() {
        assert_eq!(parse_formats("app").expect("app"), "app");
        assert_eq!(
            parse_formats("appimage,deb").expect("linux"),
            "appimage,deb"
        );
        assert!(parse_formats("app,dmg").is_ok());
        assert!(parse_formats("").is_err());
        assert!(parse_formats("nsis,msi").is_err());
    }
}
