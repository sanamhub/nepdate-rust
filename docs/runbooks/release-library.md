# Runbook: release the `nepdate` and `nepdate-patro` crates

Owner: maintainer. Approver: a second person (org §10/§12). Decided in ADR-0012. One tag
`nepdate-vX.Y.Z` releases both crates with the same version (ADR-0002).

## Before tagging

1. `main` is green in CI. `cargo xtask ci` and `cargo xtask coverage` pass locally.
2. `crates/nepdate/Cargo.toml` and `crates/nepdate-patro/Cargo.toml` have the new `version`,
   and the `nepdate` requirement in `crates/nepdate-patro/Cargo.toml` is `~X.Y.0`. Both
   `CHANGELOG.md` files have a `## [X.Y.Z] - YYYY-MM-DD` section, moved out of `[Unreleased]`
   (a crate with no user-visible change says "No changes; released with nepdate X.Y.Z"). The
   release notes are built from those sections, so they are the text people will read.
3. `cargo xtask release-check nepdate X.Y.Z` passes. It is the same check the `preflight`
   job runs.
4. From 0.1.1 on: `cargo semver-checks -p nepdate -p nepdate-patro` agrees with the bump.
5. PR with the above → reviewed → merged.

## Releasing

```bash
git tag nepdate-vX.Y.Z
git push origin nepdate-vX.Y.Z
```

That runs `release-lib.yml`:

| Job | What it does |
|---|---|
| `preflight` | tag, `Cargo.toml` version and changelog section agree. Seconds |
| `ci` | `cargo xtask ci` on the 3 OSes, then `cargo package -p nepdate -p nepdate-patro` |
| `publish` | waits for the `production` approval, gets a token by trusted publishing, `cargo publish -p nepdate`, then `cargo publish -p nepdate-patro`, creates the GitHub Release with notes from both changelogs |
| `verify-published` | installs `nepdate@X.Y.Z` and `nepdate-patro@X.Y.Z` from crates.io into a new project and runs both README examples |

If `cargo publish -p nepdate-patro` fails after `nepdate` went out, fix the cause and rerun the
`publish` job; the workflow skips a crate whose version is already on crates.io. Never publish
`nepdate-patro` without the matching `nepdate`.

Approve at the run's page: `publish` → *Review deployments*.

### First release only (0.1.0)

crates.io allows trusted publishing only for a crate that already exists.

1. Run `release-lib.yml` by `workflow_dispatch` on the tag with `dry-run: true`. It must pass.
2. On crates.io, create an API token: scope `publish-new`, crates `nepdate` and
   `nepdate-patro`, expiry 1 day.
3. From a clean checkout of the tag: `cargo login`, enter the token when it asks (never paste
   it anywhere else), then `cargo publish -p nepdate`, then `cargo publish -p nepdate-patro`.
4. On crates.io, for **each** crate → Settings → Trusted Publishing, add: owner `sanamhub`,
   repository `nepdate-rust`, workflow `release-lib.yml`, environment `production`.
5. Revoke the token. Create the GitHub Release once by hand:
   `cargo xtask changelog nepdate 0.1.0 > notes.md && gh release create nepdate-v0.1.0 --notes-file notes.md`.
   Every later release is created by the workflow.

## After releasing

1. Read the release page. Notes match the changelog.
2. docs.rs builds are green: https://docs.rs/nepdate/X.Y.Z and
   https://docs.rs/nepdate-patro/X.Y.Z.
3. Open the next cycle: add an empty `## [Unreleased]` to both changelogs. Nothing does this
   for you.

## Rolling back

A published crate can't be deleted. Yank both crates at that version, then release a fix:

```bash
cargo yank nepdate --version X.Y.Z
cargo yank nepdate-patro --version X.Y.Z
```

Yanking doesn't break existing lockfiles. It stops new ones from picking that version.
Note the yank in both changelogs and release `X.Y.(Z+1)` of both.
