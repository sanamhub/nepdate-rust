# L0: Workspace, toolchain, lints, CI

**Goal:** an empty-but-real Cargo workspace where `cargo xtask ci` runs every quality gate,
locally and in GitHub Actions, on Windows, macOS and Linux.

Read first: `AGENTS.md`, ADR-0002, ADR-0005, ADR-0006, ADR-0014.
Human reads: `docs/learn/00-setup-and-workflow.md`, `docs/learn/01-cargo-workspace.md`.

## Tasks

### L0-01 Root workspace files

Create:

`rust-toolchain.toml`
```toml
[toolchain]
channel = "1.98"            # dev toolchain; bump deliberately (see docs/runbooks/toolchain-bump.md)
components = ["rustfmt", "clippy"]
targets = ["wasm32-unknown-unknown"]
```

`Cargo.toml`
```toml
[workspace]
resolver = "3"
members = ["crates/nepdate", "xtask"]          # crates/nepdate-patro is added in L5-00, apps/nepdate-widget in W0
default-members = ["crates/nepdate"]

[workspace.package]
edition = "2024"
license = "MIT"                                # ADR-0013
repository = "https://github.com/sanamhub/nepdate-rust"   # ADR-0002
rust-version = "1.85"                          # library MSRV (edition 2024 minimum)

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
unreachable_pub = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"                   # we add #[must_use] explicitly per ADR-0007
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
indexing_slicing = "warn"                      # forces .get() or proven-safe indexing with a comment

[profile.release]
lto = "fat"
codegen-units = 1
strip = true
```

`.cargo/config.toml`
```toml
[alias]
xtask = "run --package xtask --"
```

Also: `rustfmt.toml` (`max_width = 100`), `.editorconfig`, and `LICENSE` (MIT text,
`Copyright (c) nepdate contributors`, ADR-0013).
`.gitattributes`:
```
* text=auto eol=lf
*.png binary
*.ttf binary
*.ico binary
*.icns binary
```
Without it, a Windows checkout gets CRLF line endings and `cargo xtask codegen --check` reports
drift that isn't there.
`.gitignore` already exists; extend it, don't replace it.
Create `crates/nepdate/CHANGELOG.md` with an empty `## [Unreleased]` (ADR-0002).
`shared/` already exists (ADR-0003). Before the first commit, the human replaces the copy with
the hub's first pushed commit and writes that SHA to `shared/VERSION` (it says `pending` now).

AC:
- `cargo metadata` succeeds. `cargo build` builds the (empty) library.
- `edition = "2024"` and `resolver = "3"` are present.

### L0-02 Library crate skeleton

`crates/nepdate/Cargo.toml`
```toml
[package]
name = "nepdate"            # check crates.io availability first; if taken, STOP and ask the human
version = "0.0.0"
description = "Bikram Sambat (Nepali) calendar: fast, no_std, zero-dependency BS↔AD conversion"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
categories = ["date-and-time", "no-std", "localization"]
keywords = ["nepali", "bikram-sambat", "calendar", "date", "nepal"]

[features]
default = ["std"]
std = []

[lints]
workspace = true
```

`crates/nepdate/src/lib.rs`
```rust
//! Bikram Sambat (Nepali) calendar. See `shared/spec/` in the repository for the algorithm.
#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "std")]
extern crate std;
```

AC: `cargo build -p nepdate --no-default-features` and
`cargo build -p nepdate --no-default-features --target wasm32-unknown-unknown` succeed.

### L0-03 `xtask` crate with `ci` command

`xtask/Cargo.toml`: `publish = false`, deps: `anyhow`, `serde_json` (codegen comes in L1).
`xtask/src/main.rs`: subcommands parsed from `std::env::args()` (no clap; keep it tiny):

| Command | Does |
|---|---|
| `cargo xtask ci` | runs, in order, stopping at first failure: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo nextest run --workspace --all-features --no-tests=pass` (ADR-0014; `--no-tests=pass` because L0 has no tests yet); `cargo test --doc --workspace --all-features`; `cargo doc --workspace --no-deps --all-features` with env `RUSTDOCFLAGS=-D warnings`; `cargo build -p nepdate --no-default-features`; `cargo build -p nepdate --no-default-features --target wasm32-unknown-unknown`; `cargo xtask codegen --check` (no-op until L1); `cargo deny check` (skip with a warning if not installed) |
| `cargo xtask codegen [--check]` | placeholder that prints "nothing to generate yet" (L1 fills it) |

Implementation hint: a helper `fn run(cmd: &str, args: &[&str]) -> anyhow::Result<()>` using
`std::process::Command`, printing the command before running it.

AC (PORT-PLAN phase 0):
- `cargo xtask ci` exits 0 on a clean checkout.
- It exits non-zero if you add an unused `let x = 1;` (clippy fails), and non-zero if you
  indent one line of `crates/nepdate/src/lib.rs` wrongly (`cargo fmt --check` fails). Revert both.
- The last line it prints is one summary line, e.g. `xtask ci: all 9 steps passed` (the count grows as later phases add steps) or
  `xtask ci: FAILED at step 3 (cargo clippy ...)`.

### L0-04 `deny.toml`

Licenses allowed: `MIT`, `Apache-2.0`, `Unicode-3.0`, `BSD-2-Clause`, `BSD-3-Clause`,
`ISC`, `Zlib`, `OFL-1.1` (fonts, widget). Advisories: deny. Bans: warn on duplicates.
Sources: crates.io only. (Slint's license exception is added in W0.)

AC: `cargo deny check` passes.

### L0-05 GitHub Actions CI

`.github/workflows/ci.yml`: on `push` to main and `pull_request`.
- Job `ci`: matrix `os: [ubuntu-latest, windows-latest, macos-latest]`. Steps:
  checkout → `dtolnay/rust-toolchain` (reads `rust-toolchain.toml`) →
  `Swatinem/rust-cache@v2` → `taiki-e/install-action` with `cargo-deny,cargo-nextest` →
  `cargo xtask ci`.
- Job `msrv` (ubuntu): `dtolnay/rust-toolchain@1.85` → `cargo build -p nepdate --all-features`.
- `shared-check` is added in L6-06, once the hub's reusable workflow exists.
- Supply chain: pin every third-party action to a **full commit SHA** with the version in a
  comment (`uses: dtolnay/rust-toolchain@<sha> # v1`). Top-level `permissions: contents: read`.
  `concurrency: { group: ci-${{ github.ref }}, cancel-in-progress: true }`.

AC: workflow YAML passes `actionlint` if available (optional); CI green on the PR.

### L0-06 Repository hygiene (small files, no code)
- `.github/dependabot.yml`: ecosystems `cargo` and `github-actions`, weekly, grouped minor+patch
  updates into one PR.
- `.github/pull_request_template.md`: sections *Task IDs*, *What changed and why*, *How tested*
  (paste the `cargo xtask ci` summary), *What to look at first*, *Rust concepts used*
  (links to `docs/learn/`), and a short checklist: tests added, changelog updated,
  `docs/API.md` unchanged or updated, ADR added for a significant decision.
- `SECURITY.md`: supported versions (latest release of each package), report privately via
  GitHub security advisories, not a public issue, first reply within a week.
- `CONTRIBUTING.md`: open an issue before a new feature or API change; setup (`rust-toolchain.toml`,
  `py` for `shared/spec/`); `cargo xtask ci` before a PR; one change per PR; tests for behaviour
  changes; changelog line for anything a user would notice; ADR for a significant decision;
  Conventional Commits; writing rules in `.claude/skills/writing-style/SKILL.md`.
- `CODE_OF_CONDUCT.md`: Contributor Covenant 2.1, contact = the maintainer's GitHub profile.
- `.github/ISSUE_TEMPLATE/`: `bug_report.yml` (reproducing input, expected vs actual, package
  and version, OS), `feature_request.yml` (the problem, not the solution; notes the ADR-0008
  scope list), `config.yml` (`blank_issues_enabled: false`, contact link to private security
  advisories).
- Dependabot: `open-pull-requests-limit: 5` per ecosystem.
- **Human task (not the agent):** create the GitHub repository `sanamhub/nepdate-rust`; enable
  branch protection on `main` (require PR, 1 approval, CI green, no force-push, org §10);
  enable the dependency graph and private vulnerability reporting; create the Environment
  `production` with yourself as required reviewer.

### L0-07 README status

Update root `README.md` "Status" section: "L0 done". Nothing else.

## Definition of Done
Global DoD (`00-ROADMAP.md`) + CI green on all 3 OSes.

## Human review checklist
- [ ] Open `Cargo.toml`: can you explain what `[workspace.lints]` does? (learn/01 §3)
- [ ] Run `cargo xtask ci` yourself. Then break formatting on purpose and run it again.
- [ ] Confirm the crate names `nepdate` and `nepdate-patro` are free on crates.io, or pick new ones (and update docs).

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes locally and CI is green on ubuntu, windows and macos plus the MSRV job.
- [ ] Workspace members are `crates/nepdate` and `xtask` (`crates/nepdate-patro`, lib
      `nepdate_patro`, arrives in L5-00); nothing is named `events` or `meta` (hub ADR-0004).
- [ ] `[workspace.lints]` has `unsafe_code = "forbid"`, and `crates/nepdate/src/lib.rs` starts
      with `#![no_std]` and `#![forbid(unsafe_code)]` as shown in L0-02.
- [ ] The release profile has `lto = "fat"` and `codegen-units = 1` (ADR-0004 measured with them).
- [ ] `deny.toml` passes `cargo deny check`; every action in `.github/workflows/` is pinned by SHA.
- [ ] `cargo build -p nepdate --no-default-features --target wasm32-unknown-unknown` runs in `ci`.
- [ ] Nothing under `shared/` changed.
