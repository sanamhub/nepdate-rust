# L5: Patro crate `nepdate-patro`, and optional features `serde`, `jiff`, `chrono`

**Goal:** the separate crate `nepdate-patro` with tithi, public holidays and events, and the
opt-in interop features of the core, each tested on its own. This is PORT-PLAN phase 4.

Read first: ADR-0002, ADR-0005, ADR-0006, ADR-0011, `shared/spec/PATRO.md`, `docs/API.md`
(crate `nepdate-patro`, feature impls), hub
[ADR-0004](https://github.com/sanamhub/nepdate/blob/main/docs/adr/0004-patro-in-a-separate-package.md).
Human reads: `docs/learn/06-features-and-cfg.md`.

Task IDs: until 2026-10-07 this phase built a `meta` Cargo feature inside `nepdate`. L5-00 is
new. L5-01, L5-01b and L5-02 keep their IDs and now target the crate `nepdate-patro`.
L5-06 also gains the check that the core contains no patro data.

## Tasks

### L5-00 `nepdate-patro` crate skeleton

`crates/nepdate-patro/Cargo.toml`
```toml
[package]
name = "nepdate-patro"     # check crates.io availability first; if taken, STOP and ask the human
version = "0.0.0"           # always the same as crates/nepdate (ADR-0002); L6-04 checks it
description = "Tithi, Nepal public holidays and events per Bikram Sambat day, for the nepdate crate"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
categories = ["date-and-time", "no-std", "localization"]
keywords = ["nepali", "bikram-sambat", "tithi", "holidays", "nepal"]

[lib]
name = "nepdate_patro"

[dependencies]
# Same minor as this crate (hub ADR-0004 §3). L6-07 changes it to "~0.1.0".
nepdate = { path = "../nepdate", version = "~0.0.0", default-features = false }

[lints]
workspace = true
```

`crates/nepdate-patro/src/lib.rs` starts with a `//!` crate doc line, `#![no_std]` and
`#![forbid(unsafe_code)]`. No features, no `extern crate std`.
Add `"crates/nepdate-patro"` to workspace `members` and `default-members`. Create
`crates/nepdate-patro/CHANGELOG.md` with an empty `## [Unreleased]`.
In `.github/workflows/ci.yml`, the `msrv` job (L0-05) also builds `-p nepdate-patro`.
Do not touch `crates/nepdate/Cargo.toml`: the core never names this crate (ADR-0002).

AC:
- `cargo build -p nepdate-patro` and
  `cargo build -p nepdate-patro --target wasm32-unknown-unknown` succeed.
- `cargo tree -p nepdate-patro -e normal` lists `nepdate-patro` and `nepdate` and nothing else.

### L5-01 Codegen for the patro
Extend `cargo xtask codegen` to read `shared/data/patro/*.json` (sorted by year, then date) and
write `crates/nepdate-patro/src/generated/patro.rs`. Dedupe `(np, en)` event name pairs in
first-seen order. Put one `// <bs-year>-<month>` comment before each month's run of entries in the
per-day tables, for reviewability. Fail if the tithi range has a gap.
Serials are computed in xtask by re-implementing the ~10-line `to_serial` from ALGORITHM §4
over the JSON month lengths. Do **not** make xtask depend on `nepdate` or `nepdate-patro`, and
don't add a public serial API to the core.

Layout: dense per-day tables over the coverage, as ADR-0004 §6 decides (PATRO §3 allows them when
a benchmark shows they are faster; the binary-search layout measured 3.5 times slower on B7 and
6 times on B8). Every table is indexed by `o = serial - PATRO_FIRST`.

```rust
// crates/nepdate-patro/src/generated/patro.rs  (@generated)
pub(crate) const PATRO_FIRST: u32;                       // serial of COVERAGE_FIRST
pub(crate) const PATRO_DAYS: usize;                      // days in the coverage
pub(crate) const NO_TITHI: u8 = u8::MAX;
pub(crate) static DAY_TITHI: [u8; PATRO_DAYS];           // index into TITHI_NAMES, or NO_TITHI
pub(crate) static HOLIDAY_BITS: [u64; (PATRO_DAYS + 63) / 64]; // bit o set = public holiday
pub(crate) static HOLIDAY_OFFSETS: [u16; H];             // o of each holiday, ascending (for public_holidays())
pub(crate) static DAY_EVENTS: [u16; PATRO_DAYS + 1];     // CSR: events of day o are EVENT_IDS[DAY_EVENTS[o]..DAY_EVENTS[o + 1]]
pub(crate) static EVENT_IDS: [u16; X];                   // index into NAMES, file order per day
pub(crate) static NAMES: [(&str, &str); N];              // event names (np, en), deduplicated, first-seen order
pub(crate) static TITHI_NAMES: [(&str, &str); 32];       // (np, en), first-seen order
pub(crate) const COVERAGE_FIRST: (u16, u8, u8);          // first BS day with any data
pub(crate) const COVERAGE_LAST: (u16, u8, u8);           // last BS day with any data
```

`info(date)` is one subtraction, one range check and three loads, O(1), with no search and no
allocation. Names are returned as `&'static str` from `NAMES` and `TITHI_NAMES`, never copied;
events are an iterator over `EVENT_IDS[start..end]`. Use `static`, not `const`, for every array
(ADR-0004 §3). Expected sizes today, from the data on 2026-10-07:

| Constant | Value | Meaning |
|---|---|---|
| `PATRO_DAYS` | 2793 | 2077-01-01 ..= 2084-08-20 |
| tithi days | 2190 | entries of `DAY_TITHI` that are not `NO_TITHI`, 2078-01-02 ..= 2083-12-30 |
| H | 294 | public holidays, 2077-01-01 ..= 2084-08-18 |
| days with events | 1671 | `o` with `DAY_EVENTS[o] < DAY_EVENTS[o + 1]` |
| X | 3032 | events in total |
| N | 721 | distinct event name pairs (753 distinct pairs with the 32 tithi names, PATRO §3) |

Tables are about 40 KB (24 KB of it the `&str` pairs) plus about 59 KB of UTF-8 names (ADR-0004 §6). JSON is never read at
run time.

AC:
- Unit tests in `crates/nepdate-patro/src/lib.rs`: `PATRO_DAYS == 2793`; 2190 entries of
  `DAY_TITHI` are not `NO_TITHI`; `HOLIDAY_OFFSETS.len() == 294` and the number of set bits in
  `HOLIDAY_BITS` is 294; `DAY_EVENTS[PATRO_DAYS] == 3032 == EVENT_IDS.len()`; 1671 days have
  events; `NAMES.len() == 721`; every index in `EVENT_IDS` is `< NAMES.len()` and every tithi
  index is `< 32` or `NO_TITHI`. Put the cheap ones in `const _: () = assert!(..)` too.
- Running codegen twice produces byte-identical files. Changing one event name in a
  `shared/data/patro/*.json` file (on a scratch branch) makes `cargo xtask codegen --check` fail.
- An xtask unit test with a small in-memory input whose tithi range skips one day fails with a
  message naming the missing BS date.
- `crates/nepdate/src/generated/` contains only `calendar.rs`.

### L5-01b `cargo xtask data-status`
Prints the first and last BS date (and its AD date) of each data kind (tithi, holidays, events)
and the number of days from today to that AD date. Exits 1 when tithi or holiday data ends less
than **183 days** after today. "Today" is the UTC date, or the AD date given with
`--today YYYY-MM-DD` (for tests). It reads `shared/data/patro/*.json`, which equals the tables
compiled into `nepdate-patro` whenever `cargo xtask codegen --check` passes. Used by the widget
release gate (W4-00).

AC (with today's data, where tithi ends 2083-12-30 = AD 2027-04-13):
- `cargo xtask data-status --today 2026-10-01` exits 0 (194 days left).
- `cargo xtask data-status --today 2026-10-13` exits 1 (182 days left) and names tithi.

### L5-02 Patro API (`crates/nepdate-patro/src/lib.rs`)
Exactly the `nepdate-patro` section of `docs/API.md`, semantics from PATRO §2.
- The serial of a date is `NepaliDate::MIN.days_until(date)` (never negative). A holiday serial
  becomes a date with `NepaliDate::MIN.checked_add_days(s)`; use `.ok()` in a `filter_map`, never
  `unwrap`.
- `coverage()` returns a `const COVERAGE: NepaliDateRange` built from `COVERAGE_FIRST` and
  `COVERAGE_LAST` with the const-`match` pattern from the notes in `docs/API.md`
  (`NepaliDate::new` and `NepaliDateRange::new` are both `const fn`). The `panic!` arms run only at
  compile time, so a bad table fails the build. Put `#[allow(clippy::panic)]` on that const with
  a comment saying so.

Tests in `crates/nepdate-patro/tests/anchors.rs`:
- Every row of PATRO §4: tithi English name, holiday flag, English event names in order.
  For 2081-01-01, `Event::name(Lang::Ne)` of the first event is `नयाँ वर्ष`.
- `info` of 2078-01-01 and 2084-01-01 has no tithi; 2078-01-02 and 2083-12-30 have one.
- `coverage()` is 2077-01-01 ..= 2084-08-20.
- `public_holidays().count() == 294`, strictly ascending, first 2077-01-01, last
  (`next_back`) 2084-08-18.
- `info` for every day of `coverage().days()` returns without panicking, and
  `has_events() == (events().len() > 0)`.
- A doctest on `info` with the 2081-01-01 anchor.

### L5-03 `serde` feature
`serde = { version = "1", default-features = false, optional = true }`.
Serialize with `serializer.collect_str(&self.short(Ymd, Dash, true, En))`. In `no_std`
serde, `collect_str` is a required Serializer method, so this is the idiomatic no-alloc way.
Deserialize with a visitor that accepts `&str` and uses strict parsing (no allocation). Test
with `serde_json` (dev-dependency): `"\"2080-04-15\""` round-trips; `"\"2080-13-01\""` is an
error.

### L5-04 `jiff` feature
`jiff = { version = "0.2", optional = true }` (default features OK; implies `std`).
Implement conversions + `today()` + `is_today()` (ADR-0011).

AC:
- `today_in(tz)` equals `NepaliDate::try_from(jiff::Zoned::now().with_time_zone(tz.clone()).date())`
  (not a fixed date), and `today()` equals `today_in` with `Asia/Kathmandu`.
- PORT-PLAN phase 4 clock test, through the interop conversion:
  `"2024-07-30T18:30:00Z".parse::<jiff::Timestamp>()`, viewed in `jiff::tz::TimeZone::UTC`,
  gives BS 2081-04-15, and viewed in `Asia/Kathmandu` gives BS 2081-04-16.
- `today()` uses `Asia/Kathmandu` and `today_in(tz)` takes the zone (ADR-0011, revised
  2026-10-10, PORT-PLAN phase 4, PARITY D-14).

### L5-05 `chrono` feature
`chrono = { version = "0.4", default-features = false, optional = true }`. Conversions only.
AC: `NaiveDate` 2024-04-13 converts to BS 2081-01-01 and back; `NaiveDate` 1844-04-10 gives
`Err(Error::OutOfRange)`.

### L5-06 CI feature matrix and the patro-free core check
In `cargo xtask ci`, add, in this order: `cargo build -p nepdate --no-default-features --features serde`;
`cargo test -p nepdate --features jiff,chrono,serde`; `cargo test -p nepdate` (default features,
no patro crate in the build); `cargo test -p nepdate-patro`;
`cargo build -p nepdate-patro --target wasm32-unknown-unknown`;
`cargo xtask check-core-has-no-patro`.
`cargo hack` is **not** added (dependency budget); list the combinations explicitly.

`cargo xtask check-core-has-no-patro` (PORT-PLAN phase 4: the core's manifest has no dependency
on `nepdate-patro` and the built core doesn't contain `World Day Against Human Trafficking`):
1. Fails if `crates/nepdate/Cargo.toml` contains `nepdate-patro` or `nepdate_patro`.
2. Runs `cargo package -p nepdate --list --allow-dirty` and fails if any listed file that exists
   under `crates/nepdate/` contains the bytes `World Day Against Human Trafficking`.
3. Runs `cargo build -p nepdate` (dev profile) and fails if `target/debug/libnepdate.rlib`
   contains the bytes `World Day Against Human Trafficking`. The dev profile is used on purpose: with `lto = "fat"`
   the release rlib holds LLVM bitcode, where a string is not stored as plain bytes.
4. Positive control: runs `cargo build -p nepdate-patro` and fails if
   `target/debug/libnepdate_patro.rlib` does **not** contain `World Day Against Human Trafficking`, because then the
   search itself is broken.

AC:
- `cargo xtask check-core-has-no-patro` exits 0 on a clean checkout.
- On a scratch branch, adding `nepdate-patro` as a dev-dependency of `nepdate` makes step 1 fail,
  and adding a comment `// World Day Against Human Trafficking` to `crates/nepdate/src/lib.rs` makes step 2 fail. Revert.

### L5-07 C# catalogue rows (phase 4), zero allocation (B7, B8), thread safety
- `crates/nepdate-patro/tests/csharp_catalog.rs`: one `cs_` test per catalogue row with
  `phase` = 4 (`cargo xtask catalog-check 4 --list`). `CalendarDataTests` rows that test C#'s
  internal offsets assert the L5-01 table facts instead, with a one-line comment;
  `DateTimeExtensionsTests` rows test the `jiff` and `chrono` conversions. Add
  `cargo xtask catalog-check 4` to `cargo xtask ci`.
- `crates/nepdate-patro/tests/no_alloc.rs` (`harness = false`, same counter as L2-09): B7
  (`info(d)` reading the tithi name, holiday flag and every event name) and B8 (the same for every
  day of `d`'s month, through `NepaliDateRange::month(..)?.days()`), plus a full
  `public_holidays()` walk. Must allocate 0 bytes.
- `crates/nepdate-patro/tests/send_sync.rs`: `CalendarInfo`, `Events`, `Tithi`, `Event`, and the
  `public_holidays()` iterator type; plus 8 threads calling `info` for every day of the coverage
  and comparing with the single-thread result.

AC: `cargo xtask catalog-check 4` exits 0 (or the PR says H5 is not synced yet);
`cargo test -p nepdate-patro --test no_alloc` passes.

## Definition of Done
Global DoD. `cargo tree -p nepdate --no-default-features` shows zero dependencies.
`cargo tree -p nepdate-patro -e normal` shows only `nepdate`.

## Human review checklist
- [ ] Run `cargo tree -p nepdate --no-default-features` and then with `--features jiff`. Compare.
- [ ] Find a `#[cfg(feature = "...")]`. What happens to that code when the feature is off? (learn/06 §1)
- [ ] Why is patro data a separate crate and not a feature? (learn/06 §6, ADR-0005)
- [ ] Open `crates/nepdate-patro/src/generated/patro.rs` and look up today's date by its comment.

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes; the summary line is in the PR.
- [ ] `generated/patro.rs` has the L5-01 tables and nothing else; `cargo xtask codegen --check`
      is clean.
- [ ] `info` contains no `binary_search`, loop or allocation; names come out as `&'static str`.
- [ ] `tests/anchors.rs` covers every PATRO §4 row and the counts T, H, E.
- [ ] `cargo test -p nepdate-patro --test no_alloc` covers B7 and B8 and passes.
- [ ] `cargo xtask check-core-has-no-patro` exits 0, and the two scratch-branch failures in L5-06
      were tried (the PR says so).
- [ ] `cargo tree -p nepdate --no-default-features` shows no dependencies;
      `cargo tree -p nepdate-patro -e normal` shows only `nepdate`.
- [ ] `cargo xtask catalog-check 4` exits 0, or the PR says H5 is not synced yet.
- [ ] `send_sync.rs` in both crates lists every public type.
