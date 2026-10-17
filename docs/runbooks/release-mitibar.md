# Runbook: release MitiBar

Skeleton, completed in task W4-06. Decided in ADR-0012.

## Before tagging

1. CI green on all 3 OSes. `docs/runbooks/widget-manual-test.md` completed on Windows, macOS
   and Linux for this version (attach the ticked copy to the release PR).
2. Version bump in `apps/mitibar/Cargo.toml`, a `## [X.Y.Z] - YYYY-MM-DD` section in
   `apps/mitibar/CHANGELOG.md`, and `cargo xtask release-check mitibar X.Y.Z`
   passes.
3. `cargo xtask data-status` passes: tithi and holiday data reach at least 183 days ahead.
   If not, follow the hub's [data-update runbook](https://github.com/sanamhub/nepdate/blob/main/docs/runbooks/data-update.md) first.
4. Performance numbers updated in `docs/benchmarks-widget.md` (ADR-0008 §C).

## Releasing

```bash
git tag mitibar-vX.Y.Z
git push origin mitibar-vX.Y.Z
```

`release-mitibar.yml` runs `preflight`, builds and packages on the 3 OSes (signing only
where the signing variable is set), then waits for the `production` approval and creates a
**draft** GitHub Release with the changelog section, `SHA256SUMS`, the SBOM and
attestations.

The approver downloads each artifact, checks it against `SHA256SUMS`
(`gh attestation verify <file> --repo sanamhub/nepdate-rust` also works), installs it on a clean
VM or user account per OS, then publishes the draft.

## After releasing

On each OS: install → the tray shows today's BS date (compare with
`cargo run -p nepdate --features jiff --example convert -- --today`) → open the popup → quit → uninstall
leaves no autostart entry. Then add an empty `## [Unreleased]` to the changelog.

## Rolling back

Edit the release to *pre-release* (removes it from "latest"), re-mark the previous release
as latest, add a note to the broken release. Fix forward with a patch version.
