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
2143-04-15), formats and parses Nepali dates, does date arithmetic and fiscal years, and ships the
patro: tithi, public holidays and events. It is `no_std` with no dependencies by default, allocates
nothing, and contains no `unsafe` code.

```rust
use nepdate::{Lang, NepaliDate};

fn main() -> Result<(), nepdate::Error> {
    let date = NepaliDate::new(2081, 4, 15)?;
    let ad = date.to_gregorian();
    assert_eq!((ad.year(), ad.month(), ad.day()), (2024, 7, 30));
    assert_eq!(date.weekday().name(Lang::En), "Tuesday");
    println!("{}", date.long(nepdate::LongDate::default(), Lang::En));
    Ok(())
}
```

It prints `Shrawan 15, 2081`.

> **Status:** 0.1.0-beta.1, the first public pre-release of both crates. The API may still change
> before 0.1.0. Patro data (tithi, holidays, events) is informational, not an official calendar.

## Install

```toml
[dependencies]
nepdate = "=0.1.0-beta.1"
nepdate-patro = "=0.1.0-beta.1"   # only if you need tithi, holidays or events
```

Cargo never picks a pre-release for a plain `"0.1"` requirement, so the beta is pinned exactly.

The MSRV is Rust 1.85. The `std` feature is on by default; `serde`, `chrono` and `jiff` are
optional.

## The two crates

| Crate | Contents |
|---|---|
| `nepdate` | Conversion, arithmetic, fiscal year, formatting, parsing, ranges |
| `nepdate-patro` | Tithi, public holidays and events for BS 2077-01-01 to 2084-08-20 |

`nepdate-patro` depends on `nepdate`, never the other way round, so an app that only converts
dates carries no patro data. Both crates share one version and are released together.

```rust
use nepdate::{Lang, NepaliDate};

fn main() -> Result<(), nepdate::Error> {
    let date = NepaliDate::new(2081, 1, 1)?;
    let info = nepdate_patro::info(date);
    assert_eq!(info.tithi().map(|t| t.name(Lang::En)), Some("Panchami"));
    assert!(info.is_public_holiday());
    Ok(())
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
| Public API of both crates | [docs.rs/nepdate](https://docs.rs/nepdate), [docs.rs/nepdate-patro](https://docs.rs/nepdate-patro), [docs/API.md](https://github.com/sanamhub/nepdate-rust/blob/main/docs/API.md) |
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
