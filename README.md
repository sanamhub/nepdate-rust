<p align="center">
  <img src="https://raw.githubusercontent.com/sanamhub/nepdate-rust/main/docs/assets/nepdate-rust-128.png" width="128" alt="NepDate for Rust logo">
</p>

<h1 align="center">nepdate-rust</h1>

<p align="center">
  <a href="https://github.com/sanamhub/nepdate-rust/actions/workflows/ci.yml"><img src="https://github.com/sanamhub/nepdate-rust/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/nepdate"><img src="https://img.shields.io/crates/v/nepdate?include_prereleases&logo=rust&label=nepdate" alt="nepdate on crates.io"></a>
  <a href="https://crates.io/crates/nepdate-patro"><img src="https://img.shields.io/crates/v/nepdate-patro?include_prereleases&logo=rust&label=nepdate-patro" alt="nepdate-patro on crates.io"></a>
  <a href="https://docs.rs/nepdate"><img src="https://img.shields.io/docsrs/nepdate?logo=docsdotrs" alt="docs.rs"></a>
  <a href="https://github.com/sanamhub/nepdate-rust/blob/main/docs/adr/README.md"><img src="https://img.shields.io/badge/MSRV-1.85-informational" alt="MSRV 1.85"></a>
  <a href="https://github.com/sanamhub/nepdate-rust/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License: MIT"></a>
</p>

Bikram Sambat (the Nepali calendar) for Rust.

The library converts between BS and AD for BS 1901-01-01 to 2199-12-30 (AD 1844-04-11 to
2143-04-15), formats and parses Nepali dates, does date arithmetic, inclusive ranges and fiscal
years, and ships the patro: tithi, public holidays and events. It is `no_std` with no dependencies
by default, its core operations allocate nothing, and it contains no `unsafe` code.

```rust
use nepdate::{Lang, NepaliDate};

fn main() -> Result<(), nepdate::Error> {
    let date = NepaliDate::new(2081, 4, 15)?;
    let ad = date.to_gregorian();
    assert_eq!((ad.year(), ad.month(), ad.day()), (2024, 7, 30));
    assert_eq!(date.weekday().name(Lang::En), "Tuesday");
    assert_eq!(date.year_length(), 366);
    println!("{}", date.long(nepdate::LongDate::default(), Lang::En));
    Ok(())
}
```

It prints `Shrawan 15, 2081`.

> **Status:** 1.0.0-rc.1, the release candidate for both crates on the 1.0 line. The API is stable
> from 1.0 (hub ADR-0009). Patro data (tithi, holidays, events) is informational, not an official
> calendar.

## Install

```toml
[dependencies]
nepdate = "=1.0.0-rc.1"
nepdate-patro = "=1.0.0-rc.1"   # only if you need tithi, holidays or events
```

Cargo never picks a pre-release for a plain `"0.2"` requirement, so the release candidate is
pinned exactly.

The MSRV is Rust 1.85. The `std` feature is on by default; `serde`, `chrono` and `jiff` are
optional. `std` turns on `alloc`, which adds ordered batch conversion and the ranges that return
`Vec`.

## The two crates

| Crate | Contents |
|---|---|
| `nepdate` | Conversion, arithmetic, fiscal year, formatting, parsing, ranges |
| `nepdate-patro` | Tithi, public holidays and events for BS 2077-01-01 to 2084-08-20 |

`nepdate-patro` depends on `nepdate`, never the other way round, so an app that only converts
dates carries no patro data. Both crates share one version and are released together.

## Dates, identity and comparison

`NepaliDate` is a valid BS date, four bytes and `Copy`. Every value is valid, so there is no
default value to check and no `IsDefault`.

```rust
use nepdate::{Month, NepaliDate};

fn main() -> Result<(), nepdate::Error> {
    let date = NepaliDate::new(2081, 4, 15)?;
    assert_eq!(date.year(), 2081);
    assert_eq!(date.month(), Month::Shrawan);
    assert_eq!(date.month_number(), 4);
    assert_eq!(date.day(), 15);
    assert_eq!(date.month_length(), 32);
    assert_eq!(date.year_length(), 366);
    assert_eq!(date.day_of_year(), 109);
    assert_eq!(date.last_day_of_month().to_string(), "2081/04/32");
    Ok(())
}
```

`MIN` is 1901-01-01 and `MAX` is 2199-12-30. Comparison, equality and hashing use the standard
traits. The operator `-` returns a `DateDiff` in the C# direction, `this - other`; the named
`days_until(other)` answers the other question, `other - this`.

## Conversion and serialization

```rust
use nepdate::{to_english_dates, to_nepali_dates, GregorianDate, NepaliDate};

fn main() -> Result<(), nepdate::Error> {
    let bs = NepaliDate::new(2081, 4, 15)?;
    let ad = bs.to_gregorian();
    assert_eq!((ad.year(), ad.month(), ad.day()), (2024, 7, 30));
    assert_eq!(NepaliDate::try_from(ad)?, bs);
    assert_eq!(bs.to_ymd_int(), 20810415);
    assert_eq!(NepaliDate::from_ymd_int(20810415)?, bs);

    let dates = to_nepali_dates([GregorianDate::new(2024, 7, 30)?])?;
    assert_eq!(dates, [bs]);
    assert_eq!(to_english_dates(dates)?[0], ad);
    Ok(())
}
```

`GregorianDate` is the minimal AD date the core needs; the optional `chrono` and `jiff` features
add their own conversions. The canonical text form is `YYYY-MM-DD`. With the `serde` feature a
`NepaliDate` serializes as that string and rejects anything else, including JSON `null`.

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let date = nepdate::NepaliDate::new(2081, 4, 15)?;
    let text = serde_json::to_string(&date)?;
    assert_eq!(text, "\"2081-04-15\"");
    assert_eq!(serde_json::from_str::<nepdate::NepaliDate>(&text)?, date);
    Ok(())
}
```

## Arithmetic

```rust
use nepdate::{MonthOverflow, NepaliDate};

fn main() -> Result<(), nepdate::Error> {
    let date = NepaliDate::new(2081, 4, 32)?;
    assert_eq!(date.checked_add_days(1)?.to_string(), "2081/05/01");
    assert_eq!(date.checked_add_months(2, MonthOverflow::Clamp)?.to_string(), "2081/06/30");
    assert_eq!(date.checked_add_months(2, MonthOverflow::Overflow)?.to_string(), "2081/07/02");
    assert_eq!(date.checked_add_years(1, MonthOverflow::Clamp)?.to_string(), "2082/04/31");
    assert_eq!(date.days_until(NepaliDate::MAX), 43_340);
    assert_eq!((date - NepaliDate::MIN).total_days, 65_871);
    Ok(())
}
```

Months and days are integers. A relative overflow either clamps to the target month's last day or
spills into the next month. `diff(other)` returns years, months and days with the clamp rule.

## Formatting

```rust
use nepdate::{DateOrder, Lang, LongDate, NepaliDate, Separator};

fn main() -> Result<(), nepdate::Error> {
    let date = NepaliDate::new(2081, 4, 15)?;
    assert_eq!(date.to_string(), "2081/04/15");
    assert_eq!(
        date.short(DateOrder::Dmy, Separator::Dash, true, Lang::En).to_string(),
        "15-04-2081"
    );
    assert_eq!(
        date.short(DateOrder::Ymd, Separator::Slash, true, Lang::Ne).to_string(),
        "२०८१/०४/१५"
    );
    assert_eq!(date.long(LongDate::default(), Lang::En).to_string(), "Shrawan 15, 2081");
    assert_eq!(date.format_pattern("dd.MM.yy", Lang::En).to_string(), "15.04.81");
    assert_eq!(date.format_pattern("yyyy\\", Lang::En).to_string(), "2081\\");
    Ok(())
}
```

The short form has six orders and six separators, with or without padding. The long form is
`[Weekday, ]Month day[, year]`. Patterns read `y`, `M` and `d` runs, single-quoted literals and
`\` escapes. A trailing `\` writes a literal `\`. Every formatter has `write_to` for a caller's
buffer and a `Display` implementation.

## Parsing

```rust
use nepdate::{Error, NepaliDate, ParseErrorKind};

fn main() -> Result<(), Error> {
    let date = NepaliDate::new(2081, 4, 15)?;
    assert_eq!("2081/04/15".parse::<NepaliDate>()?, date);
    assert_eq!(NepaliDate::try_parse("२०८१/०४/१५")?, date);
    assert_eq!(NepaliDate::parse_lenient("15 Shrawan 2081")?, date);
    assert_eq!(
        NepaliDate::try_parse_lenient("15 Shrawan 81"),
        Err(Error::Parse(ParseErrorKind::Ambiguous))
    );
    Ok(())
}
```

Strict parsing accepts year, month and day in that order with `- / . _ \ space | ।` separators,
ASCII and Devanagari digits, mixed, and at most four digits per group. Lenient parsing adds
day-month-year order, month names, `,`, era markers and the words `गते` and `मिति`. It never
guesses.

## Ranges

```rust
use nepdate::{DateOrder, Lang, NepaliDate, NepaliDateRange, Separator};

fn main() -> Result<(), nepdate::Error> {
    let range = NepaliDateRange::new(
        NepaliDate::new(2081, 4, 10)?,
        NepaliDate::new(2081, 4, 20)?,
    )?;
    assert_eq!(range.len(), 11);
    assert!(range.contains(NepaliDate::new(2081, 4, 15)?));
    assert_eq!(range.days().count(), 11);
    assert_eq!(range.to_string(), "2081/04/10 - 2081/04/20");
    assert_eq!(
        range.to_string_with(DateOrder::Dmy, Separator::Dash, Lang::En).to_string(),
        "10-04-2081 - 20-04-2081"
    );

    let month = NepaliDateRange::month(2081, 4)?;
    let day = NepaliDateRange::single(NepaliDate::new(2081, 4, 15)?);
    assert_eq!(month.intersect(day), Some(day));
    assert_eq!(month.except(day).len(), 2);
    assert_eq!(month.split_by_month().len(), 1);
    assert_eq!(month.split_by_fiscal_quarter().len(), 1);
    assert!(month.working_days(false).count() < month.len() as usize);
    assert!(month.weekend_days(true).count() >= month.weekend_days(false).count());
    assert_eq!(month.dates_with_interval(3)?.count(), 11);
    Ok(())
}
```

A range is inclusive and never empty, so `start > end` is refused. `except`, `split_by_month` and
`split_by_fiscal_quarter` return `Vec` and need the `alloc` feature. At the data end the day
helpers stop after 2199-12-30, the splits return their last piece, and adjacency returns `false`
instead of throwing.

## Fiscal year

```rust
use nepdate::{FiscalYear, Lang, NepaliDate, Quarter};

fn main() -> Result<(), nepdate::Error> {
    let fy = NepaliDate::new(2083, 2, 10)?.fiscal_year();
    assert_eq!(fy, FiscalYear::new(2082));
    assert_eq!(fy.start()?.to_string(), "2082/04/01");
    assert_eq!(fy.end()?.to_string(), "2083/03/32");
    assert_eq!(fy.quarter_range(Quarter::Q1)?.end().to_string(), "2082/06/31");
    assert_eq!(fy.label(Lang::En).to_string(), "2082/83");
    Ok(())
}
```

## Patro

```rust
use nepdate::{Lang, NepaliDate};

fn main() -> Result<(), nepdate::Error> {
    let date = NepaliDate::new(2081, 1, 1)?;
    let info = nepdate_patro::info(date);
    assert_eq!(info.tithi().map(|t| t.name(Lang::En)), Some("Panchami"));
    assert!(info.is_public_holiday());
    let events: Vec<&str> = info.events().map(|e| e.name(Lang::En)).collect();
    assert_eq!(events, ["New Year", "Mesh Sankranti", "Bisket Jatra"]);
    Ok(())
}
```

## Errors

One `Error` type covers construction, conversion, arithmetic, ranges and parsing. It implements
`core::error::Error` in every build, so `?` works without `std`.

```rust
use nepdate::{Error, NepaliDate};

fn main() {
    assert_eq!(NepaliDate::new(1900, 1, 1), Err(Error::OutOfRange));
    assert_eq!(NepaliDate::new(2081, 13, 1), Err(Error::InvalidMonth(13)));
    assert_eq!(
        NepaliDate::new(2081, 4, 33),
        Err(Error::InvalidDay { month_length: 32, day: 33 })
    );
}
```

## MitiBar

MitiBar, the tray calendar, moved to its own repository:
[`sanamhub/mitibar`](https://github.com/sanamhub/mitibar). Its releases, issues and documentation
live there. The `mitibar-v0.1.0-beta.1` release and tag were removed from this repository.

## Repository

```
crates/nepdate/        core library
crates/nepdate-patro/  tithi, holidays, events
shared/                read-only copy of the hub's data and spec
xtask/                 code generation, checks and release helpers
docs/                  API, ADRs, design notes, benchmarks and runbooks
```

## More

| Topic | Where |
|---|---|
| Public API of both crates | [docs/API.md](https://github.com/sanamhub/nepdate-rust/blob/main/docs/API.md), [docs.rs/nepdate](https://docs.rs/nepdate), [docs.rs/nepdate-patro](https://docs.rs/nepdate-patro) |
| Why the libraries are built this way | [docs/adr](https://github.com/sanamhub/nepdate-rust/blob/main/docs/adr/README.md) |
| Library speed against C# NepDate and other crates | [docs/benchmarks.md](https://github.com/sanamhub/nepdate-rust/blob/main/docs/benchmarks.md) |
| What changed in each version | [nepdate](https://github.com/sanamhub/nepdate-rust/blob/main/crates/nepdate/CHANGELOG.md), [nepdate-patro](https://github.com/sanamhub/nepdate-rust/blob/main/crates/nepdate-patro/CHANGELOG.md) |
| Building, testing and pull requests | [CONTRIBUTING.md](https://github.com/sanamhub/nepdate-rust/blob/main/CONTRIBUTING.md) |
| Reporting a vulnerability | [SECURITY.md](https://github.com/sanamhub/nepdate-rust/blob/main/SECURITY.md) |
| Guides and examples | [nepdate.com/docs/rust](https://nepdate.com/docs/rust/) |
| A wrong date, holiday or tithi | the hub's [data error form](https://github.com/sanamhub/nepdate/issues/new/choose) |
| The algorithm, data and conformance vectors | [shared/spec](https://github.com/sanamhub/nepdate-rust/blob/main/shared/spec/README.md) |
| The other languages | [sanamhub/nepdate](https://github.com/sanamhub/nepdate#packages) |

## Credits and license

Calendar data and algorithm come from [NepDate](https://github.com/RajuPrasai/NepDate) by Raju
Prasai and contributors, MIT. See [`shared/data/NOTICE`](https://github.com/sanamhub/nepdate-rust/blob/main/shared/data/NOTICE).

This repository is MIT ([`LICENSE`](https://github.com/sanamhub/nepdate-rust/blob/main/LICENSE)). Not
affiliated with the Government of Nepal. Holiday data is informational.
