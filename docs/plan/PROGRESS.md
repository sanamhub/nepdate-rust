# Progress

One row per task (hub `docs/handover.md` §3). Status is `done`, `blocked` (on what) or `partial`
(what is missing).

| Task | Status | Commit | CI last line | Notes and open questions |
|---|---|---|---|---|
| L0-01, L0-02 | done | 9cf0dcc | `xtask ci: all 9 steps passed` | toolchain 1.98 (windows-gnu host locally) |
| L0-03, L0-04 | done | ddebe5a | `xtask ci: all 9 steps passed` | AC checked: an unused `let` fails step 2, a mis-indented line fails step 1. `unused-allowed-license = "allow"` because the list covers later phases |
| L0-05 | partial | b1d5f81 | actionlint 1.7.12 clean | "CI green on all 3 OSes" needs the GitHub repository (human task). MSRV job sets `RUSTUP_TOOLCHAIN`, since `rust-toolchain.toml` would override the 1.85 toolchain |
| L0-06 | partial | see L0-06 commit | actionlint clean | files done; repository settings (branch protection, dependency graph, private vulnerability reporting, `production` environment) are human tasks |
| L0-07 | done | this commit | | |
| L0 checkpoint | ready for review | this commit | `xtask ci: all 9 steps passed` | open: CI run on GitHub |
| L1-01, L1-02 | done | 35c87f6 | `xtask ci: all 9 steps passed` | codegen twice is byte-identical; a changed length in the JSON fails `codegen --check`. Binary size AC (43 to 46 KB) is measured in L2, when conversion first uses the tables (in L1 the linker drops them) |
| L1-03 to L1-05 | done | 578bce3 | `xtask ci: all 9 steps passed` | 46 unit tests and doctests; every valid day of 1901 to 2199 builds and day `len + 1` fails |
| L1 checkpoint | ready for review | this commit | `xtask ci: all 9 steps passed` | `Lang::digits` comes in L4 as planned |
| L2-01 to L2-07 | done | 78be674 | `xtask ci: all 9 steps passed` | vectors 7176 rows, C# dates.tsv 1018 rows, exhaustive 109,212 serials against a copy of ALGORITHM §4/§5. Binary size (L1-02 AC): a release binary converting both ways grows by 47,616 B; the four tables are 44,260 B of it (in the 43 to 46 KB range), the rest is the probe's Display/Debug and panic code |
| L2-09 | done | 0c27688 | `xtask ci: all 10 steps passed` | deviation: workspace `unsafe_code` is `deny`, not `forbid`, so `tests/no_alloc.rs` alone can implement `GlobalAlloc`; both libraries keep `#![forbid(unsafe_code)]` in lib.rs. `no_alloc` runs as its own CI step because nextest can't list a harness-less binary |
| L2-08 | done | a9267a1 | `xtask ci: all 11 steps passed` | 113 phase-1 rows: 103 `cs_` tests (89 run, 14 `#[ignore = "L4-03: needs FromStr"]` or L5 for `Today`), 10 deviation comments (D-06 ×5, D-12 ×3, D-01 ×2). Rows needing a later API follow hub PORT-PLAN (94946ac) |
| L2 checkpoint | ready for review | this commit | `xtask ci: all 11 steps passed` | |
| L3-01, L3-02 | done | 8d38dbc | clippy and tests only; full CI at L3-06 | all ALGORITHM §8 and §8a examples; `i32::MAX`/`i32::MIN` days, months and years give `OutOfRange` |
| L3-04 | done | 3377bfb | clippy and tests only; full CI at L3-06 | year length equals the sum of its months for every year 1901 to 2199; `Days` has no `Clone`/`Debug` derive because `docs/API.md` lists none |
| L3-03 | done | 5a32636 | clippy and tests only; full CI at L3-06 | `Lang::digits` and `Digits` added here (L3 plan allows it); `Digits` lives in `names.rs`, re-exported at the crate root, until L4-02 adds `format.rs` |
| L3-05 | done | 52fed44 | clippy and tests only; full CI at L3-06 | add-days.tsv 9162 rows, add-months.tsv 10180 rows (both columns), dates.tsv fiscal columns 1018 rows, no row skipped. proptest without its `std` feature: `std` pulls in `getrandom`, which needs an assembler on the windows-gnu host; the seed is then fixed |
| L3-06 | done | 54d1170 | `xtask ci: all 12 steps passed` | 180 phase 2 rows: 129 `cs_` tests (120 run, 8 `#[ignore = "L5: ..."]` for `today()`, 1 `#[ignore = "L4-04: ..."]`), 51 deviation comments (D-09 ×45, D-02 ×4, D-06 ×2); 17 `NepaliDateIsRelativeTests` D-14 comments. Open: the 4 range `ToString` rows use D-09 (the port's range has no text form); the human may prefer a different reading |
| L3 checkpoint | ready for review | this commit | `xtask ci: all 12 steps passed` | branch `feat/L3-arith`, not merged |
