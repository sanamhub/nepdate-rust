# W4: Packaging, performance verification, release MitiBar 0.1.0

Read first: ADR-0012, ADR-0008 §C (targets). Human reads: `docs/learn/12-shipping.md`.

## Tasks

### W4-00 Data freshness gate
The `preflight` job of the widget release runs `cargo xtask data-status` (L5-01b), which applies
the hub's rule (hub task H1-02): it fails if public-holiday data ends less than 183 days after the
day the job runs, and prints a `WARNING` (copied into the release PR) if tithi or events do. Fix a
failure by getting the data into the hub and merging the `shared-sync` PR
([data-update runbook](https://github.com/sanamhub/nepdate/blob/main/docs/runbooks/data-update.md)),
not by lowering the threshold. With today's data, holidays end BS 2084-08-18 (AD 2027-12-03), so the
gate fails from 2027-06-04; tithi ends BS 2083-12-30 (AD 2027-04-13), so the warning already shows.

AC: `cargo xtask data-status --today 2026-10-20` exits 0 with a tithi warning;
`--today 2027-06-04` exits 1.

### W4-01 Release profile for the widget
In the workspace `Cargo.toml`, add `[profile.release-widget]` inheriting `release`, with
`opt-level = "s"` and `panic = "abort"` (the library keeps unwinding for users). Build with
`cargo build -p mitibar --profile release-widget`.

### W4-02 Icons and app metadata
App icon (`assets/icon.svg` → PNG 16…1024, `.ico`, `.icns`).
A simple calendar page with "सं" is fine. Generate the sizes with a script in `xtask`
(`cargo xtask icons`) using `resvg` (xtask-only dependency).

### W4-03 `cargo-packager` config
`apps/mitibar/Packager.toml`: product name "MitiBar", identifier
`sanamhub.MitiBar`, formats per ADR-0012, `before-packaging-command` = the profile build,
license file, and macOS `LSUIElement = true` (menu-bar app, no Dock icon).
`cargo xtask package` wraps it.

### W4-04 Release workflow
`.github/workflows/release-mitibar.yml` on tag `mitibar-v*`, and `workflow_dispatch` with `dry-run`
(ADR-0012):
1. `preflight`: `cargo xtask release-check mitibar <version>` and `cargo xtask data-status`.
2. Matrix build + package on the 3 OSes (macOS builds both arches and runs `lipo`).
3. Signing steps that run only when `vars.WINDOWS_SIGNING_ENABLED` / `vars.MACOS_SIGNING_ENABLED`
   is `true`.
4. `SHA256SUMS`, a CycloneDX SBOM (`cargo cyclonedx -p mitibar`, installed with
   `taiki-e/install-action`), then build-provenance attestations (`actions/attest`, which
   supersedes `attest-build-provenance`; needs `id-token: write` + `attestations: write` on that
   job only).
5. `environment: production`, then a **draft** GitHub Release with notes from
   `cargo xtask changelog mitibar <version>`. A human publishes it after the manual test.

### W4-05 Performance verification (ADR-0008 §C)
`docs/benchmarks-widget.md`: binary size per OS; idle RSS after 5 min with the popup closed;
cold start time to the tray icon (log timestamp from `main` start to tray-ready); idle CPU.
If any target is missed, report it to the human. Do not add features to "fix" it.

### W4-06 Manual test run + runbooks
- Complete `docs/runbooks/widget-manual-test.md` on all 3 OSes (the human or a tester ticks it).
- Write `docs/runbooks/release-mitibar.md`: build, verify checksums, install/uninstall on each
  OS, post-release smoke test (the tray shows today's BS date correctly), rollback steps.

### W4-07 Version 0.1.0
Bump, CHANGELOG, then **stop**. The human tags and publishes.

## Definition of Done
Global DoD + milestone M5.

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes; the data freshness gate (W4-00) passes.
- [ ] Installers build for all 3 OSes in the release workflow dry run, with checksums.
- [ ] W4-05 performance numbers are in the PR and meet ADR-0008 §C, or the miss is reported.
- [ ] `docs/runbooks/widget-manual-test.md` is ticked on all 3 OSes and
      `docs/runbooks/release-mitibar.md` has rollback steps.
- [ ] Version and changelog are 0.1.0; nothing was tagged or published (the agent stopped).
