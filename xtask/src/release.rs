//! `cargo xtask release-check <package> <version>` and `cargo xtask changelog <package> <version>`
//! (ADR-0012). `<package>` is `nepdate` (both library crates, released together) or
//! `mitibar` (ADR-0017). The checks work on file contents, so the tests use small strings.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::codegen::workspace_root;

const REPO: &str = "https://github.com/sanamhub/nepdate-rust";

/// The files of one package, read once.
struct Package {
    name: &'static str,
    manifest: String,
    changelog: String,
    /// The cargo-packager config, for `mitibar` only. Its version must follow the
    /// manifest's, or an installer would be named for the wrong release.
    packager: Option<String>,
}

impl Package {
    fn read(root: &Path, name: &'static str, dir: &str) -> Result<Self> {
        let dir = root.join(dir);
        let read = |file: &str| {
            let path = dir.join(file);
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
        };
        Ok(Self {
            name,
            manifest: read("Cargo.toml")?,
            changelog: read("CHANGELOG.md")?,
            packager: None,
        })
    }

    fn with_packager(mut self, root: &Path) -> Result<Self> {
        let path = root.join("apps/mitibar/Packager.toml");
        self.packager =
            Some(fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?);
        Ok(self)
    }
}

/// The packages a release tag covers: `nepdate` releases both library crates (ADR-0002).
fn packages(package: &str) -> Result<Vec<Package>> {
    let root = workspace_root();
    match package {
        "nepdate" => Ok(vec![
            Package::read(&root, "nepdate", "crates/nepdate")?,
            Package::read(&root, "nepdate-patro", "crates/nepdate-patro")?,
        ]),
        "mitibar" => Ok(vec![
            Package::read(&root, "mitibar", "apps/mitibar")?.with_packager(&root)?,
        ]),
        other => bail!("unknown package `{other}`: expected nepdate or mitibar"),
    }
}

pub(crate) fn run_check(package: &str, version: &str) -> Result<()> {
    let problems = check(&packages(package)?, version);
    if problems.is_empty() {
        println!("release-check: {package} {version} is ready");
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("release-check: {problem}");
        }
        bail!(
            "release-check: {package} {version} failed with {} problem(s)",
            problems.len()
        )
    }
}

pub(crate) fn run_changelog(package: &str, version: &str) -> Result<()> {
    if !is_version(version) {
        bail!("`{version}` is not a version like 1.2.3 or 1.2.3-beta.1");
    }
    print!("{}", notes(package, &packages(package)?, version)?);
    Ok(())
}

/// Every problem that stops a release of `packages` at `version`; empty when it may go out.
fn check(packages: &[Package], version: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if !is_version(version) {
        problems.push(format!(
            "`{version}` is not a version like 1.2.3 or 1.2.3-beta.1"
        ));
        return problems;
    }
    for package in packages {
        match package_version(&package.manifest) {
            Some(found) if found == version => {}
            Some(found) => problems.push(format!(
                "{}: Cargo.toml has version {found}, expected {version}",
                package.name
            )),
            None => problems.push(format!("{}: Cargo.toml has no version", package.name)),
        }
        if !has_dated_heading(&package.changelog, version) {
            problems.push(format!(
                "{}: CHANGELOG.md has no `## [{version}] - YYYY-MM-DD` section",
                package.name
            ));
        }
        if let Some(packager) = &package.packager {
            match top_level_version(packager) {
                Some(found) if found == version => {}
                Some(found) => problems.push(format!(
                    "{}: Packager.toml has version {found}, expected {version}",
                    package.name
                )),
                None => problems.push(format!("{}: Packager.toml has no version", package.name)),
            }
        }
    }
    // The patro crate follows the core's minor version (hub ADR-0004 §3, ADR-0002). A
    // pre-release pins the exact core, because `~0.1.0` does not match `0.1.0-beta.1`.
    if let Some(patro) = packages.iter().find(|p| p.name == "nepdate-patro") {
        let expected = core_requirement(version);
        match dependency_version(&patro.manifest, "nepdate") {
            Some(found) if found == expected => {}
            Some(found) => problems.push(format!(
                "nepdate-patro: requires nepdate `{found}`, expected `{expected}`"
            )),
            None => problems.push("nepdate-patro: no version on its nepdate dependency".into()),
        }
    }
    problems
}

/// The release notes: each package's changelog section, then Install and Verification.
fn notes(package: &str, packages: &[Package], version: &str) -> Result<String> {
    let mut out = String::new();
    let several = packages.len() > 1;
    for p in packages {
        let text = section(&p.changelog, version)
            .with_context(|| format!("{}: CHANGELOG.md has no `## [{version}]` section", p.name))?;
        if several {
            // One level down, so each crate's "### Added" sits under its "### <crate>".
            let _ = write!(out, "### {}\n\n", p.name);
            for line in text.lines() {
                if line.starts_with("### ") {
                    out.push('#');
                }
                out.push_str(line);
                out.push('\n');
            }
        } else {
            out.push_str(text);
            out.push('\n');
        }
        out.push('\n');
    }
    // ADR-0012 tag names: `nepdate-vX.Y.Z` and `mitibar-vX.Y.Z` (ADR-0017 §2).
    let tag = format!("{package}-v{version}");
    if package == "mitibar" {
        let _ = write!(
            out,
            "### Install\n\nDownload the installer for your system from the assets below.\n\n\
             ### Verification\n\nCheck the download against `SHA256SUMS`, or its build \
             provenance with `gh attestation verify <file> --repo sanamhub/nepdate-rust`.\n\n\
             Full changelog: {REPO}/blob/{tag}/apps/mitibar/CHANGELOG.md\n"
        );
    } else {
        // A pre-release is pinned exactly: cargo never picks one for a plain requirement.
        let pin = if is_prerelease(version) { "=" } else { "" };
        let mut install = String::new();
        let mut links = Vec::new();
        for p in packages {
            let _ = writeln!(install, "cargo add {}@{pin}{version}", p.name);
            links.push(format!(
                "[{}]({REPO}/blob/{tag}/crates/{}/CHANGELOG.md)",
                p.name, p.name
            ));
        }
        let _ = write!(
            out,
            "### Install\n\n```sh\n{install}```\n\n\
             ### Verification\n\nPublished to crates.io by the `release-lib.yml` workflow of \
             {REPO} after the `production` approval. The same run then builds both crates from \
             crates.io in a new cargo project and checks one conversion and one patro lookup on \
             Linux, macOS and Windows (`verify-published.yml`). \
             Full changelogs: {}.\n",
            links.join(", ")
        );
    }
    Ok(out)
}

/// Semver `1.2.3` or `1.2.3-beta.1`: three numbers without leading zeros, then an optional
/// pre-release of dot-separated identifiers. Build metadata (`+...`) is refused.
fn is_version(text: &str) -> bool {
    let (core, pre) = match text.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (text, None),
    };
    let numeric = |p: &str| {
        !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) && (p == "0" || !p.starts_with('0'))
    };
    let parts: Vec<&str> = core.split('.').collect();
    let pre_ok = pre.is_none_or(|pre| {
        pre.split('.').all(|id| {
            !id.is_empty()
                && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && (!id.bytes().all(|b| b.is_ascii_digit()) || numeric(id))
        })
    });
    parts.len() == 3 && parts.iter().all(|p| numeric(p)) && pre_ok
}

fn is_prerelease(version: &str) -> bool {
    version.contains('-')
}

/// The requirement `nepdate-patro` must have on `nepdate`: `~X.Y.0` for `X.Y.Z`, and the exact
/// `=X.Y.Z-pre` for a pre-release, which no tilde or caret requirement matches.
fn core_requirement(version: &str) -> String {
    if is_prerelease(version) {
        return format!("={version}");
    }
    let mut parts = version.split('.');
    let major = parts.next().unwrap_or("0");
    let minor = parts.next().unwrap_or("0");
    format!("~{major}.{minor}.0")
}

/// Whether the changelog has `## [version] - YYYY-MM-DD`. A beta heading does not satisfy the
/// stable version or the other way round, because the version in brackets is matched exactly.
fn has_dated_heading(changelog: &str, version: &str) -> bool {
    let prefix = format!("## [{version}] - ");
    changelog.lines().any(|line| {
        line.trim_end().strip_prefix(&prefix).is_some_and(|date| {
            date.len() == 10
                && date.bytes().enumerate().all(|(i, c)| {
                    if i == 4 || i == 7 {
                        c == b'-'
                    } else {
                        c.is_ascii_digit()
                    }
                })
        })
    })
}

/// The lines of one TOML table (`[package]`), without the header.
fn table<'a>(toml: &'a str, name: &str) -> impl Iterator<Item = &'a str> {
    let header = format!("[{name}]");
    toml.lines()
        .map(str::trim)
        .skip_while(move |line| *line != header)
        .skip(1)
        .take_while(|line| !line.starts_with('['))
}

/// The value of `key = "value"` on one line.
fn string_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?.trim_start().strip_prefix('=')?;
    let rest = rest.trim_start().strip_prefix('"')?;
    rest.split_once('"').map(|(value, _)| value)
}

fn package_version(manifest: &str) -> Option<&str> {
    table(manifest, "package").find_map(|line| string_value(line, "version"))
}

/// `version` at the top of a TOML file, before its first `[table]` (Packager.toml).
fn top_level_version(toml: &str) -> Option<&str> {
    toml.lines()
        .map(str::trim)
        .take_while(|line| !line.starts_with('['))
        .find_map(|line| string_value(line, "version"))
}

/// `version` inside `name = { ... version = "..." ... }` under `[dependencies]`.
fn dependency_version<'a>(manifest: &'a str, name: &str) -> Option<&'a str> {
    let line = table(manifest, "dependencies").find(|line| {
        line.strip_prefix(name)
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    })?;
    let at = line.find("version")?;
    string_value(line.get(at..)?, "version")
}

/// The body of `## [version]` (heading and surrounding blank lines left out), up to the next
/// `## ` heading or the end of the file.
fn section<'a>(changelog: &'a str, version: &str) -> Option<&'a str> {
    let heading = format!("## [{version}]");
    let mut offset = 0;
    let mut start = None;
    for line in changelog.split_inclusive('\n') {
        let trimmed = line.trim_end();
        match start {
            None if trimmed == heading || trimmed.starts_with(&format!("{heading} ")) => {
                start = Some(offset + line.len());
            }
            Some(begin) if trimmed.starts_with("## ") => {
                return Some(changelog.get(begin..offset)?.trim());
            }
            _ => {}
        }
        offset += line.len();
    }
    start
        .and_then(|begin| changelog.get(begin..))
        .map(str::trim)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing)]
    use super::*;

    fn manifest(version: &str, dep: Option<&str>) -> String {
        let mut text = format!("[package]\nname = \"x\"\nversion = \"{version}\"\n");
        if let Some(dep) = dep {
            let _ = write!(
                text,
                "\n[dependencies]\nnepdate = {{ path = \"../nepdate\", version = \"{dep}\", default-features = false }}\n"
            );
        }
        text
    }

    const CHANGELOG: &str = "# Changelog\n\n## [Unreleased]\n\n## [0.1.0] - 2026-10-10\n\n\
                             ### Added\n\n- Everything.\n\n## [0.0.1] - 2026-01-01\n\n- Old.\n";

    fn pair(core: &str, patro: &str, dep: &str, changelog: &str) -> Vec<Package> {
        vec![
            Package {
                name: "nepdate",
                manifest: manifest(core, None),
                changelog: changelog.into(),
                packager: None,
            },
            Package {
                name: "nepdate-patro",
                manifest: manifest(patro, Some(dep)),
                changelog: changelog.into(),
                packager: None,
            },
        ]
    }

    #[test]
    fn ready_pair_passes() {
        assert!(check(&pair("0.1.0", "0.1.0", "~0.1.0", CHANGELOG), "0.1.0").is_empty());
    }

    #[test]
    fn missing_section_fails() {
        let problems = check(&pair("0.2.0", "0.2.0", "~0.2.0", CHANGELOG), "0.2.0");
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(
            problems
                .iter()
                .all(|p| p.contains("no `## [0.2.0] - YYYY-MM-DD` section"))
        );
    }

    #[test]
    fn wrong_version_fails() {
        let problems = check(&pair("0.0.9", "0.1.0", "~0.1.0", CHANGELOG), "0.1.0");
        assert_eq!(
            problems,
            ["nepdate: Cargo.toml has version 0.0.9, expected 0.1.0"]
        );
    }

    #[test]
    fn patro_version_differs_fails() {
        let problems = check(&pair("0.1.0", "0.1.1", "~0.1.0", CHANGELOG), "0.1.0");
        assert_eq!(
            problems,
            ["nepdate-patro: Cargo.toml has version 0.1.1, expected 0.1.0"]
        );
    }

    #[test]
    fn old_core_requirement_fails() {
        let problems = check(&pair("0.1.0", "0.1.0", "~0.0.0", CHANGELOG), "0.1.0");
        assert_eq!(
            problems,
            ["nepdate-patro: requires nepdate `~0.0.0`, expected `~0.1.0`"]
        );
    }

    #[test]
    fn patch_release_keeps_the_minor_requirement() {
        assert_eq!(core_requirement("0.1.3"), "~0.1.0");
        assert_eq!(core_requirement("1.2.0-beta.1"), "=1.2.0-beta.1");
    }

    #[test]
    fn bad_version_text_fails() {
        for bad in [
            "0.1",
            "v0.1.0",
            "0.1.0-",
            "0.1.x",
            "",
            "01.1.0",
            "0.1.0-beta..1",
            "0.1.0-beta.01",
            "0.1.0+build.1",
            "0.1.0-beta_1",
        ] {
            assert!(!is_version(bad), "{bad}");
        }
        for good in ["0.2.0-beta.1", "1.0.0-rc.10", "0.1.0-0.3.7", "10.20.30"] {
            assert!(is_version(good), "{good}");
        }
    }

    const BETA_CHANGELOG: &str = "# Changelog\n\n## [Unreleased]\n\n## [0.1.0-beta.1] - 2026-10-22\n\n\
                                  ### Added\n\n- First.\n";

    #[test]
    fn beta_pair_passes_with_an_exact_core_pin() {
        let pair = pair(
            "0.1.0-beta.1",
            "0.1.0-beta.1",
            "=0.1.0-beta.1",
            BETA_CHANGELOG,
        );
        assert!(check(&pair, "0.1.0-beta.1").is_empty());
    }

    #[test]
    fn beta_with_a_tilde_core_requirement_fails() {
        // `~0.1.0` does not match 0.1.0-beta.1, so patro could not resolve its own core.
        let problems = check(
            &pair("0.1.0-beta.1", "0.1.0-beta.1", "~0.1.0", BETA_CHANGELOG),
            "0.1.0-beta.1",
        );
        assert_eq!(
            problems,
            ["nepdate-patro: requires nepdate `~0.1.0`, expected `=0.1.0-beta.1`"]
        );
    }

    #[test]
    fn beta_and_stable_sections_are_not_interchangeable() {
        let stable_for_beta = check(
            &pair("0.1.0-beta.1", "0.1.0-beta.1", "=0.1.0-beta.1", CHANGELOG),
            "0.1.0-beta.1",
        );
        assert_eq!(stable_for_beta.len(), 2, "{stable_for_beta:?}");
        let beta_for_stable = check(&pair("0.1.0", "0.1.0", "~0.1.0", BETA_CHANGELOG), "0.1.0");
        assert_eq!(beta_for_stable.len(), 2, "{beta_for_stable:?}");
    }

    #[test]
    fn an_undated_heading_fails() {
        for heading in [
            "## [0.1.0]\n",
            "## [0.1.0] - \n",
            "## [0.1.0] - soon\n",
            "## [0.1.0] - 2026-10\n",
        ] {
            assert!(!has_dated_heading(heading, "0.1.0"), "{heading}");
        }
        assert!(has_dated_heading("## [0.1.0] - 2026-10-22\r\n", "0.1.0"));
    }

    #[test]
    fn beta_notes_pin_the_exact_version() {
        let text = notes(
            "nepdate",
            &pair(
                "0.1.0-beta.1",
                "0.1.0-beta.1",
                "=0.1.0-beta.1",
                BETA_CHANGELOG,
            ),
            "0.1.0-beta.1",
        )
        .unwrap_or_default();
        assert!(
            text.contains(
                "cargo add nepdate@=0.1.0-beta.1\ncargo add nepdate-patro@=0.1.0-beta.1\n"
            )
        );
    }

    /// The real files: both crates must pass the check for the newest dated version in their
    /// changelog, so the repository is always one tag away from a release.
    #[test]
    fn repository_passes_for_its_newest_changelog_version() {
        let packages = packages("nepdate").expect("read the crates");
        let version = newest_dated_version(&packages[0].changelog).expect("a dated section");
        assert!(
            check(&packages, version).is_empty(),
            "{:?}",
            check(&packages, version)
        );
    }

    /// The version of the first dated `## [X] - YYYY-MM-DD` heading, skipping `[Unreleased]`.
    fn newest_dated_version(changelog: &str) -> Option<&str> {
        changelog.lines().find_map(|line| {
            let rest = line.strip_prefix("## [")?;
            let (version, _) = rest.split_once("] - ")?;
            has_dated_heading(line, version).then_some(version)
        })
    }

    #[test]
    fn section_in_the_middle() {
        assert_eq!(
            section(CHANGELOG, "0.1.0"),
            Some("### Added\n\n- Everything.")
        );
        assert_eq!(section(CHANGELOG, "Unreleased"), Some(""));
    }

    #[test]
    fn section_last_in_file() {
        assert_eq!(section(CHANGELOG, "0.0.1"), Some("- Old."));
        assert_eq!(section("## [1.0.0]\n- Last.", "1.0.0"), Some("- Last."));
    }

    #[test]
    fn section_needs_the_exact_version() {
        assert_eq!(section(CHANGELOG, "0.1"), None);
        assert_eq!(section(CHANGELOG, "0.0.10"), None);
    }

    #[test]
    fn crlf_files_work() {
        let text = CHANGELOG.replace('\n', "\r\n");
        assert_eq!(section(&text, "0.0.1").map(str::trim), Some("- Old."));
        let manifest = manifest("0.1.0", Some("~0.1.0")).replace('\n', "\r\n");
        assert_eq!(package_version(&manifest), Some("0.1.0"));
        assert_eq!(dependency_version(&manifest, "nepdate"), Some("~0.1.0"));
    }

    #[test]
    fn dev_dependency_is_not_the_requirement() {
        let text = "[package]\nversion = \"0.1.0\"\n[dependencies]\nnepdate-x = { version = \"9\" }\n\
                    [dev-dependencies]\nnepdate = { version = \"~0.1.0\" }\n";
        assert_eq!(dependency_version(text, "nepdate"), None);
    }

    #[test]
    fn notes_for_the_pair() {
        let text = notes(
            "nepdate",
            &pair("0.1.0", "0.1.0", "~0.1.0", CHANGELOG),
            "0.1.0",
        )
        .unwrap_or_default();
        assert!(
            text.starts_with("### nepdate\n\n#### Added\n\n- Everything.\n\n### nepdate-patro\n")
        );
        assert!(text.contains("cargo add nepdate@0.1.0\ncargo add nepdate-patro@0.1.0\n"));
        assert!(text.contains("/blob/nepdate-v0.1.0/crates/nepdate-patro/CHANGELOG.md"));
        assert!(!text.contains("## [0.1.0]"));
    }

    #[test]
    fn notes_need_the_section() {
        assert!(
            notes(
                "nepdate",
                &pair("0.2.0", "0.2.0", "~0.2.0", CHANGELOG),
                "0.2.0"
            )
            .is_err()
        );
    }

    #[test]
    fn widget_notes_use_the_mitibar_tag() {
        let packages = [Package {
            name: "mitibar",
            manifest: manifest("0.1.0", None),
            changelog: CHANGELOG.into(),
            packager: Some("version = \"0.1.0\"\n[nsis]\ninstallMode = \"currentUser\"\n".into()),
        }];
        let text = notes("mitibar", &packages, "0.1.0").unwrap_or_default();
        assert!(text.contains("/blob/mitibar-v0.1.0/apps/mitibar/CHANGELOG.md"));
        assert!(text.contains("### Install"));
        assert!(!text.contains("widget-v"));
    }

    #[test]
    fn packager_version_must_match_the_manifest() {
        let packages = |packager: &str| {
            [Package {
                name: "mitibar",
                manifest: manifest("0.1.0", None),
                changelog: CHANGELOG.into(),
                packager: Some(packager.into()),
            }]
        };
        assert!(check(&packages("version = \"0.1.0\"\n"), "0.1.0").is_empty());
        let problems = check(&packages("version = \"0.0.0\"\n"), "0.1.0");
        assert_eq!(
            problems,
            ["mitibar: Packager.toml has version 0.0.0, expected 0.1.0"]
        );
    }
}
