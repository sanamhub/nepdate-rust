<p align="center">
  <img src="https://raw.githubusercontent.com/sanamhub/nepdate-rust/main/docs/assets/nepdate-rust-128.png" width="96" alt="NepDate logo">
</p>

<h1 align="center">nepdate</h1>

<p align="center">
  <a href="https://crates.io/crates/nepdate"><img src="https://img.shields.io/crates/v/nepdate?include_prereleases&logo=rust" alt="nepdate on crates.io"></a>
  <a href="https://docs.rs/nepdate"><img src="https://img.shields.io/docsrs/nepdate?logo=docsdotrs" alt="docs.rs"></a>
  <a href="https://github.com/sanamhub/nepdate-rust/actions/workflows/ci.yml"><img src="https://github.com/sanamhub/nepdate-rust/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/sanamhub/nepdate-rust/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License: MIT"></a>
</p>

Bikram Sambat (BS, the Nepali calendar) dates for Rust: conversion to and from the Gregorian
calendar (AD), arithmetic, fiscal years, formatting in English or Nepali, and parsing. `no_std`,
no dependencies by default, no heap allocation, no `unsafe`.

Tithi, public holidays and events are in the companion crate
[`nepdate-patro`](https://crates.io/crates/nepdate-patro), so this crate carries no patro data.

> **Status:** 0.1.0-beta.1, the first public pre-release. The API may still change before 0.1.0.

## Quick start

```toml
[dependencies]
nepdate = "=0.1.0-beta.1"
```

```rust
use nepdate::{Lang, LongDate, MonthOverflow, NepaliDate, Weekday};

// BS to AD.
let date = NepaliDate::new(2081, 4, 15)?;
let ad = date.to_gregorian();
assert_eq!((ad.year(), ad.month(), ad.day()), (2024, 7, 30));
assert_eq!(date.weekday(), Weekday::Tuesday);

// AD to BS.
assert_eq!(NepaliDate::from_gregorian(2024, 7, 30)?, date);

// Arithmetic never panics: out of range gives an error.
assert_eq!(date.checked_add_days(1000)?.to_string(), "2084/01/13");
assert_eq!(
    date.checked_add_months(13, MonthOverflow::Clamp)?.to_string(),
    "2082/05/15"
);

// Formatting and parsing.
assert_eq!(date.to_string(), "2081/04/15");
assert_eq!(
    date.long(LongDate::default(), Lang::En).to_string(),
    "Shrawan 15, 2081"
);
assert_eq!("2081/04/15".parse::<NepaliDate>()?, date);
assert_eq!(NepaliDate::parse_lenient("15 Shrawan 2081")?, date);
# Ok::<(), nepdate::Error>(())
```

Every formatter (`short`, `long`, `format_pattern`, `Lang::digits`, `FiscalYear::label`) has a
`write_to` method that writes into any `core::fmt::Write` without allocating; `to_string()` needs
`std`.

Two runnable examples are in the repository:
[`convert`](https://github.com/sanamhub/nepdate-rust/blob/main/crates/nepdate/examples/convert.rs)
(`cargo run --example convert --features jiff -- 2081/04/15`) and
[`month_calendar`](https://github.com/sanamhub/nepdate-rust/blob/main/crates/nepdate/examples/month_calendar.rs)
(`cargo run --example month_calendar -- 2081 4`).

## Supported range

BS 1901-01-01 to 2199-12-30 (AD 1844-04-11 to 2143-04-15). Every function returns
`Error::OutOfRange` outside it. Month lengths for years beyond the officially published
calendar are projections inherited from NepDate and may change in a later release
([known data questions](https://github.com/sanamhub/nepdate/blob/main/data/README.md#known-data-questions-to-raise-upstream)).

## Features

| Feature | Default | What it adds |
|---|---|---|
| `std` | yes | `std::error::Error` for `Error`, and `to_string()` through `Display` |
| `serde` | no | `Serialize` and `Deserialize` for `NepaliDate` as `"YYYY-MM-DD"` |
| `chrono` | no | `From<NepaliDate> for chrono::NaiveDate` and `TryFrom<chrono::NaiveDate>` |
| `jiff` | no | `From<NepaliDate> for jiff::civil::Date`, `TryFrom<jiff::civil::Date>`, `NepaliDate::today()` (the date in `Asia/Kathmandu`), `today_in(&TimeZone)` and `is_today()`. Turns on `std` |

Without default features the crate builds for `wasm32-unknown-unknown` and other targets with no
standard library.

## Coming from C# NepDate

| C# | Rust |
|---|---|
| `new NepaliDate(y, m, d)` (throws) | `NepaliDate::new(y, m, d)` returns `Result<NepaliDate, Error>` and is a `const fn` |
| `NepaliDate.Parse(s)` / `TryParse` | `s.parse::<NepaliDate>()` (strict) or `NepaliDate::parse_lenient(s)` |
| `date.EnglishDate` | `date.to_gregorian()`, or `chrono::NaiveDate::from(date)` with feature `chrono` |
| `DateTime.ToNepaliDate()` | `NepaliDate::from_gregorian(y, m, d)`, or `NepaliDate::try_from(naive_date)` |
| `AddDays(n)` | `checked_add_days(n)` |
| `AddMonths(n, awayFromMonthEnd)` | `checked_add_months(n, MonthOverflow::Clamp)` or `MonthOverflow::Overflow` |
| `MonthEndDay` | `month_length()` |
| `DayOfWeek` | `weekday()` |
| `ToString(format)` | `Display` (`YYYY/MM/DD`), `short(..)`, `long(..)` and `format_pattern(pattern, lang)` |
| `IsToday` | `is_today()` with feature `jiff` |

The places where this crate deliberately behaves differently from C# NepDate are listed in
[PARITY.md](https://github.com/sanamhub/nepdate/blob/main/spec/PARITY.md).

## Minimum supported Rust version

Rust 1.85 (edition 2024). Raising it is a minor version bump.

## Data and license

The calendar data and conversion algorithm come from
[NepDate](https://github.com/RajuPrasai/NepDate) by Raju Prasai and contributors, used under the
MIT License. Its notice ships in this crate as `LICENSE-THIRD-PARTY`. The shared data and the
specification are maintained in [`sanamhub/nepdate`](https://github.com/sanamhub/nepdate).

This crate is MIT licensed (`LICENSE`). It is not affiliated with the Government of Nepal.

## More

| Topic | Where |
|---|---|
| What changed in each version | [CHANGELOG.md](https://github.com/sanamhub/nepdate-rust/blob/main/crates/nepdate/CHANGELOG.md) |
| Speed against C# NepDate and other crates | [benchmarks](https://github.com/sanamhub/nepdate-rust/blob/main/docs/benchmarks.md) |
| Reporting a vulnerability | [SECURITY.md](https://github.com/sanamhub/nepdate-rust/blob/main/SECURITY.md) |
| A wrong date, holiday or tithi | the hub's [data error form](https://github.com/sanamhub/nepdate/issues/new/choose) |
| The other languages | [sanamhub/nepdate](https://github.com/sanamhub/nepdate#packages) |
