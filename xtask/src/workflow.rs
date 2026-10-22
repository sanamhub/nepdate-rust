//! Static check of `.github/workflows/release-mitibar.yml`. The workflow needs GitHub Actions,
//! so this test parses the YAML and checks the structure it requires: triggers, pinned actions, least privilege, signing gates, checksums, SBOM and
//! a draft release. Comments are ignored by the parser, not by string matching.

use std::path::Path;

use anyhow::{Context, Result, bail};
use yaml_rust2::yaml::Hash as Mapping;
use yaml_rust2::{Yaml as Value, YamlLoader};

use crate::codegen::workspace_root;

const WORKFLOW: &str = ".github/workflows/release-mitibar.yml";

pub(crate) fn check(root: &Path) -> Result<()> {
    let path = root.join(WORKFLOW);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    check_text(&text).with_context(|| format!("check {}", path.display()))
}

/// Parses one YAML document and checks it.
fn check_text(text: &str) -> Result<()> {
    check_value(&parse(text)?)
}

fn parse(text: &str) -> Result<Value> {
    let mut docs = YamlLoader::load_from_str(text).context("parse the workflow as YAML")?;
    if docs.len() != 1 {
        bail!("expected one YAML document, found {}", docs.len());
    }
    Ok(docs.remove(0))
}

fn check_value(value: &Value) -> Result<()> {
    let root = map(value)?;
    if field(root, "name")?.as_str() != Some("release-mitibar") {
        bail!("workflow name must be release-mitibar");
    }
    check_pinned_actions(value)?;
    check_triggers(root)?;
    check_permissions(root)?;
    check_jobs(root)?;
    check_no_expressions_in_run(root)?;
    Ok(())
}

/// Every `uses` anywhere must be `owner/repo@<40 lowercase hex>`. A tag, branch, local path or
/// expression is rejected.
fn check_pinned_actions(value: &Value) -> Result<()> {
    let mut uses = Vec::new();
    collect_uses(value, &mut uses);
    for action in uses {
        let Some((name, reference)) = action.split_once('@') else {
            bail!("action `{action}` has no pinned commit");
        };
        if name.is_empty()
            || name.starts_with('.')
            || reference.len() != 40
            || !reference.bytes().all(|b| b.is_ascii_hexdigit())
        {
            bail!("action `{action}` is not pinned to a 40-digit commit");
        }
    }
    Ok(())
}

fn collect_uses<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
    match value {
        Value::Hash(map) => {
            for (key, child) in map {
                if key.as_str() == Some("uses") {
                    if let Some(action) = child.as_str() {
                        out.push(action);
                    }
                } else {
                    collect_uses(child, out);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_uses(item, out);
            }
        }
        _ => {}
    }
}

fn check_triggers(root: &Mapping) -> Result<()> {
    let on = map_field(root, "on")?;
    let mut triggers: Vec<&str> = on.keys().filter_map(Value::as_str).collect();
    triggers.sort_unstable();
    if triggers != ["push", "workflow_dispatch"] || triggers.len() != on.len() {
        bail!("triggers must be exactly push (tags) and workflow_dispatch");
    }
    let push = map_field(on, "push")?;
    let tags = seq_field(push, "tags")?;
    if !tags.iter().any(|tag| tag.as_str() == Some("mitibar-v*")) {
        bail!("push trigger must include the mitibar-v* tag");
    }
    let dispatch = map_field(on, "workflow_dispatch")?;
    let inputs = map_field(dispatch, "inputs")?;
    let dry_run = map_field(inputs, "dry-run")?;
    if field(dry_run, "type")?.as_str() != Some("boolean") {
        bail!("dry-run input must be a boolean");
    }
    if field(dry_run, "default")?.as_bool() != Some(true) {
        bail!("dry-run input must default to true");
    }
    Ok(())
}

fn check_permissions(root: &Mapping) -> Result<()> {
    let permissions = map_field(root, "permissions")?;
    if permissions.len() != 1 || field(permissions, "contents")?.as_str() != Some("read") {
        bail!("top-level permissions must be exactly contents: read");
    }
    let jobs = map_field(root, "jobs")?;
    for (name, job) in jobs {
        let Some(name) = name.as_str() else {
            continue;
        };
        let job = map(job)?;
        let Some(permissions) = get(job, "permissions") else {
            continue;
        };
        let permissions = map(permissions)?;
        if name == "attest" {
            let expected = [
                ("contents", "read"),
                ("id-token", "write"),
                ("attestations", "write"),
            ];
            if permissions.len() != expected.len()
                || !expected.iter().all(|(key, value)| {
                    get(permissions, key).and_then(Value::as_str) == Some(*value)
                })
            {
                bail!(
                    "attest job permissions must be contents read, id-token write and attestations write"
                );
            }
        } else if get(permissions, "id-token").is_some()
            || get(permissions, "attestations")
                .is_some_and(|value| !(name == "verify" && value.as_str() == Some("read")))
        {
            // The verify job reads attestations to check them; nothing else may touch them.
            bail!("only the attest job may have id-token or attestations");
        }
        if name == "release"
            && (permissions.len() != 1
                || get(permissions, "contents").and_then(Value::as_str) != Some("write"))
        {
            bail!("release job permissions must be exactly contents: write");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn check_jobs(root: &Mapping) -> Result<()> {
    let jobs = map_field(root, "jobs")?;
    for name in [
        "preflight",
        "build",
        "sbom",
        "checksums",
        "attest",
        "verify",
        "release",
    ] {
        if get(jobs, name).is_none() {
            bail!("workflow has no `{name}` job");
        }
    }

    let preflight = job(jobs, "preflight")?;
    let preflight_text = run_text(preflight)?;
    for command in [
        "cargo xtask release-check mitibar",
        "cargo xtask data-status",
    ] {
        if !preflight_text.contains(command) {
            bail!("preflight does not run `{command}`");
        }
    }

    let build = job(jobs, "build")?;
    let matrix = map_field(map_field(build, "strategy")?, "matrix")?;
    let include = seq_field(matrix, "include")?;
    let mut rows = Vec::new();
    for item in include {
        let row = map(item)?;
        let os = field(row, "os")?.as_str().unwrap_or_default();
        let target = field(row, "target")?.as_str().unwrap_or_default();
        let arch = field(row, "arch")?.as_str().unwrap_or_default();
        rows.push((os, target, arch));
    }
    let expected = [
        ("ubuntu-latest", "x86_64-unknown-linux-gnu", "x64"),
        ("windows-latest", "x86_64-pc-windows-msvc", "x64"),
        ("windows-latest", "aarch64-pc-windows-msvc", "arm64"),
        ("macos-latest", "universal-apple-darwin", "universal"),
    ];
    for row in expected {
        if !rows.contains(&row) {
            bail!("build matrix is missing {} {} {}", row.0, row.1, row.2);
        }
    }
    if rows.len() != expected.len() {
        bail!("build matrix must have exactly four jobs");
    }
    let build_text = run_text(build)?;
    for command in [
        "cargo xtask package",
        "lipo",
        "cargo-packager --version 0.11.8",
    ] {
        if !build_text.contains(command) {
            bail!("build job does not run `{command}`");
        }
    }
    check_signing_gate(build, "Windows", "WINDOWS_SIGNING_ENABLED")?;
    check_signing_gate(build, "macOS", "MACOS_SIGNING_ENABLED")?;
    check_windows_signing(build)?;
    check_macos_signing(build)?;
    check_windows_arm64(build)?;

    let sbom_text = run_text(job(jobs, "sbom")?)?;
    if !sbom_text.contains("cargo cyclonedx") {
        bail!("sbom job does not run cargo cyclonedx");
    }
    // cargo-cyclonedx adds only ".json" to an override, and verify expects MitiBar-<v>.cdx.json.
    if !sbom_text.contains(r#"--override-filename "MitiBar-${VERSION}.cdx""#) {
        bail!("sbom job must name the SBOM MitiBar-${{VERSION}}.cdx (the tool appends .json)");
    }
    if !run_text(job(jobs, "checksums")?)?.contains("sha256sum") {
        bail!("checksums job does not run sha256sum");
    }
    let attest = job(jobs, "attest")?;
    let attests = steps(attest)?.iter().any(|step| {
        let Ok(step) = map(step) else {
            return false;
        };
        get(step, "uses")
            .and_then(Value::as_str)
            .is_some_and(|action| action.starts_with("actions/attest@"))
            && get(step, "with")
                .and_then(|with| map(with).ok())
                .is_some_and(|with| get(with, "subject-path").is_some())
    });
    if !attests {
        bail!("attest job does not attest the artifacts");
    }

    let verify = job(jobs, "verify")?;
    let verify_text = run_text(verify)?;
    for command in ["sha256sum -c SHA256SUMS", "gh attestation verify"] {
        if !verify_text.contains(command) {
            bail!("verify job does not run `{command}`");
        }
    }

    let release = job(jobs, "release")?;
    let release_needs = field(release, "needs")?;
    if !seq_field(release, "needs")
        .is_ok_and(|needs| needs.iter().any(|n| n.as_str() == Some("verify")))
    {
        bail!("release job must need verify, got {release_needs:?}");
    }
    if field(release, "environment")?.as_str() != Some("production") {
        bail!("release job must use the production environment");
    }
    let condition = field(release, "if")?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("release job has no condition"))?;
    if !condition.contains("github.ref_type == 'tag'")
        || !condition.contains("inputs.dry-run == false")
    {
        bail!("release job must require a tag and the dry-run switch");
    }
    if !run_text(release)?.contains("cargo xtask changelog mitibar") {
        bail!("release job does not build notes from the changelog");
    }
    let steps = steps(release)?;
    let draft = steps.iter().any(|step| {
        let Ok(step) = map(step) else {
            return false;
        };
        get(step, "uses")
            .and_then(Value::as_str)
            .is_some_and(|action| action.starts_with("softprops/action-gh-release@"))
            && get(step, "with")
                .and_then(|with| map(with).ok())
                .is_some_and(|with| get(with, "draft").and_then(Value::as_bool) == Some(true))
    });
    if !draft {
        bail!("release job must create a draft");
    }
    Ok(())
}

/// Scripts read values from `env`, never from an expression spliced into the shell text, so a
/// tag name or variable cannot inject a command.
fn check_no_expressions_in_run(root: &Mapping) -> Result<()> {
    let jobs = map_field(root, "jobs")?;
    for (name, job) in jobs {
        if run_text(map(job)?)?.contains("${{") {
            bail!(
                "job `{}` puts a ${{{{ }}}} expression inside run; pass it through env",
                name.as_str().unwrap_or("?")
            );
        }
    }
    Ok(())
}

/// The signing step must exist, mention its repository variable, and be gated on it being true.
fn check_signing_gate(job: &Mapping, os: &str, variable: &str) -> Result<()> {
    let found = steps(job)?.iter().any(|step| {
        let Ok(step) = map(step) else {
            return false;
        };
        let name = get(step, "name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let condition = get(step, "if").and_then(Value::as_str).unwrap_or_default();
        name.contains("Sign")
            && name.contains(os)
            && condition.contains(variable)
            && condition.contains("== 'true'")
    });
    if !found {
        bail!("no {os} signing step gated on vars.{variable} == 'true'");
    }
    Ok(())
}

/// the Windows binary is signed before `cargo xtask package`, and the NSIS setup is
/// signed after it, so the installer and the portable zip carry a signed `mitibar.exe`.
fn check_windows_signing(job: &Mapping) -> Result<()> {
    let sign_binary = step_named(job, "Sign Windows binary")?;
    let sign_binary_if = get(map(sign_binary)?, "if")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !sign_binary_if.contains("WINDOWS_SIGNING_ENABLED") || !sign_binary_if.contains("== 'true'")
    {
        bail!("the Windows binary signing step is not gated on WINDOWS_SIGNING_ENABLED");
    }
    let sign_binary_run = get(map(sign_binary)?, "run")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !sign_binary_run.contains("signtool sign")
        || !sign_binary_run.contains("$env:MITIBAR_BINARY")
    {
        bail!("the Windows binary step does not signtool the binary");
    }
    let binary_path = env_value(sign_binary, "MITIBAR_BINARY")?;
    if !binary_path.contains("release-widget/mitibar.exe") {
        bail!("the Windows binary step signs `{binary_path}`, not the release-widget binary");
    }

    let package = step_named(job, "Package (Windows)")?;
    let package_text = run_text(job)?;
    if !package_text.contains("cargo xtask package") {
        bail!("the Windows package step does not run cargo xtask package");
    }
    let binaries_dir = env_value(package, "MITIBAR_BINARIES_DIR")?;
    if !binaries_dir.contains("release-widget") {
        bail!("the Windows package step does not use the signed release-widget binary");
    }

    let setup = step_named(job, "Sign Windows installer")?;
    let setup_run = get(map(setup)?, "run")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !setup_run.contains("signtool sign") || !setup_run.contains("*-setup.exe") {
        bail!("the Windows installer step does not signtool the NSIS setup");
    }

    let build = step_position(job, "Build the Windows binary")?;
    let sign = step_position(job, "Sign Windows binary")?;
    let package = step_position(job, "Package (Windows)")?;
    let setup = step_position(job, "Sign Windows installer")?;
    if !(build < sign && sign < package && package < setup) {
        bail!("the Windows steps must build, sign the binary, package, then sign the installer");
    }
    Ok(())
}

/// the macOS `.app` is signed before the DMG is built, then the DMG is notarized and
/// stapled. The old step mounted a compressed DMG read-only and signed the app inside it.
fn check_macos_signing(job: &Mapping) -> Result<()> {
    let package = step_named(job, "Package (macOS app)")?;
    if env_value(package, "MITIBAR_PACKAGE_FORMATS")? != "app" {
        bail!("the macOS package step must ask for the app format only");
    }

    let sign = step_named(job, "Sign macOS app")?;
    let sign_run = get(map(sign)?, "run")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !sign_run.contains(r#"--sign "$MACOS_SIGNING_IDENTITY" dist/MitiBar.app"#) {
        bail!("the macOS step does not codesign dist/MitiBar.app before the DMG");
    }
    if sign_run.contains("dist/*.dmg") || sign_run.contains("MitiBar-*.dmg") {
        bail!("the macOS step signs the DMG instead of the .app inside it");
    }

    let dmg = step_named(job, "Build macOS DMG")?;
    let dmg_run = get(map(dmg)?, "run")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !dmg_run.contains("hdiutil create") || !dmg_run.contains("dist/MitiBar.app") {
        bail!("the macOS DMG step does not build the DMG around the signed .app");
    }

    let notarize = step_named(job, "Notarize and staple macOS DMG")?;
    let notarize_run = get(map(notarize)?, "run")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !notarize_run.contains("notarytool") || !notarize_run.contains("stapler staple") {
        bail!("the macOS step does not notarize and staple the DMG");
    }

    let package = step_position(job, "Package (macOS app)")?;
    let sign = step_position(job, "Sign macOS app")?;
    let dmg = step_position(job, "Build macOS DMG")?;
    let notarize = step_position(job, "Notarize and staple macOS DMG")?;
    if !(package < sign && sign < dmg && dmg < notarize) {
        bail!("the macOS steps must package the app, sign it, build the DMG, then notarize");
    }

    // The bare .app is a directory: sha256sum fails on it and the release must not carry it.
    let keep = step_named(job, "Keep only the DMG (macOS)")?;
    let keep_run = get(map(keep)?, "run")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !keep_run.contains("rm -rf dist/MitiBar.app") {
        bail!("the macOS build must remove dist/MitiBar.app after the DMG");
    }
    if step_position(job, "Keep only the DMG (macOS)")? < notarize {
        bail!("the macOS build removes dist/MitiBar.app before the DMG is notarized");
    }
    Ok(())
}

/// the Windows matrix builds `aarch64-pc-windows-msvc`, and the package step tells
/// cargo-packager and the portable zip which architecture it is.
fn check_windows_arm64(job: &Mapping) -> Result<()> {
    let build = step_named(job, "Build the Windows binary")?;
    let build_run = get(map(build)?, "run")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !build_run.contains("rustup target add") || !build_run.contains("--target") {
        bail!("the Windows build step does not add and build the matrix target");
    }
    let package = step_named(job, "Package (Windows)")?;
    if !env_value(package, "MITIBAR_TARGET")?.contains("matrix.target") {
        bail!("the Windows package step does not pass the matrix target to cargo-packager");
    }
    if !env_value(package, "MITIBAR_ARCH")?.contains("matrix.arch") {
        bail!("the Windows package step does not pass the matrix arch to the portable zip");
    }
    Ok(())
}

fn step_named<'a>(job: &'a Mapping, name: &str) -> Result<&'a Value> {
    for step in steps(job)? {
        let mapping = map(step)?;
        if get(mapping, "name").and_then(Value::as_str) == Some(name) {
            return Ok(step);
        }
    }
    bail!("workflow has no `{name}` step")
}

fn step_position(job: &Mapping, name: &str) -> Result<usize> {
    steps(job)?
        .iter()
        .position(|step| {
            map(step)
                .ok()
                .and_then(|step| get(step, "name"))
                .and_then(Value::as_str)
                == Some(name)
        })
        .ok_or_else(|| anyhow::anyhow!("workflow has no `{name}` step"))
}

fn env_value<'a>(step: &'a Value, key: &str) -> Result<&'a str> {
    let step = map(step)?;
    let env = map_field(step, "env")?;
    field(env, key)?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("`{key}` must be a string"))
}

fn job<'a>(jobs: &'a Mapping, name: &str) -> Result<&'a Mapping> {
    let job = get(jobs, name).ok_or_else(|| anyhow::anyhow!("workflow has no `{name}` job"))?;
    map(job)
}

fn steps(job: &Mapping) -> Result<Vec<&Value>> {
    Ok(seq_field(job, "steps")?.iter().collect())
}

/// All `run:` text in a job, joined, so a command split over lines is still found.
fn run_text(job: &Mapping) -> Result<String> {
    let mut out = String::new();
    for step in steps(job)? {
        if let Some(run) = get(map(step)?, "run").and_then(Value::as_str) {
            out.push_str(run);
            out.push('\n');
        }
    }
    Ok(out)
}

/// The value at string key `name`, if any.
fn get<'a>(value: &'a Mapping, name: &str) -> Option<&'a Value> {
    value.get(&Value::String(name.to_owned()))
}

fn map(value: &Value) -> Result<&Mapping> {
    value
        .as_hash()
        .ok_or_else(|| anyhow::anyhow!("expected a YAML mapping"))
}

fn field<'a>(value: &'a Mapping, name: &str) -> Result<&'a Value> {
    get(value, name).ok_or_else(|| anyhow::anyhow!("missing `{name}`"))
}

fn map_field<'a>(value: &'a Mapping, name: &str) -> Result<&'a Mapping> {
    map(field(value, name)?)
}

fn seq_field<'a>(value: &'a Mapping, name: &str) -> Result<&'a Vec<Value>> {
    field(value, name)?
        .as_vec()
        .ok_or_else(|| anyhow::anyhow!("`{name}` must be a list"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    fn workflow_text() -> String {
        // A Windows checkout may have CRLF line ends; the edits below are written with LF.
        std::fs::read_to_string(workspace_root().join(WORKFLOW))
            .expect("read the workflow")
            .replace("\r\n", "\n")
    }

    /// The real workflow with one exact edit; the edit must apply, so a later change to the
    /// test cannot silently turn a failed edit into a no-op.
    fn mutated(from: &str, to: &str) -> String {
        let text = workflow_text();
        assert_eq!(text.matches(from).count(), 1, "`{from}` must occur once");
        text.replacen(from, to, 1)
    }

    fn expect_failure(text: &str, needle: &str) {
        let error = check_text(text).expect_err("the mutated workflow must fail");
        let chain = format!("{error:#}");
        assert!(chain.contains(needle), "unexpected error: {chain}");
    }

    #[test]
    fn release_workflow_matches_the_runbook() {
        check(&workspace_root()).expect("release workflow");
    }

    #[test]
    fn an_unpinned_action_fails_the_negative_fixture() {
        let fixture = workspace_root().join("xtask/tests/fixtures/release-mitibar-unpinned.yml");
        let text = std::fs::read_to_string(&fixture).expect("read negative fixture");
        expect_failure(&text, "not pinned");
    }

    #[test]
    fn id_token_outside_the_attest_job_fails() {
        let text = mutated(
            "    permissions:\n      contents: write\n",
            "    permissions:\n      contents: write\n      id-token: write\n",
        );
        expect_failure(&text, "only the attest job");
    }

    #[test]
    fn dry_run_defaulting_to_false_fails() {
        let text = mutated("        default: true\n", "        default: false\n");
        expect_failure(&text, "dry-run input must default to true");
    }

    #[test]
    fn a_published_release_fails() {
        let text = mutated("          draft: true\n", "          draft: false\n");
        expect_failure(&text, "must create a draft");
    }

    #[test]
    fn a_release_without_the_production_environment_fails() {
        let text = mutated(
            "    environment: production\n",
            "    environment: staging\n",
        );
        expect_failure(&text, "production environment");
    }

    #[test]
    fn a_release_that_skips_verify_fails() {
        let text = mutated(
            "    needs: [preflight, checksums, attest, verify]\n",
            "    needs: [preflight, checksums, attest]\n",
        );
        expect_failure(&text, "release job must need verify");
    }

    #[test]
    fn a_verify_job_without_the_attestation_check_fails() {
        let text = mutated(
            "            gh attestation verify \"$file\" --repo \"$REPOSITORY\"\n",
            "            echo \"$file\"\n",
        );
        expect_failure(&text, "gh attestation verify");
    }

    #[test]
    fn attestations_write_outside_attest_fails() {
        let text = mutated("      attestations: read\n", "      attestations: write\n");
        expect_failure(&text, "only the attest job");
    }

    #[test]
    fn an_unsigned_windows_binary_fails() {
        let text = mutated(
            "      - name: Sign Windows binary\n",
            "      - name: Sign Windows binary missing\n",
        );
        expect_failure(&text, "Sign Windows binary");
    }

    #[test]
    fn a_macos_app_signed_inside_the_dmg_fails() {
        let text = mutated(
            r#"codesign --force --options runtime --sign "$MACOS_SIGNING_IDENTITY" dist/MitiBar.app"#,
            r#"codesign --force --options runtime --sign "$MACOS_SIGNING_IDENTITY" dist/MitiBar-*.dmg"#,
        );
        expect_failure(&text, "does not codesign dist/MitiBar.app");
    }

    /// Signing the binary after packaging would ship an unsigned binary inside a
    /// signed installer and an unsigned portable zip; the order check must catch it.
    #[test]
    fn signing_the_windows_binary_after_packaging_fails() {
        let text = workflow_text();
        let sign = text
            .find(
                "      - name: Sign Windows binary
",
            )
            .expect("sign step");
        let package = text
            .find(
                "      - name: Package (Windows)
",
            )
            .expect("package step");
        let linux = text
            .find(
                "      - name: Package (Linux)
",
            )
            .expect("linux step");
        assert!(
            sign < package && package < linux,
            "expected step order in the workflow"
        );
        let mut moved = String::new();
        moved.push_str(&text[..sign]);
        moved.push_str(&text[package..linux]);
        moved.push_str(&text[sign..package]);
        moved.push_str(&text[linux..]);
        expect_failure(&moved, "must build, sign the binary, package");
    }

    #[test]
    fn windows_arm64_missing_fails() {
        let text = mutated(
            "            target: aarch64-pc-windows-msvc\n",
            "            target: x86_64-pc-windows-msvc\n",
        );
        expect_failure(&text, "build matrix is missing");
    }

    #[test]
    fn an_ungated_signing_step_fails() {
        let text = mutated(
            "      - name: Sign Windows binary\n        if: matrix.os == 'windows-latest' && vars.WINDOWS_SIGNING_ENABLED == 'true'\n",
            "      - name: Sign Windows binary\n        if: matrix.os == 'windows-latest'\n",
        );
        expect_failure(&text, "WINDOWS_SIGNING_ENABLED");
    }

    #[test]
    fn a_pull_request_trigger_fails() {
        let text = mutated("on:\n  push:\n", "on:\n  pull_request_target:\n  push:\n");
        expect_failure(&text, "triggers must be exactly");
    }

    #[test]
    fn an_expression_inside_run_fails() {
        let text = mutated(
            r#"cargo xtask changelog mitibar "$VERSION" > notes.md"#,
            r#"cargo xtask changelog mitibar "${{ needs.preflight.outputs.version }}" > notes.md"#,
        );
        expect_failure(&text, "pass it through env");
    }

    #[test]
    fn comments_are_not_actions() {
        let text = r#"
name: release-mitibar
# uses: actions/checkout@v7 (this comment must not be read as an action)
on:
  push:
    tags: ["mitibar-v*"]
  workflow_dispatch:
    inputs:
      dry-run:
        type: boolean
        default: true
permissions:
  contents: read
jobs: {}
"#;
        let value = parse(text).expect("parse");
        let root = map(&value).expect("mapping");
        check_pinned_actions(&value).expect("the comment is not a `uses`");
        assert!(check_jobs(root).is_err(), "the fixture has no real jobs");
    }

    /// `renderer-check` (ADR-0018) must cover every target the release builds, with the universal
    /// macOS build as its two halves.
    #[test]
    fn renderer_check_covers_every_release_target() {
        let value = parse(&workflow_text()).expect("parse");
        let jobs = map_field(map(&value).expect("mapping"), "jobs").expect("jobs");
        let matrix = map_field(
            map_field(job(jobs, "build").expect("build"), "strategy").expect("strategy"),
            "matrix",
        )
        .expect("matrix");
        let mut targets: Vec<&str> = Vec::new();
        for item in seq_field(matrix, "include").expect("include") {
            let target = field(map(item).expect("row"), "target")
                .expect("target")
                .as_str()
                .expect("text");
            if target == "universal-apple-darwin" {
                targets.extend(["x86_64-apple-darwin", "aarch64-apple-darwin"]);
            } else {
                targets.push(target);
            }
        }
        targets.sort_unstable();
        let mut checked = crate::renderer::TARGETS.to_vec();
        checked.sort_unstable();
        assert_eq!(targets, checked);
    }
}
