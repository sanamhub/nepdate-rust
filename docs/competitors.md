# Other Rust crates for the Bikram Sambat calendar

Checked 2026-09-25 against the crates.io API, the crate sources and GitHub; versions rechecked
against the crates.io API on 2026-10-08 (no crate below has released since). Used by task L6-02
(benchmarks). No crate has reverse dependencies, so there is no established incumbent.

## Native support

Rust has no built-in Bikram Sambat calendar, so this repository builds its own:

- The standard library has no calendar types at all (`std::time` has only `Instant`,
  `SystemTime` and `Duration`).
- ICU4X `icu_calendar` 2.3.0 (2026-08-13, about 17.9 million downloads), the Unicode project's
  Rust calendar crate, has these calendars: Buddhist, Chinese, Coptic, Dangi, Ethiopian (two
  eras), Gregorian, Hebrew, Indian (the Indian national Saka calendar), Hijri (several variants),
  ISO, Japanese, Julian, Persian and ROC. There is no Nepali or Bikram Sambat calendar: its
  `AnyCalendarKind` enum has no such variant, and the crate source contains neither "Nepal",
  "Bikram" nor "Vikram" (source of `icu_calendar-2.3.0.crate` from static.crates.io,
  https://docs.rs/icu_calendar/2.3.0/icu_calendar/enum.AnyCalendarKind.html). This matches
  upstream ICU and CLDR, which have no Nepali calendar either.
- `chrono`, `time` and `jiff` are Gregorian only. `jiff` is still used for "today" and time zones
  (ADR-0011).

## Crates we benchmark against

| Crate (version pinned for benches) | License | BS range | Approach | Cost per conversion | Deps | `no_std` | State |
|---|---|---|---|---|---|---|---|
| `npdatetime =0.2.10` | MIT | 1975–2100 (table); "any" with `astronomical` | const table copied into a `lazy_static` `Vec` | O(years): month-by-month from 1975; AD→BS also walks Gregorian months | 5 (+ chrono with `std`) | no (docs claim it; code uses `String`, `std::time`) | active, young, frequent releases |
| `adtobs =0.2.0` | GPL-3.0 | 2000–2090 | hard-coded table | AD→BS walks **one day at a time** from 1944-01-01; BS→AD O(years) | 1 (chrono) | no | light but alive |
| `nepali-date-converter =0.1.1` | MIT | 1975–2099 | JSON parsed by serde_json on first use | O(years) scan | 4 (chrono, once_cell, serde, serde_json) | no | abandoned since 2026-02 |
| `nepdate-cli =0.1.5` (lib `bikram`) | GPL-3.0-or-later | 2000–2099 (table); silent astronomical fallback outside | table + `f64` Julian-day maths | O(years) scan; fallback searches day by day | 1 (chrono) | no | frozen; repo points to a location that 404s |

Calls to benchmark (exact API):

| Crate | BS → AD | AD → BS |
|---|---|---|
| `npdatetime` | `NepaliDate::new(y, m, d)?.to_gregorian()` → `(i32, u8, u8)` | `NepaliDate::from_gregorian(y, m, d)` |
| `adtobs` | `NepaliDate::from_bs_ymd(y, m, d)?.to_gregorian()` → `NaiveDate` (the constructor already converts once, for the weekday) | `NepaliDate::from_gregorian_ymd(y, m, d)` |
| `nepali-date-converter` | `bs_to_ad("YYYY-MM-DD")` (string API only; warm up first, label "includes string parsing") | `ad_to_bs("YYYY-MM-DD")` |
| `nepdate-cli` | `Bikram::new().to_gregorian(y, m, d)` | `let mut b = Bikram::new(); b.from_gregorian(y, m, d); (b.get_year(), b.get_month(), b.get_day())` |

## Correctness notes (why the table needs an "agrees" column)

Compared with NepDate's month lengths (our `shared/data/`):

- `nepali-date-converter`: rows for about BS 2064–2090 look shifted by one year (its 2065 row sums to
  364 days); more differences in 1977/81/89/93/98 and 2024; 2098 has 10 months; no day validation in
  `bs_to_ad` (day 0 underflows a `u32`); the weekday comes from an already reduced variable.
- `npdatetime`: 2082 reads `31,31,31,32,…` where NepDate and the others have `31,31,32,31,…`
  (Asar 32); 2062 and 2084–2087 differ too.
- `adtobs` and `nepdate-cli` disagree with each other on 2087–2090. NepDate's own 2087–2090 is also
  questionable (`shared/data/README.md`, "Known data questions").
- `nepdate-cli` validates nothing in `to_gregorian` and silently switches to approximate maths
  outside its table.

A speed table without correctness hides this, so every row also says whether the crate matches
NepDate on the benchmark's fixed dates.

## Not benchmarked

- `yorion_engine` (git only, v0.4.0, MIT OR Apache-2.0): BS 1975–2100 plus panchanga and festival
  rules; heavy deps (chrono, astro, suncalc, rrule, serde_json). Closest feature match to our `nepdate-patro`.
- `teistro-calendar 0.0.0` (`no_std`, Apache-2.0): placeholder that promises Bikram Sambat. Watch it.
- Placeholders or apps without a conversion API: `nepali-chrono 0.0.0`, `ndate 0.1.0` (2021),
  `ncal 0.3.0` (terminal `cal` clone), `npltz 1.1.7` (terminal patro app), `Patro`, `miti`.
- Not calendar crates despite the name: `nepali-core 0.1.0` (a programming language),
  `lekhai 0.0.1` (transliteration).

## Names

`nepdate`, `nepdate-patro` and `mitibar` were all free on crates.io on 2026-10-17
(the API returns "not found" for each; ADR-0017). L0's human checklist checks again before
publishing. khumnath already uses "nepdate" for a GPL-3.0 Qt desktop widget
(`github.com/khumnath/nepdate`) and the `nepdate-cli` crate, so the desktop app does not share
that name: it is MitiBar (ADR-0017). The app is `publish = false` anyway.

## Sources

crates.io API (`/api/v1/crates/<name>`, dependencies, reverse dependencies) for every crate above;
crate sources from static.crates.io;
https://github.com/4mritGiri/npdatetime, https://github.com/amrit073/adtobs-rs,
https://github.com/dhurbachy/Nepali-Date-Converter, https://github.com/khumnath/nepdate-cli,
https://github.com/khumnath/nepdate, https://github.com/Yorion-io/yorion_engine.
