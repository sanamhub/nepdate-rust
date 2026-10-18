//! Static check of `.github/workflows/release-mitibar.yml` (W4-04). The workflow cannot run
//! until the repository has a remote, so this test parses the YAML and checks the structure the
//! plan requires: triggers, pinned actions, least privilege, signing gates, checksums, SBOM and
//! a draft release. Comments are ignored by the parser, not by string matching.

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_yaml_ng::{Mapping, Value};

use crate::codegen::workspace_root;

const WORKFLOW: &str = ".github/workflows/release-mitibar.yml";

pub(crate) fn check(root: &Path) -> Result<()> {
    let path = root.join(WORKFLOW);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let value: Value = serde_yaml_ng::from_str(&text)
        .with_context(|| format!("parse {} as YAML", path.display()))?;
    check_value(&value)
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
        Value::Mapping(map) => {
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
        Value::Sequence(items) => {
            for item in items {
                collect_uses(item, out);
            }
        }
        _ => {}
    }
}

fn check_triggers(root: &Mapping) -> Result<()> {
    let on = map_field(root, "on")?;
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
        let Some(permissions) = job.get("permissions") else {
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
                    permissions.get(*key).and_then(Value::as_str) == Some(*value)
                })
            {
                bail!(
                    "attest job permissions must be contents read, id-token write and attestations write"
                );
            }
        } else if permissions.contains_key("id-token") || permissions.contains_key("attestations") {
            bail!("only the attest job may have id-token or attestations");
        }
        if name == "release"
            && (permissions.len() != 1
                || permissions.get("contents").and_then(Value::as_str) != Some("write"))
        {
            bail!("release job permissions must be exactly contents: write");
        }
    }
    Ok(())
}

fn check_jobs(root: &Mapping) -> Result<()> {
    let jobs = map_field(root, "jobs")?;
    for name in [
        "preflight",
        "build",
        "sbom",
        "checksums",
        "attest",
        "release",
    ] {
        if !jobs.contains_key(name) {
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
    let os = seq_field(matrix, "os")?;
    let names: Vec<&str> = os.iter().filter_map(Value::as_str).collect();
    if names.len() != 3
        || !names.contains(&"ubuntu-latest")
        || !names.contains(&"windows-latest")
        || !names.contains(&"macos-latest")
    {
        bail!("build matrix must be exactly the three OS runners");
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

    if !run_text(job(jobs, "sbom")?)?.contains("cargo cyclonedx") {
        bail!("sbom job does not run cargo cyclonedx");
    }
    if !run_text(job(jobs, "checksums")?)?.contains("sha256sum") {
        bail!("checksums job does not run sha256sum");
    }
    let attest = job(jobs, "attest")?;
    let attests = steps(attest)?.iter().any(|step| {
        let Ok(step) = map(step) else {
            return false;
        };
        step.get("uses")
            .and_then(Value::as_str)
            .is_some_and(|action| action.starts_with("actions/attest@"))
            && step
                .get("with")
                .and_then(|with| map(with).ok())
                .is_some_and(|with| with.contains_key("subject-path"))
    });
    if !attests {
        bail!("attest job does not attest the artifacts");
    }

    let release = job(jobs, "release")?;
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
        step.get("uses")
            .and_then(Value::as_str)
            .is_some_and(|action| action.starts_with("softprops/action-gh-release@"))
            && step
                .get("with")
                .and_then(|with| map(with).ok())
                .is_some_and(|with| with.get("draft").and_then(Value::as_bool) == Some(true))
    });
    if !draft {
        bail!("release job must create a draft");
    }
    Ok(())
}

/// The signing step must exist, mention its repository variable, and be gated on it being true.
fn check_signing_gate(job: &Mapping, os: &str, variable: &str) -> Result<()> {
    let found = steps(job)?.iter().any(|step| {
        let Ok(step) = map(step) else {
            return false;
        };
        let name = step.get("name").and_then(Value::as_str).unwrap_or_default();
        let condition = step.get("if").and_then(Value::as_str).unwrap_or_default();
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

fn job<'a>(jobs: &'a Mapping, name: &str) -> Result<&'a Mapping> {
    let job = jobs
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("workflow has no `{name}` job"))?;
    map(job)
}

fn steps(job: &Mapping) -> Result<Vec<&Value>> {
    Ok(seq_field(job, "steps")?.iter().collect())
}

/// All `run:` text in a job, joined, so a command split over lines is still found.
fn run_text(job: &Mapping) -> Result<String> {
    let mut out = String::new();
    for step in steps(job)? {
        if let Some(run) = map(step)?.get("run").and_then(Value::as_str) {
            out.push_str(run);
            out.push('\n');
        }
    }
    Ok(out)
}

fn map(value: &Value) -> Result<&Mapping> {
    value
        .as_mapping()
        .ok_or_else(|| anyhow::anyhow!("expected a YAML mapping"))
}

fn field<'a>(value: &'a Mapping, name: &str) -> Result<&'a Value> {
    value
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("missing `{name}`"))
}

fn map_field<'a>(value: &'a Mapping, name: &str) -> Result<&'a Mapping> {
    map(field(value, name)?)
}

fn seq_field<'a>(value: &'a Mapping, name: &str) -> Result<&'a Vec<Value>> {
    field(value, name)?
        .as_sequence()
        .ok_or_else(|| anyhow::anyhow!("`{name}` must be a list"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn release_workflow_matches_the_plan() {
        check(&workspace_root()).expect("release workflow");
    }

    #[test]
    fn an_unpinned_action_fails_the_negative_fixture() {
        let fixture = workspace_root().join("xtask/tests/fixtures/release-mitibar-unpinned.yml");
        let text = std::fs::read_to_string(&fixture).expect("read negative fixture");
        let value: Value = serde_yaml_ng::from_str(&text).expect("parse negative fixture");
        let error = check_value(&value).expect_err("an unpinned action must fail");
        assert!(
            error.to_string().contains("not pinned"),
            "unexpected error: {error}"
        );
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
        let value: Value = serde_yaml_ng::from_str(text).expect("parse");
        let root = map(&value).expect("mapping");
        check_pinned_actions(&value).expect("the comment is not a `uses`");
        assert!(check_jobs(root).is_err(), "the fixture has no real jobs");
    }
}
