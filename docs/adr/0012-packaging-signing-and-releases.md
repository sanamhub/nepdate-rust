# ADR-0012: Packaging, signing and releases

- Status: Accepted
- Date: 2026-09-24 (revised 2026-09-25: trusted publishing, preflight, notes from changelog;
  revised 2026-10-07: `nepdate-patro` released with `nepdate`, hub ADR-0004)
- Deciders: Sanam Pakuwal

## Context

Two release trains: the library crates `nepdate` and `nepdate-patro` (crates.io, released
together, ADR-0002) and the `nepdate-widget` desktop app (Windows, macOS, Linux). Org policy: semver + changelog, green CI, approval-gated
production release, rollback plan, post-release verification (SEA §9, §12). The release
flow follows the one proven in `sanamhub/ada-csharp`.

## Decision

### Common to both packages

- Tag with the package prefix (`nepdate-vX.Y.Z`, `widget-vX.Y.Z`). A tag containing `-`
  (`nepdate-v0.2.0-beta.1`) is a prerelease.
- **Preflight job first**, seconds long: `cargo xtask release-check <package> <version>`
  fails unless the tag, the manifest version and a `## [X.Y.Z]` section in that package's
  `CHANGELOG.md` all agree. For `nepdate` it checks both library crates (L6-04). A wrong tag
  costs seconds, not a full build.
- **Approval gate:** the publishing job runs in GitHub Environment `production` with a
  required reviewer who is not the tagger (org §10).
- **Release notes come from the changelog section** (`cargo xtask changelog <package>
  <version>`), plus an *Install* and a *Verification* block. Never `generate_release_notes`,
  which lists commits instead of what shipped.
- The workflow creates the GitHub Release. Nobody creates one by hand.
- `workflow_dispatch` with `dry-run: true` (default) runs everything except publishing.
- A workflow step that must trigger another workflow calls it (`workflow_call`). A release
  created with `GITHUB_TOKEN` does not fire `on: release`.

### Libraries (`nepdate` and `nepdate-patro`)

- One tag `nepdate-vX.Y.Z` publishes both crates with the same version X.Y.Z, in this order:
  `cargo publish -p nepdate`, then `cargo publish -p nepdate-patro` (cargo waits until the
  first is in the index before the second resolves it). If the second publish fails, rerun
  only that step; the first can't be published twice.
- Published by **crates.io trusted publishing**: `rust-lang/crates-io-auth-action` swaps
  a GitHub OIDC token for a 30-minute crates.io token. The job needs `id-token: write`.
  No long-lived registry token exists anywhere.
- crates.io allows trusted publishing only for a crate that already exists. So **0.1.0 of
  both crates is published once by hand** (runbook), after the dry run passes on the tag.
  Then the human registers the trusted publisher (repository, `release-lib.yml`, environment
  `production`) on each crate and deletes the token.
- Before publishing: `cargo package -p nepdate -p nepdate-patro` builds each crate from its
  packaged files, which catches a file missing from `include`. From 0.1.1
  on, `cargo semver-checks` must agree with the version bump for both crates.
- After publishing: a job installs `nepdate@X.Y.Z` and `nepdate-patro@X.Y.Z` from crates.io
  into a new project and runs both README examples (retries for up to 10 minutes while the
  index updates).
- Rollback = `cargo yank` of both crates at that version + patch release of both (a published
  crate can't be deleted).

### Widget

- Bundled with **`cargo-packager`** (standalone Tauri bundler):

  | OS | Formats | Arch |
  |---|---|---|
  | Windows | NSIS `.exe` installer (per-user, no admin) + portable `.zip` | x86_64, aarch64 |
  | macOS | `.dmg` containing `.app` | universal (aarch64 + x86_64 via `lipo`) |
  | Linux | `.AppImage`, `.deb` | x86_64 |

- Risk: cargo-packager's last release was 0.11.8 (2025-11) and the repo is only lightly
  maintained. Fallback if it breaks: `cargo-bundle` (≥ 0.12, no NSIS/DMG, so Windows would
  ship MSI + zip and macOS a zipped `.app`). Switching requires a new ADR.
- Release profile: `opt-level = "s"`, `lto = "fat"`, `codegen-units = 1`,
  `panic = "abort"`, `strip = true`.
- **Signing**: Windows Authenticode (Azure Artifact Signing) and macOS Developer ID +
  notarization are *required for 1.0*, *optional for 0.x*. Each signing step runs only when
  its repository variable is set (`WINDOWS_SIGNING_ENABLED`, `MACOS_SIGNING_ENABLED`), and is
  skipped rather than faked otherwise. Until signed, the release notes say that SmartScreen /
  Gatekeeper warnings will appear.
- The release is a **draft** until the approver has installed each artifact (manual test
  runbook), because a desktop app can't be verified by a script alone.
- Attached to every widget release: `SHA256SUMS`, a CycloneDX SBOM (`cargo cyclonedx`), and
  build-provenance attestations (`actions/attest`).
- **No auto-update in v0.1** (ADR-0008 scope). Users download new versions from GitHub
  Releases.
- Rollback = mark the GitHub Release as pre-release, re-mark the previous one as latest
  (runbook `docs/runbooks/release-widget.md`).

## Consequences

- No registry secret to leak or rotate after 0.1.0.
- One tool for all three OSes' installers; no WiX/pkgbuild scripts to maintain.
- Unsigned 0.x builds trigger OS warnings. Acceptable for previews, not for 1.0.
