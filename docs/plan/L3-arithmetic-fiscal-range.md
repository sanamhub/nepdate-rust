# L3: Arithmetic, diff, fiscal year, ranges

**Goal:** date arithmetic identical to C# (except deviations), plus the Rust-idiomatic
`FiscalYear` and `NepaliDateRange`.

Read first: `shared/spec/ALGORITHM.md` §8, 8a, 8b; `docs/API.md` (arithmetic, fiscal, range);
ADR-0009 D-02, D-03, D-07, D-08, D-09.
Human reads: `docs/learn/04-traits-and-iterators.md`.

## Tasks

### L3-01 `checked_add_days`, `checked_add_months`, `checked_add_years` (`src/arith.rs`)
Exactly ALGORITHM §8. Use `i64` for intermediate sums so that
`checked_add_days(i32::MAX)` returns `OutOfRange` instead of overflowing.
`MonthOverflow` enum per API.

AC:
- Unit tests for the ALGORITHM §8 examples and a few more: 2081-04-32 +2 months Clamp →
  2081-06-30, Overflow → 2081-07-02; −2 (either mode) → 2081-02-32; +5 Clamp → 2081-09-29,
  Overflow → 2081-10-03; 2080-12-30 +5 days → 2081-01-05; 2081-01-05 −5 days → 2080-12-30.
- Extreme inputs: `MAX.checked_add_days(1)`, `MIN.checked_add_days(-1)`,
  `checked_add_months(i32::MIN, _)` → `OutOfRange`, no panic.

### L3-02 `diff` → `DateDiff` (ALGORITHM §8a)
AC: `a.diff(a)` all zeros; `2080-01-01 → 2081-01-01` = 1y 0m 0d, total 365;
`2081-04-32 → 2081-05-01` = 0y 0m 1d; reversed args → `negative: true`, same magnitudes.
Proptest (L3-05): `a.checked_add_months(years*12+months, Clamp)?.checked_add_days(days)? == b`.

### L3-03 `FiscalYear`, `Quarter` (`src/fiscal.rs`, ALGORITHM §8b)
`label(lang)` returns a small Display struct. En: `2082/83`. Ne: `२०८२/८३`.
Needs `Lang::digits` (if L4 isn't merged yet, implement `digits` here in `names.rs` and
L4 reuses it).

AC: `FiscalYear(2082).label(Lang::Ne)` displays `२०८२/८३` and `FiscalYear(2099).label(Lang::En)`
displays `2099/00`; `FiscalYear::containing(2081-03-31)` = `FiscalYear(2080)` and
`containing(2081-04-01)` = `FiscalYear(2081)`; `FiscalYear(1900).start()` and
`FiscalYear(2199).end()` → `OutOfRange` (ALGORITHM §8b).

### L3-04 `NepaliDateRange`, `Days` iterator (`src/range.rs`)
- Store `start` and `end` as dates. `new` is a `const fn` (`docs/API.md`): compare
  `(year, month, day)` field by field, because `>` on `NepaliDate` is a trait call and not
  allowed in a `const fn`. The iterator is a serial cursor `{ front: u32, back: u32, done: bool }`.
- Implement `Iterator` (+ `size_hint` exact), `DoubleEndedIterator`, `ExactSizeIterator`,
  `FusedIterator`.

AC: `NepaliDateRange::month(2081, 4)?.len() == 32`; `.days().rev().next()` is the last day;
`year(2087)?.len() == 367`; for every year 1901..=2199, `year(y)?.len()` equals the sum of its
12 month lengths; `new(b, a)` with `a < b` → `None`; `new(a, a)` has `len() == 1`;
`days().len()` equals `len()`; `year(2200)` → `OutOfRange`.

### L3-05 Tests
- `tests/csharp_parity.rs`: add `add-days.tsv` (all rows), `add-months.tsv` (all rows, both
  columns), `dates.tsv` fiscal columns (`fy_start`, `fy_end`, `quarter_start`, `quarter_end`).
  `ERR` ⇔ `Err(Error::OutOfRange)`.
- `tests/props.rs` with `proptest` (dev-dependency). Strategy: any serial in range →
  `NepaliDate`. Properties: add_days inverse; `a < b ⇔ a.to_gregorian() < b.to_gregorian()`;
  `days_until` antisymmetry; the diff reconstruction from L3-02.

### L3-06 C# catalogue rows (phase 2), zero allocation (B3, B4), thread safety
- `crates/nepdate/tests/csharp_catalog.rs`: one `cs_` test per catalogue row with `phase` = 2
  (`cargo xtask catalog-check 2 --list`; files in `docs/plan/00-ROADMAP.md` "C# test catalogue").
  The 17 `NepaliDateIsRelativeTests` rows get `// cs:<id> D-14` comments. Add
  `cargo xtask catalog-check 2` to `cargo xtask ci`.
- Add to `crates/nepdate/tests/no_alloc.rs` (L2-09): B3 (`checked_add_days(1000)`), B4
  (`checked_add_months(13, MonthOverflow::Clamp)`), `diff`, `fiscal_year`, and a full
  `NepaliDateRange::year(2081)?.days()` walk.
- Add `DateDiff`, `MonthOverflow`, `FiscalYear`, `Quarter`, `NepaliDateRange` and `Days` to
  `tests/send_sync.rs`.

AC: `cargo xtask catalog-check 2` exits 0 (or the PR says H5 is not synced yet);
`cargo test -p nepdate --test no_alloc` passes with the new rows.

## Definition of Done
Global DoD. Parity files for arithmetic are 100 % green (no deviations apply to integer
arithmetic).

## Human review checklist
- [ ] Why `i64` in `checked_add_days`? Try `i32::MAX + 1` in the Rust Playground in debug mode.
- [ ] Read the `Iterator` impl for `Days`: which method is required and which are "free"? (learn/04 §3)
- [ ] Compare `FiscalYear` with the C# `FiscalYearQuarterStartDate(quarter, yearOffset)` API. Which is easier to misuse?

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes; the summary line is in the PR.
- [ ] `checked_add_days` is `from_serial(to_serial + n)` with an `i64` range check: no loop over
      days or months (ADR-0004 §4).
- [ ] `add-days.tsv` and `add-months.tsv` pass in full; no filter or skip in those tests.
- [ ] The year-length property runs for every year 1901 to 2199.
- [ ] `Days` implements `ExactSizeIterator`, `DoubleEndedIterator` and `FusedIterator`, and
      `size_hint` is exact.
- [ ] `cargo test -p nepdate --test no_alloc` covers B1 to B4 and passes.
- [ ] `cargo xtask catalog-check 2` exits 0, or the PR says H5 is not synced yet; every D-14
      comment names a real row.
- [ ] `send_sync.rs` lists the new public types.
