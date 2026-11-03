# Runbook: release the `nepdate` and `nepdate-patro` crates

Owner: maintainer. Approver: a second maintainer. Decided in ADR-0012 and ADR-0021. One tag
`vX.Y.Z` releases both crates with the same version (ADR-0002).

## Before tagging

1. `main` is green in CI. `cargo xtask ci` and `cargo xtask coverage` pass locally.
2. `crates/nepdate/Cargo.toml` and `crates/nepdate-patro/Cargo.toml` have the new `version`,
   and the `nepdate` requirement in `crates/nepdate-patro/Cargo.toml` is `~X.Y.0`, or the exact
   `=X.Y.Z-pre.N` for a pre-release, because no tilde or caret requirement matches one. Both
   `CHANGELOG.md` files have a `## [X.Y.Z] - YYYY-MM-DD` section, moved out of `[Unreleased]`
   (a crate with no user-visible change says "No changes; released with nepdate X.Y.Z"). The
   release notes are built from those sections, so they are the text people will read.
3. `cargo xtask release-check nepdate X.Y.Z` passes. It is the same check the `preflight`
   job runs.
4. Once `nepdate` is on crates.io: `cargo semver-checks -p nepdate -p nepdate-patro` agrees with
   the bump. The `preflight` job runs it from then on and skips it before the first upload.
5. Open a pull request with the above, get approval, and merge.

## Releasing

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

That runs `release-lib.yml`:

| Job | What it does |
|---|---|
| `preflight` | tag, `Cargo.toml` version and changelog section agree. Seconds |
| `ci` | `cargo xtask ci` on the 3 OSes, then `cargo package -p nepdate -p nepdate-patro` |
| `publish` | waits for the `production` approval, exchanges the OIDC token from crates.io trusted publishing, runs `cargo publish -p nepdate`, which waits until the index serves it, then `cargo publish -p nepdate-patro`, and creates the GitHub Release with notes from both changelogs. A version with `-` becomes a GitHub pre-release |
| `verify-published` | calls `verify-published.yml`: on Linux, macOS and Windows, `scripts/verify-published.sh` builds a new project against `=X.Y.Z` of both crates from crates.io, runs the binary and compares its output and the resolved versions |

`verify-published.yml` can also be started by hand with a version, to re-check a release with the
script on `main`.

The same workflow runs on a schedule. Every Monday one Linux runner installs the newest version
from crates.io (`latest`, resolved the way `cargo add` resolves it) and checks it; every day a
metadata-only run confirms that crates.io still lists the version and serves the crate file, with
no download. A failed scheduled run opens one issue labelled `published-check`, titled
`Published nepdate <version> does not install or run`, and comments on it only when the failing
output changed. Create that label once on the repository; nothing else is needed. A green
scheduled run says nothing and opens nothing.

If `cargo publish -p nepdate-patro` fails after `nepdate` went out, fix the cause and rerun the
`publish` job; the workflow skips a crate whose version is already on crates.io. Never publish
`nepdate-patro` without the matching `nepdate`.

Approve at the run's page: `publish` → *Review deployments*.

### Trusted publishing on crates.io

crates.io attaches a trusted publisher only to a crate that exists. The 0.1.0 releases used a
short-lived `publish-new` token for that reason. Both crates now exist, the trusted publisher is
registered for each (owner `sanamhub`, repository `nepdate-rust`, workflow `release-lib.yml`,
environment `production`), and the `CARGO_REGISTRY_TOKEN` secret is deleted. The workflow has no
token fallback.

If a new crate is ever added to this repository, create a `publish-new` token for it, store it as
`CARGO_REGISTRY_TOKEN` in the `production` environment, publish once, then register the trusted
publisher and delete the secret again.

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
