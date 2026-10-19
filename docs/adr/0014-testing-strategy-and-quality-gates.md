# ADR-0014: Testing strategy and quality gates

- **Status:** accepted
- **Date:** 2026-09-24 (revised 2026-10-07: gates for `nepdate-patro`, hub ADR-0004; revised
  2026-10-08: C# test catalogue, allocation, fuzz and thread-safety tests, benchmark protocol)
- **Approved by:** Sanam

## Context

The testing rules are a test pyramid, tests per acceptance criterion, an 80 % coverage
target, lint errors that block merge, and CI security scanning. Every change passes the
same gates.

## Decision

### Test layers

| Layer | Where | What |
|---|---|---|
| Unit | `#[cfg(test)] mod tests` next to the code | every function; edge cases (range ends, month 12→1, 29/32-day months) |
| Doc tests | `///` examples on every public item | API usage stays correct |
| Conformance | `crates/nepdate/tests/vectors.rs` | all rows of `shared/spec/vectors/month-boundaries.csv` |
| C# parity | `crates/nepdate/tests/csharp_parity.rs` | all rows of `shared/spec/vectors/csharp-golden/*.tsv` except documented deviations (ADR-0009) |
| Patro | `crates/nepdate-patro/tests/anchors.rs` | every anchor of `shared/spec/PATRO.md` §4, counts T/H/E, coverage |
| C# catalogue | `crates/nepdate/tests/csharp_catalog.rs`, `crates/nepdate-patro/tests/csharp_catalog.rs` | one test per row of `shared/spec/vectors/csharp-tests/catalog.tsv`, tagged with its catalogue group and named `cs_<id>` by the catalogue rule, or a `// cs:<id> D-xx` comment for a deviation; `cargo xtask catalog-check` proves none is missing |
| Exhaustive | `crates/nepdate/tests/exhaustive.rs` | every serial 0..=109211 round-trips BS↔serial↔AD, and the table path (ADR-0004) equals a direct transcription of ALGORITHM §4 to §6 (fast: ~110k iterations) |
| Allocation | `crates/nepdate/tests/no_alloc.rs`, `crates/nepdate-patro/tests/no_alloc.rs` (`harness = false`) | a counting `#[global_allocator]`; each B-operation of `shared/spec/BENCHMARK.md` runs over the 64 dates and must allocate 0 bytes |
| Fuzz | `fuzz/` (cargo-fuzz, not a workspace member) | targets `parse_strict`, `parse_lenient`, `format_pattern` (arbitrary pattern and date): no panic; parse of a formatted date round-trips |
| Thread safety | `crates/*/tests/send_sync.rs` | compile-time assertion that every public type is `Send + Sync`; one test converts on 8 threads at once and compares with the single-thread result |
| Compile time | `const _: () = assert!(..)` in the library, `compile_fail` doctests | table invariants hold in every build; private fields can't be set from outside; `NepaliDate::new` works in a `const` |
| Property | `proptest` in `crates/nepdate/tests/props.rs` | `add_days(a).add_days(-a) == id`; `ordering(bs) == ordering(ad)`; `parse(format(x)) == x` for every format; `write_to` output equals `Display` output; arbitrary strings never panic either parser |
| Widget logic | unit tests on `apps/mitibar/src/app/*` (no UI) | month-grid model, settings load/validate, clock text, next-holiday search |
| Widget UI | manual checklist per release (`docs/runbooks/widget-manual-test.md`) + Slint's `slint::testing` where practical | tray, popup, Devanagari rendering |

### Tools and gates (all block merge)

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  (workspace lints: `clippy::pedantic` warn with a documented allow-list,
  `unsafe_code = "forbid"`, `missing_docs = "warn"` → error in the library)
- `cargo nextest run --workspace --all-features` + `cargo test --doc --workspace`
- `cargo build -p nepdate --no-default-features` and
  `cargo build -p nepdate --no-default-features --target wasm32-unknown-unknown`
- `cargo test -p nepdate` and `cargo test -p nepdate-patro` each on their own, and
  `cargo build -p nepdate-patro --target wasm32-unknown-unknown`
- `cargo xtask check-core-has-no-patro`: the core manifest has no `nepdate-patro`
  dependency and the built core contains no `World Day Against Human Trafficking`
- `cargo xtask codegen --check` (generated code drift)
- `cargo deny check` (advisories, licenses, bans, sources)
- Coverage: `cargo llvm-cov nextest -p nepdate -p nepdate-patro --all-features` must be
  **≥ 90 %** lines for the libraries (higher than the general 80 % because it's pure logic),
  ≥ 80 % for `apps/mitibar/src/app`.
- MSRV job: build with the declared `rust-version`.
- No Miri job: `#![forbid(unsafe_code)]` in both libraries and no dependencies leave Miri nothing
  to check that the normal tests don't (Miri finds undefined behaviour, which needs `unsafe`).
  If `unsafe` is ever proposed, the ADR proposing it adds a Miri job.
- Fuzzing runs in the scheduled fuzz workflow, not per PR: it needs nightly Rust and
  libFuzzer (Linux).

### CI matrix
`ubuntu-latest`, `windows-latest`, `macos-latest` × `stable`; plus MSRV on ubuntu.
Linux packages: `libxkbcommon-dev libwayland-dev libfontconfig1-dev libdbus-1-dev`.

### Benchmarks
`criterion` 0.8 benches in the unpublished `benches/nepdate-bench` crate implement B1 to B8 of
`shared/spec/BENCHMARK.md` for `nepdate` and `nepdate-patro`, and the same operations for each
competitor crate in `docs/competitors.md` where it has them. Run on the reference machine
pinned to logical CPU 2 by `cargo xtask bench`, manually before a release, not per PR.
Target (BENCHMARK §6, raised by the maintainer to every operation): every B-row faster than the C#
NepDate column copied from the hub, with 0 bytes per operation. The allocation test above
enforces the 0 bytes in CI on every PR.

### Dev-only dependencies this ADR allows
`proptest` (property tests), `serde_json` (tests of the `serde` feature), `criterion` 0.8 and the
pinned competitor crates (only in `benches/nepdate-bench`), `libfuzzer-sys` (only in `fuzz/`). None
reaches a published crate's normal dependencies.

## Consequences

- A pull request is done only when `cargo xtask ci` (runs all gates locally) passes.
