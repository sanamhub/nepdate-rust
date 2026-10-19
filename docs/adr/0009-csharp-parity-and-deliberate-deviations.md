# ADR-0009: Parity with C# NepDate: Rust names for the shared deviations

- **Status:** accepted
- **Date:** 2026-09-24 (revised 2026-09-25: the deviation list moved to the hub)
- **Approved by:** Sanam

## Context

Parity rules and the deviation list D-01 … D-15 bind every port, so they live in the hub:
`shared/spec/PARITY.md` (hub ADR-0003). This ADR only says how the Rust port applies them.

## Decision

- `crates/nepdate/tests/csharp_parity.rs` loads `shared/spec/vectors/csharp-golden/*.tsv` and
  must match every row, except rows affected by a deviation in `shared/spec/PARITY.md`, which
  the test skips with an explicit filter and a comment naming the D-ID.
- Rust API for each deviation (full signatures in `docs/API.md`):

| ID | Rust |
|---|---|
| D-01 | no `Default` impl |
| D-02 | `checked_add_months(i32, MonthOverflow)` with `MonthOverflow::{Clamp, Overflow}` |
| D-03 | `checked_add_days(i32)` |
| D-04 | `Month::short_name` returns `Asa` / `Aso` |
| D-05, D-06 | `FromStr` (strict), `NepaliDate::parse_lenient` |
| D-07 | `a.days_until(b) -> i32` |
| D-08 | `FiscalYear` newtype: `FiscalYear::containing(date)`, `.start()`, `.end()`, `.quarter_range(q)` |
| D-09 | `NepaliDateRange`: `month()`, `year()`, `contains`, `len`, `days()` |
| D-10 | not ported; docs show `iter().map()` / `rayon` |
| D-11 | `serde` feature, string `"YYYY-MM-DD"` |
| D-12 | not ported; year length is `NepaliDateRange::year(y)?.len()` (364..=367) |
| D-13 | separate crate `nepdate-patro`: `nepdate_patro::info(date)`, `coverage()`, `public_holidays()` |
| D-14 | `today().checked_add_days(±1)` in docs; `is_today()` only |
| D-15 | `Lang::Ne` on every formatter |

- A Rust-only deviation is not allowed. A new deviation is a PR to the hub first.

## Consequences

- Rust users coming from C# get a mapping table (ADR-0007) plus this list.
