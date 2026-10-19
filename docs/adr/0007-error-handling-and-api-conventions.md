# ADR-0007: Error handling and public API conventions

- **Status:** accepted
- **Date:** 2026-09-24
- **Approved by:** Sanam

## Context

C# NepDate throws exceptions (`InvalidNepaliDateFormatException`, etc.) and offers
`TryParse` twins. Rust has no exceptions; fallible operations return `Result`.
Consistent conventions make the API predictable and easy to extend.

## Decision

### Errors

One public, non-exhaustive error enum, `Copy`, no allocation:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    OutOfRange,                  // outside 1901-01-01..=2199-12-30 BS (or AD equivalent)
    InvalidMonth(u8),            // month not in 1..=12
    InvalidDay { month_length: u8, day: u8 },
    InvalidGregorian,            // AD input that is not a real date, e.g. 2023-02-29
    Parse(ParseErrorKind),       // see shared/spec/PARSING.md
}
```

- `impl core::fmt::Display for Error`; `impl std::error::Error` behind `std`.
- No `thiserror`/`anyhow` in the library (zero deps). The widget may use `anyhow`.

### Panics

- Public library functions **never panic** on any input. Arithmetic uses
  `checked_*` and returns `Result`/`Option`.
- Operator impls (`date + Days(n)`) are **not** provided in v0.1 because `Add` cannot
  return `Result` without surprising users. Use `checked_add_days`. (Revisit via ADR.)
- `#[track_caller]` `expect`/`unwrap` allowed only in tests, examples and the widget's `main`.

### Naming (follow Rust API Guidelines, https://rust-lang.github.io/api-guidelines/)

| C# | Rust |
|---|---|
| `new NepaliDate(y, m, d)` (throws) | `NepaliDate::new(y, m, d) -> Result<Self, Error>` (`const fn`) |
| `NepaliDate.Parse(s)` / `TryParse` | `s.parse::<NepaliDate>()` via `FromStr` → `Result` |
| `date.EnglishDate` | `date.to_gregorian() -> GregorianDate` / `From<NepaliDate> for chrono::NaiveDate` (feature) |
| `DateTime.ToNepaliDate()` | `NepaliDate::from_gregorian(y, m, d) -> Result` / `TryFrom<NaiveDate>` |
| `AddDays(n)` | `checked_add_days(i32) -> Result<Self, Error>` |
| `AddMonths(n, awayFromMonthEnd)` | `checked_add_months(i32, MonthOverflow) -> Result` (see spec) |
| `MonthEndDay` | `month_length() -> u8` |
| `DayOfWeek` | `weekday() -> Weekday` |
| `ToString(format)` | `Display` (default `YYYY/MM/DD`) + `short(..)`, `long(..)`, `format_pattern(pattern, lang)` (`docs/API.md`, `shared/spec/FORMATTING.md`) |
| `IsToday` | `is_today()` behind feature `jiff` (ADR-0011) |

- Getters are plain nouns: `year()`, `month()`, `day()` (no `get_` prefix).
- Conversions: `to_*` for cheap-by-value, `into_*` for consuming, `from_*` constructors.
- Every public item has a doc comment with at least one runnable doctest example.
- `#[must_use]` on all pure functions returning a value.
- Enums `Month` and `Weekday` are `#[repr(u8)]`, `Copy`, with `TryFrom<u8>`.

## Consequences

- API feels native to Rust users; C# users get a mapping table in the README.
- `#[non_exhaustive]` lets us add error variants without a breaking release.
