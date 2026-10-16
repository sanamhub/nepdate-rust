# Roadmap

Two products, built in order: **libraries first** (L0–L6: the core `nepdate` and the patro
crate `nepdate-patro`), then **widget** (W0–W4).
Each phase is one PR (or a small PR series), sized so an agent can finish it in one
session and a Rust beginner can review it in about an hour.

```
L0 workspace+CI ─► L1 core types+codegen ─► L2 conversion ─► L3 arithmetic/fiscal/range
                                                   │
                                                   └──────► L4 format+parse ─► L5 patro+features ─► L6 release 0.1.0
                                                                                                 │
W0 skeleton+font spike ─► W1 tray+popup+clock ─► W1B Windows taskbar text ─► W1R rename to MitiBar ─► W2 month grid ─► W3 converter/settings/lifecycle ─► W4 package+release
```

L3 and L4 both depend only on L2 and may run in parallel (separate agents/branches).
W0 may start once L2 is merged. W1 needs L4 (formatting); W2 needs L5 (`nepdate-patro` and
the `jiff` feature).
Hub dependency: L0 needs hub task H0 merged (first `shared/VERSION`). L6-06 needs hub H3
(reusable sync workflows). Calendar data arrives through the weekly `shared-sync` PR; the hub
warns when data is about to run out ([data-update runbook](https://github.com/sanamhub/nepdate/blob/main/docs/runbooks/data-update.md)).

The PORT-PLAN column maps each phase to `shared/spec/PORT-PLAN.md`; its acceptance criteria
are part of the tasks listed.

| Phase | PORT-PLAN | File | Output | Learn doc |
|---|---|---|---|---|
| L0 | 0 | `L0-workspace-and-ci.md` | workspace, toolchain, lints, CI, `cargo xtask ci` | `learn/01-cargo-workspace.md` |
| L1 | 1 | `L1-core-types-and-codegen.md` | `NepaliDate::new`, `Month`, `Weekday`, `Error`, generated calendar table | `learn/02-structs-enums-errors.md` |
| L2 | 1 | `L2-conversion.md` | serial, BS↔AD, weekday, vectors + exhaustive + parity tests | `learn/03-const-fn-and-tests.md` |
| L3 | 2 | `L3-arithmetic-fiscal-range.md` | add days/months/years, diff, fiscal year, ranges + iterators | `learn/04-traits-and-iterators.md` |
| L4 | 3 | `L4-format-and-parse.md` | Display adapters, patterns, strict + lenient parsing | `learn/05-display-fromstr-strings.md` |
| L5 | 4 | `L5-patro-and-interop.md` | crate `nepdate-patro`; features `serde`, `jiff`, `chrono`; patro-free core check | `learn/06-features-and-cfg.md` |
| L6 | 5 | `L6-library-release.md` | docs, benches, coverage ≥ 90 %, release automation, `shared/` sync, crates.io 0.1.0 of both crates | `learn/07-docs-benches-release.md` |
| W0 | – | `W0-widget-skeleton.md` | Slint app boots, fonts, Devanagari spike (go/no-go) | `learn/08-slint-basics.md` |
| W1 | – | `W1-tray-popup-clock.md` | tray, popup, header, clock timer | `learn/09-event-loop-and-state.md` |
| W1B | – | `W1B-windows-taskbar-text.md` | Windows 11 taskbar text (ADR-0016, WR-12) | – |
| W1R | – | `W1R-rename-mitibar.md` | Rename the app to MitiBar (ADR-0017), no behaviour change | – |
| W2 | – | `W2-month-grid.md` | grid, navigation, day detail, next holiday | `learn/10-models-and-callbacks.md` |
| W3 | – | `W3-converter-settings-lifecycle.md` | converter, settings, persistence, autostart, single instance | `learn/11-io-and-platform.md` |
| W4 | – | `W4-package-and-release.md` | installers, perf check, manual test, release 0.1.0 | `learn/12-shipping.md` |

## Task IDs and traceability (org §2 "code ties to a ticket")

- Task IDs: `L2-03`, `W1-02`, … (phase + number). If an external tracker (ADO/GitHub
  Issues) is used, create one issue per task with the ID in the title.
- Branch: `feat/L2-03-serial-conversion`. Commit subject:
  `feat(nepdate): L2-03 serial conversion`. PR title starts with the task ID(s).
- Widget requirement IDs (`WR-03 AC2`) are cited in widget PRs and tests
  (`#[test] fn wr03_ac1_grid_has_5_or_6_rows()`).

## Global Definition of Done (every task)

1. All acceptance criteria of the task are met and each has at least one test (or, for
   UI-only criteria, a line in the manual checklist `docs/runbooks/widget-manual-test.md`).
2. `cargo xtask ci` passes locally (fmt, clippy `-D warnings`, tests, doc tests,
   no-default-features build, codegen drift check, cargo-deny, and from L5 the patro-free core
   check).
3. No new public API beyond `docs/API.md`. No new dependency beyond the ADRs.
4. No `unwrap()`/`expect()`/`panic!` in library code paths (tests/examples OK).
5. The package's `CHANGELOG.md` (`crates/nepdate/`, `crates/nepdate-patro/` or
   `apps/nepdate-widget/`) `[Unreleased]` updated for user-visible changes.
6. Prose follows `.claude/skills/writing-style/SKILL.md`.
7. PR description: task IDs, what changed, how it was tested, and "What to look at
   first" for the reviewer (2–3 bullet points, beginner-friendly).

## C# test catalogue

`shared/spec/vectors/csharp-tests/catalog.tsv` (hub task H5, in `shared/` since 2026-10-09) lists every test
case of C# NepDate's suite at `cb05cb65` (395 `[Fact]` and 147 `[InlineData]` rows) with a `phase`
column holding the PORT-PLAN phase (1 to 4), `n/a` or a PARITY deviation. Each row becomes one
`#[test] fn cs_<name>()` in `crates/nepdate/tests/csharp_catalog.rs` (phases 1 to 3) or
`crates/nepdate-patro/tests/csharp_catalog.rs` (phase 4), with `<name>` made from the row id by the
rule in L2-08; each `#n` row of a theory is its own test (`..._3`). A row that doesn't apply gets a
`// cs:<id> D-xx` comment. `cargo xtask catalog-check <phase>` (L2-08) proves nothing is missing and
runs in `cargo xtask ci` from the phase that adds it. The catalogue's `phase` column wins; until it
lands, this is the expected mapping by file (counts are `[Fact]` plus `[InlineData]` lines in each
file):

| C# test file | Rows | PORT-PLAN phase | Rust phase |
|---|---|---|---|
| `Core/NepaliDateConstructionTests.cs`, `Core/NepaliDatePropertiesTests.cs`, `Abilities/NepaliDateComparableTests.cs`, `Core/DictionaryIntegrityTests.cs` | 29, 9, 11, 11 | 1 | L2 (L2-08; L1 has no catalogue task because the checker arrives in L2) |
| `Core/OptimizationVerificationTests.cs` | 133 | split by method prefix | `Constructor`, `CompareTo`, `Equals`, `GetHashCode`, `Operator`, `DayOfWeek`, `EnglishDate`, `MonthEndDay`, `IsDefault`, `IsLeapYear` in L2; `Subtraction` in L3; `ToString`, `ToLongDateString`, `ToUnicodeString`, `UnicodeConversion`, `SmartDateParser` in L4; `BulkConvert` n/a |
| `Core/NepaliDateArithmeticEdgeCaseTests.cs`, `Core/NepaliDateManipulationTests.cs`, `Core/FiscalYearTests.cs`, `Core/FiscalYearInstanceMethodTests.cs`, `Core/NepaliDateRangeTests.cs`, `Core/NepaliDateRangeExtendedTests.cs` | 25, 12, 11, 45, 19, 64 | 2 | L3 (L3-06) |
| `Core/NepaliDateIsRelativeTests.cs` | 17 | deviation | D-14 (not ported; `// cs:` comments in L3) |
| `Core/NepaliDateMonthNameTests.cs`, `Core/SmartDateParserTests.cs`, `Abilities/NepaliDateFormattableTests.cs`, `Abilities/NepaliDateParsableTests.cs`, `Extensions/StringExtensionsTests.cs` | 31, 14, 20, 5, 5 | 3 | L4 (L4-06) |
| `Core/CalendarDataTests.cs` | 35 | 4 | L5 (L5-07); the `CalendarOffsets_*` rows test C#'s internal layout and map to the dense-table assertions of L5-01 |
| `Extensions/DateTimeExtensionsTests.cs` | 5 | 4 | L5 (L5-07, against the `jiff` and `chrono` features) |
| `Core/BulkConvertTests.cs`, `Abilities/NepaliDateTypeConverterTests.cs`, `Serialization/NepaliDateSerializationTests.cs`, `Integration/MemoryUsageTests.cs`, `Integration/MemoryOptimizationBenchmark.cs` | 7, 18, 9, 4, 3 | n/a | none: C#-only API (ADR-0009); the memory rows are replaced by the zero-allocation tests (ADR-0014) |

If this table and `catalog.tsv` disagree, the catalogue wins and the PR says so; the row names are
checked against the file.

### Extra tests Rust allows (beyond the catalogue)

| Kind | Where | Phase |
|---|---|---|
| Zero allocation (counting `#[global_allocator]`, every B-operation) | `crates/*/tests/no_alloc.rs` | L2 (B1, B2), L3 (B3, B4), L4 (B5, B6), L5 (B7, B8) |
| `Send + Sync` assertions and an 8-thread run | `crates/*/tests/send_sync.rs` | L2, extended in every later phase |
| Compile-time checks (`const _: () = assert!(..)` on tables, `const` use of `NepaliDate::new`, `compile_fail` doctests) | library source | L1 |
| `#![forbid(unsafe_code)]`; no Miri job while it holds (ADR-0014) | both `lib.rs` | L0 |
| Property tests (`proptest`) | `crates/nepdate/tests/props.rs` | L3, L4 |
| Fuzzing (`cargo-fuzz`, targets `parse_strict`, `parse_lenient`, `format_pattern`) | `fuzz/` | written in L4 (L4-07), scheduled in L6 (L6-02b) |
| Exhaustive (all 109,212 serials) | `crates/nepdate/tests/exhaustive.rs` | L2 |

## Milestones

| Milestone | Phases | Exit |
|---|---|---|
| M1 "converts" | L0–L2 | all 7176 boundary vectors + exhaustive round-trip pass |
| M2 "parity" | L3–L4 | all C# golden rows pass except ADR-0009 deviations |
| M3 "lib 0.1.0" | L5–L6 | `nepdate` and `nepdate-patro` 0.1.0 published to crates.io (after human approval) |
| M4 "widget preview" | W0–W3 | runs on Win/macOS/Linux from `cargo run` |
| M5 "widget 0.1.0" | W4 | signed-or-flagged installers on GitHub Releases |
