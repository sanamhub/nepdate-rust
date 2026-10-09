# L6: Docs, benchmarks, coverage, release automation, `nepdate` and `nepdate-patro` 0.1.0

**Goal:** two crates that someone who has never seen this repo can use from docs.rs alone,
a repeatable, approval-gated release of both from one tag, and the weekly sync of `shared/`
from the hub. This is PORT-PLAN phase 5.

Read first: ADR-0003, ADR-0012, ADR-0014. Human reads: `docs/learn/07-docs-benches-release.md`.

## Tasks

### L6-01 Crate docs
- `crates/nepdate/README.md` (used as crate-level docs via `#![doc = include_str!("../README.md")]`):
  quick start, feature table, the C#→Rust mapping table from ADR-0007, range, data
  source + MIT notice, "not affiliated with the Government of Nepal", MSRV. Links are
  absolute (`https://github.com/sanamhub/nepdate-rust/...`), because the README is shown on
  crates.io and docs.rs, where relative links break.
- `crates/nepdate-patro/README.md` (crate docs the same way): quick start with
  `nepdate_patro::info`, data coverage (BS 2077-01-01 to 2084-08-20, tithi until 2083-12-30),
  "holiday data is informational, not affiliated with the Government of Nepal", the note that
  its version always equals `nepdate`'s, and the data provenance question from
  `shared/data/README.md`. Absolute links, as above.
- `#![warn(missing_docs)]` → `#![deny(missing_docs)]` in both libraries.
- `[package.metadata.docs.rs] all-features = true, rustdoc-args = ["--cfg", "docsrs"]` in
  `crates/nepdate/Cargo.toml`, and `#[cfg_attr(docsrs, doc(cfg(feature = "...")))]` on every
  item gated by `serde`, `jiff`, `chrono` or `std`.
- `crates/nepdate/examples/`: `convert.rs` (CLI: `cargo run --example convert -- 2081/04/15`,
  `-- --ad 2024-07-30`, and `-- --today`, where `--today` needs the `jiff` feature: declare
  `[[example]] name = "convert" required-features = ["jiff"]`), `month_calendar.rs` (prints a
  month grid like `cal`, using `short_name` weekday headers).
- Package include list, in both crates: `include = ["src/**", "README.md", "LICENSE", "LICENSE-THIRD-PARTY", "CHANGELOG.md"]`.
  `LICENSE` is a copy of the root MIT license. `LICENSE-THIRD-PARTY` holds the NepDate MIT notice
  (ADR-0013).

AC: `cargo doc -p nepdate -p nepdate-patro --all-features --no-deps` with
`RUSTDOCFLAGS=-D warnings` succeeds. `cargo package -p nepdate --list` and
`cargo package -p nepdate-patro --list` show no file from `shared/` or `tests/`.
`cargo package -p nepdate -p nepdate-patro` builds both. Packaged together, cargo resolves
`nepdate-patro`'s `nepdate` dependency to the freshly packaged `nepdate`, so this works before
anything is on crates.io. If the pinned cargo can't do that, stop and report it; don't switch
to `--no-verify`.

### L6-02 Benchmarks and competitor comparison
Implements `shared/spec/BENCHMARK.md` (B1 to B8) and the targets in ADR-0004 "Measurements".

- Workspace member `benches/nepdate-bench` (`publish = false`), so competitors never touch the
  library's dependencies, MSRV or `cargo deny`. Its `[dev-dependencies]`: `criterion = "0.8"`,
  the `nepdate` and `nepdate-patro` path dependencies, and the four crates in
  `docs/competitors.md` pinned with `=` (`npdatetime =0.2.10`, `adtobs =0.2.0`,
  `nepali-date-converter =0.1.1`, `nepdate-cli =0.1.5`). `[[bench]] name = "compare",
  harness = false`. In `deny.toml`: `[graph] exclude-unpublished = true` (two competitors are GPL;
  nothing GPL reaches users). Release profile for the bench crate: `lto = "fat"`,
  `codegen-units = 1` (what ADR-0004 measured with).
- `src/inputs.rs` builds the 64 dates of BENCHMARK §2 (serial `s0 + i * ((s1 - s0) / 63)`, `s0` =
  serial of 2000-01-01, `s1` = serial of 2090-12-30), their AD equivalents, and the 64
  `YYYY/MM/DD` strings, once, outside the timed loop.
- One `criterion` benchmark group per B-operation (`b1_bs_to_ad` ... `b8_month_grid`). Each
  iteration runs the operation over all 64 inputs; set `group.throughput(Throughput::Elements(64))`
  so the report shows time per operation. Consume every result with `std::hint::black_box`
  (criterion's own is deprecated). Settings: warm-up 3 s, measurement 5 s, 100 samples.
- Our functions per row: B1 `NepaliDate::new(y, m, d)?.to_gregorian()`; B2
  `NepaliDate::from_gregorian(y, m, d)`; B3 `checked_add_days(1000)`; B4
  `checked_add_months(13, MonthOverflow::Clamp)`; B5 `d.long(LongDate { weekday: true, ..LongDate::default() }, Lang::Ne).write_to(&mut buf)`
  into a reused 128-byte stack buffer that implements `fmt::Write`, plus a second function
  `to_string()` labelled "allocates, for comparison"; B6 `s.parse::<NepaliDate>()` (strict, `FromStr`); B7 `nepdate_patro::info`
  touching the tithi, holiday flag and every event name; B8 the same for every day of the month.
  B7 and B8 use the 64 dates moved into the patro coverage (hub BENCHMARK.md, "Inputs for B7 and
  B8"), so they time real lookups, not empty ones. Exact names are those in `docs/API.md`.
- Competitors: the calls in `docs/competitors.md` "Calls to benchmark", for B1 and B2 only (none
  has the other operations). Warm `nepali-date-converter` before timing and label it "includes
  string parsing". A competitor that returns an error or panics for an input in the 64 is run over
  the inputs it accepts and the row says how many.
- Allocations: `src/bin/alloc.rs` with a counting `#[global_allocator]` (wraps `System`, two
  `AtomicUsize` for calls and bytes). It runs each B-operation of each library 10,000 times over
  the 64 inputs and prints calls and bytes per operation. The library CI test
  (`crates/*/tests/no_alloc.rs`, ADR-0014) is the gate; this binary only fills the table.
- Agreement: `src/bin/agree.rs` compares each competitor's B1 and B2 results on the 64 dates with
  `nepdate` and prints "yes" or "no (k of 64 differ)".
- `cargo xtask bench` runs, in order and one at a time: `alloc`, `agree`, then
  `cargo bench -p nepdate-bench`, each pinned to logical CPU 2 (on Windows
  `cmd /c start /affinity 4 /wait ...`; on Linux `taskset -c 2`). It prints the exact commands.
- Record results in `docs/benchmarks.md` in the BENCHMARK §5 layout: the machine table, rustc and
  competitor versions, date, commands, and one table with columns ID, `nepdate` ns/op, bytes/op,
  each competitor's ns/op and "agrees", and C# NepDate ns/op and bytes/op copied from the hub's
  `docs/benchmarks/dotnet.md` with its hub commit.

AC:
1. `cargo bench -p nepdate-bench -- --list` lists eight groups `b1_` to `b8_`, and B1 and B2 each
   have five functions (ours and four competitors).
2. `cargo run -p nepdate-bench --release --bin alloc` prints 0 bytes and 0 calls for every
   `nepdate` and `nepdate-patro` B-operation except the labelled `to_string()` row.
3. `cargo run -p nepdate-bench --release --bin agree` prints one line per competitor; `nepdate`
   itself agrees on 64 of 64 with `shared/data` by construction (checked against the vectors).
4. On the reference machine, pinned to logical CPU 2, every B-row meets the ADR-0004 target and is
   faster than the C# column. `docs/benchmarks.md` exists with every column filled.
5. **If a target is missed**, don't micro-optimise speculatively and don't change the data
   layout. Stop and report the numbers to the human with a profile (`cargo flamegraph` is
   optional).

### L6-02b Scheduled fuzzing
`.github/workflows/fuzz.yml`: weekly `schedule` + `workflow_dispatch`, ubuntu, nightly Rust,
`cargo install cargo-fuzz --locked`, then each target of `fuzz/` (ADR-0014) for 10 minutes
(`cargo fuzz run <target> -- -max_total_time=600`). A crash uploads the artifact and fails the job.
AC: the workflow runs once by hand and every target completes with no crash; `fuzz/` is not a
workspace member, so `cargo xtask ci` doesn't build it.

### L6-03 Coverage gate
`cargo llvm-cov -p nepdate -p nepdate-patro --all-features --fail-under-lines 90` in `cargo xtask coverage`
(not in `ci`, because llvm-cov needs extra setup; CI runs it as a separate ubuntu job).

### L6-04 Release commands in xtask
- `cargo xtask release-check <package> <version>`, where `<package>` is `nepdate` or
  `nepdate-widget`. Fails unless the package's `Cargo.toml` version equals `<version>` and its
  `CHANGELOG.md` has a `## [<version>]` section. For `nepdate` it checks the pair (ADR-0002):
  - `crates/nepdate/Cargo.toml` and `crates/nepdate-patro/Cargo.toml` both have
    `version = "<version>"`;
  - both `CHANGELOG.md` files have a `## [<version>]` section;
  - the `nepdate` dependency in `crates/nepdate-patro/Cargo.toml` is
    `version = "~<major>.<minor>.0"` (for 0.1.3: `~0.1.0`).
- `cargo xtask changelog <package> <version>`: prints that section (without its heading) to
  stdout, then an *Install* block and a *Verification* block (published by trusted publishing;
  link to the full changelog at the tag). For `nepdate` it prints `### nepdate` and
  `### nepdate-patro` with each crate's section under it, and the install block is
  `cargo add nepdate@<version>` plus `cargo add nepdate-patro@<version>`.

AC: unit tests for both, using small changelog and manifest strings: missing section, wrong
version, section is the last one in the file, `nepdate-patro` version differs from `nepdate`,
dependency requirement `~0.0.0` while releasing 0.1.0.

### L6-05 Release workflow + runbook
`.github/workflows/release-lib.yml` per ADR-0012 and `docs/runbooks/release-library.md`:
- on tag `nepdate-v*`, and `workflow_dispatch` with input `dry-run` (boolean, default `true`).
- Jobs `preflight` → `ci` → `publish` (`environment: production`, permissions
  `contents: write` + `id-token: write`, `rust-lang/crates-io-auth-action`, then
  `cargo publish -p nepdate`, then `cargo publish -p nepdate-patro` (this order, ADR-0012), then
  `softprops/action-gh-release` with `body_path` from `cargo xtask changelog` and `prerelease`
  when the tag contains `-`) → `verify-published` (`cargo new` +
  `cargo add nepdate@<version> nepdate-patro@<version>` + run both README examples; retry every
  60 s for up to 10 min).
- Each `cargo publish` step first asks `https://crates.io/api/v1/crates/<crate>/<version>`; on
  HTTP 200 it prints "already published" and skips, so a rerun after a half-finished publish
  only publishes what is missing.
- `ci` also runs `cargo package -p nepdate -p nepdate-patro` and
  `cargo xtask check-core-has-no-patro`.
- From 0.1.1: `cargo semver-checks -p nepdate -p nepdate-patro` in `preflight`
  (`taiki-e/install-action`).
- All actions pinned by SHA (L0-05).

AC: a dry run on a test tag passes every job except `publish`, which is skipped.

### L6-06 Shared check and weekly sync
Needs hub task H3. Both workflows are the hub's reusable ones, pinned by SHA (ADR-0003):
- In `ci.yml`, job `shared-check`: `uses: sanamhub/nepdate/.github/workflows/shared-check.yml@<sha>`.
- `.github/workflows/shared-sync.yml`: `schedule: cron "15 3 * * 2"` + `workflow_dispatch`, one
  job `uses: sanamhub/nepdate/.github/workflows/shared-sync.yml@<sha>` with
  `regenerate: cargo xtask codegen` and the app secrets (`SYNC_APP_ID`, `SYNC_APP_PRIVATE_KEY`).

AC: editing any file under `shared/` on a test branch makes `shared-check` fail. A manual run of
`shared-sync` with `shared/VERSION` set to an older hub commit opens one PR whose generated
files pass `cargo xtask codegen --check`.

### L6-07 Version 0.1.0
Set `version = "0.1.0"` in `crates/nepdate/Cargo.toml` and `crates/nepdate-patro/Cargo.toml`,
change the `nepdate` requirement in `crates/nepdate-patro/Cargo.toml` to `~0.1.0`, and move
`[Unreleased]` to `[0.1.0] - <date>` in both `CHANGELOG.md` files.
AC: `cargo xtask release-check nepdate 0.1.0` passes. **Agent stops here.** The human follows
the first-release section of the runbook.

## Definition of Done
Global DoD + milestone M3 (human-published).

## Human review checklist
- [ ] Run `cargo doc --open -p nepdate -p nepdate-patro --all-features` and read it as a stranger. Is anything confusing?
- [ ] Run the release workflow once with `dry-run: true`.
- [ ] Run `shared-sync` once by hand and read the PR it opens.

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` and `cargo xtask coverage` pass (≥ 90 % lines for both libraries).
- [ ] `docs/benchmarks.md` has the BENCHMARK §5 table with every column filled: `nepdate` ns/op and
      bytes/op for B1 to B8, the four competitors (ns/op and "agrees") for B1 and B2, and the C#
      column with its hub commit; the exact `cargo xtask bench` commands and pinning are recorded.
- [ ] Every B-row meets its ADR-0004 target and beats the C# column, or the PR reports the miss
      with numbers and a profile and stops.
- [ ] `alloc` prints 0 bytes for every library B-operation; both `no_alloc` tests pass in CI.
- [ ] The fuzz workflow ran once by hand with no crash.
- [ ] `cargo doc` with `-D warnings` succeeds; `cargo package --list` shows no `shared/` or
      `tests/` file in either crate.
- [ ] `cargo xtask release-check nepdate 0.1.0` passes; the release workflow dry run passed.
- [ ] Nothing was published; the agent stopped at L6-07.
