<p align="center">
  <img src="https://raw.githubusercontent.com/sanamhub/nepdate-rust/main/docs/assets/nepdate-rust-128.png" width="96" alt="NepDate logo">
</p>

<h1 align="center">nepdate-patro</h1>

<p align="center">
  <a href="https://crates.io/crates/nepdate-patro"><img src="https://img.shields.io/crates/v/nepdate-patro?include_prereleases&logo=rust" alt="nepdate-patro on crates.io"></a>
  <a href="https://docs.rs/nepdate-patro"><img src="https://img.shields.io/docsrs/nepdate-patro?logo=docsdotrs" alt="docs.rs"></a>
  <a href="https://github.com/sanamhub/nepdate-rust/actions/workflows/ci.yml"><img src="https://github.com/sanamhub/nepdate-rust/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/sanamhub/nepdate-rust/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License: MIT"></a>
</p>

The patro (पात्रो, the Nepali almanac) for the [`nepdate`](https://crates.io/crates/nepdate)
crate: tithi, the Nepal public holiday flag and events for each Bikram Sambat day. `no_std`, no
heap allocation, no `unsafe`. Every lookup is O(1) over per-day tables compiled into the crate,
and every name is a `&'static str`.

> **Status:** 0.1.0-beta.1, the first public pre-release. The API may still change before 0.1.0.
> Holiday data is informational, not an official calendar.

## Quick start

```toml
[dependencies]
nepdate = "=0.1.0-beta.1"
nepdate-patro = "=0.1.0-beta.1"
```

```rust
use nepdate::{Lang, NepaliDate};

let info = nepdate_patro::info(NepaliDate::new(2081, 1, 1)?);
assert_eq!(info.tithi().map(|t| t.name(Lang::En)), Some("Panchami"));
assert!(info.is_public_holiday());
assert_eq!(info.events().next().map(|e| e.name(Lang::En)), Some("New Year"));
assert_eq!(info.events().next().map(|e| e.name(Lang::Ne)), Some("नयाँ वर्ष"));

// Every public holiday in the data, in order.
let first = nepdate_patro::public_holidays().next();
assert!(first.is_some_and(|d| nepdate_patro::coverage().contains(d)));
# Ok::<(), nepdate::Error>(())
```

`info` never fails: a day without data, inside or outside the coverage, gets an empty info (no
tithi, not a holiday, no events).

## Data coverage

| Data | From | To |
|---|---|---|
| Days with data (`coverage()`) | BS 2077-01-01 | BS 2084-08-20 |
| Tithi | BS 2078-01-02 | BS 2083-12-30 |
| Public holidays | BS 2077-01-01 | BS 2084-08-18 |

Coverage grows through patch releases when the shared data is updated. Holidays the government
announces late are missing until the next data update.

## Versions

This crate's version always equals `nepdate`'s. Both are released together from one tag, and
`nepdate-patro` X.Y.Z depends on `nepdate` `~X.Y.0`. A pre-release such as 0.1.0-beta.1 depends
on exactly that `nepdate` pre-release (`=0.1.0-beta.1`), so one beta of this crate never runs
with another beta of the core. Use the same version for both.

## Data, provenance and license

The tithi, holiday and event data come from
[NepDate](https://github.com/RajuPrasai/NepDate) by Raju Prasai and contributors, used under the
MIT License. Its notice ships in this crate as `LICENSE-THIRD-PARTY`. NepDate has not published
where its tithi, holiday and event data came from. That question is open with its author
([known data questions](https://github.com/sanamhub/nepdate/blob/main/data/README.md#known-data-questions-to-raise-upstream)).

Holiday data is informational. This crate is not affiliated with the Government of Nepal; check
the government's official notice before relying on a holiday.

This crate is MIT licensed (`LICENSE`). Semantics of every field:
[PATRO.md](https://github.com/sanamhub/nepdate/blob/main/spec/PATRO.md).

## More

| Topic | Where |
|---|---|
| What changed in each version | [CHANGELOG.md](https://github.com/sanamhub/nepdate-rust/blob/main/crates/nepdate-patro/CHANGELOG.md) |
| Speed against C# NepDate and other crates | [benchmarks](https://github.com/sanamhub/nepdate-rust/blob/main/docs/benchmarks.md) |
| Reporting a vulnerability | [SECURITY.md](https://github.com/sanamhub/nepdate-rust/blob/main/SECURITY.md) |
| A wrong date, holiday or tithi | the hub's [data error form](https://github.com/sanamhub/nepdate/issues/new/choose) |
| The other languages | [sanamhub/nepdate](https://github.com/sanamhub/nepdate#packages) |
