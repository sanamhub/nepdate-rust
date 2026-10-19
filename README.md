# nepdate-rust

[![CI](https://github.com/sanamhub/nepdate-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/sanamhub/nepdate-rust/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Bikram Sambat (the Nepali calendar) for Rust, plus MitiBar, a small tray calendar for Windows,
macOS and Linux.

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

> **Status:** `nepdate` and `nepdate-patro` are 0.1.0-ready but not published yet. MitiBar builds
> and runs on Windows; its 0.1.0 release waits for a quiet idle measurement and a manual pass on
> macOS and Linux.

## Install

Once 0.1.0 is on crates.io:

```toml
[dependencies]
nepdate = "0.1"
nepdate-patro = "0.1"
```

Until then, use the repository:

```toml
[dependencies]
nepdate = { git = "https://github.com/sanamhub/nepdate-rust" }
nepdate-patro = { git = "https://github.com/sanamhub/nepdate-rust" }
```

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

MitiBar keeps today's BS date in the tray, with a month grid, tithi, public holidays, a converter
and settings. It opens from the tray or the Windows 11 taskbar and closes when it loses focus. It
makes no network requests.

```bash
cargo run -p mitibar --release
```

On Linux it needs a tray (StatusNotifierItem or the GNOME AppIndicator extension), fontconfig,
xkbcommon and D-Bus. The renderer is software only, so no OpenGL or GPU driver is needed. The
release workflow builds an NSIS installer and portable zip on Windows, a universal `.dmg` on macOS
and an `.AppImage` and `.deb` on Linux.

## Repository

```
crates/nepdate/        core library
crates/nepdate-patro/  tithi, holidays, events
apps/mitibar/          the desktop app
shared/                read-only copy of the hub's data and spec
xtask/                 code generation, checks and release helpers
docs/                  API, ADRs, design notes, benchmarks and runbooks
```

## Documentation

| Link | Contents |
|---|---|
| [`docs/API.md`](docs/API.md) | Public API of both crates |
| [`docs/adr/`](docs/adr/README.md) | Why the libraries and the app are built this way |
| [`docs/design/`](docs/design/widget-requirements.md) | MitiBar requirements and design tokens |
| [`docs/benchmarks.md`](docs/benchmarks.md) | Library benchmarks against C# NepDate and other crates |
| [`docs/benchmarks-widget.md`](docs/benchmarks-widget.md) | MitiBar idle memory, CPU and cold start |
| [`shared/spec/`](shared/spec/README.md) | Algorithm, data and conformance vectors, shared by every port |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Building, testing and pull request rules |

## Credits and license

Calendar data and algorithm come from [NepDate](https://github.com/RajuPrasai/NepDate) by Raju
Prasai and contributors, MIT. See [`shared/data/NOTICE`](shared/data/NOTICE). MitiBar follows
[NepDateWidget](https://github.com/RajuPrasai/NepDateWidget), with its author's permission
([record](docs/legal/README.md)).

This repository is MIT ([`LICENSE`](LICENSE)). MitiBar uses Slint under its royalty-free licence;
the bundled Noto fonts are under the SIL Open Font License. Not affiliated with the Government of
Nepal. Holiday data is informational.
