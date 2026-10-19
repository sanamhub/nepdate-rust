# ADR-0004: Date representation, tables and lookups (performance design)

- **Status:** accepted
- **Date:** 2026-09-24 (rewritten 2026-10-08 before publication, as ADR-0001 allows: measured performance
  design for BENCHMARK.md B1 to B8, patro layout, allocation-free formatting)
- **Approved by:** Sanam

## Context

`shared/spec/BENCHMARK.md` §6 sets the Rust target: faster than C# NepDate on B1, B2 and B7, with
zero bytes allocated. The maintainer raised it to every operation B1 to B8, in both crates. Speed has to
come from data layout and data access, so this ADR records what was measured, not assumed.

What C# NepDate does (read at commit `cb05cb65`):

- `NepaliDate` holds three `int`s plus a cached `DateTime?` (about 32 bytes). Every constructor
  converts to AD at once.
- BS to AD: `NepaliToEnglish.data[(y - 1901) * 12 + m - 1]`, a 3588-entry array of
  `(int, int, int, int)` tuples with the AD date of the month's last day, then
  `new DateTime(..).AddDays(day - monthLength)`.
- AD to BS: `EnglishToNepali.data[...]`, 3589 tuples with the BS date of the AD month's last day,
  then a loop that walks back over BS months (`SubtractNepaliDays`).
- Patro: sorted `ushort` day offsets per kind, `Array.BinarySearch`, names in a pool of
  `(string Np, string En)`. `GetCalendarInfo`, `EventsNp` and `EventsEn` allocate new `string[]`
  arrays on every call.
- All tables are `static readonly` arrays built by a type initializer on first use.

The ALGORITHM spec says how results are computed; "how a port stores the tables is its own
decision" (`shared/spec/ALGORITHM.md`, preamble).

## Measurements

Experiments ran on the reference machine (i5-12500H, Windows 11) with rustc 1.99.0
(`x86_64-pc-windows-gnu`, `lto = "fat"`, `codegen-units = 1`) and .NET 8.0.31, using
`std::time::Instant` and `std::hint::black_box` over the 64 dates of BENCHMARK §2, best of
7 runs of 200,000 rounds. Other jobs were running on the machine, so treat the numbers as rough
ratios, not as the published benchmark. "Quiet" is a run pinned to logical
CPU 2 with little other load; "busy" is an unpinned run under load. The experiment code was
throwaway and is not in this repository; the published benchmark rebuilds the measurement.

Every variant below agreed with every other on all 109,212 days, in both directions.

| Op | Variant | Quiet ns | Busy ns |
|---|---|---|---|
| B1 BS to AD | V1: `PACKED_YEARS` + `YEAR_START`, month loop, Hinnant `civil_from_days` (the earlier design) | 32.3 | 27.3 |
| B1 | V2: one-load month table `MONTH_INFO`, Hinnant (signed `i32` division) | 24.0 | 19.2 |
| B1 | V2 with Neri-Schneider `civil_from_days` | 7.2 | 13.8 |
| B1 | V3: C#'s method (AD date of month end, then day arithmetic) | 32.6 | 47.3 |
| B1 | **V4: `BS_MONTH_AD` table (AD date of each BS month start) + one carry** | **6.3** | **11.9** |
| B2 AD to BS | V1: Hinnant + `partition_point` over years + month loop | 36.4 | 53.5 |
| B2 | V2: Hinnant + month estimate | 15.6 | 21.1 |
| B2 | V2 with Neri-Schneider `days_from_civil` | 10.4 | 15.1 |
| B2 | V3: C#'s method (AD month table + walk back) | 6.4 | 12.7 |
| B2 | **V4: `AD_MONTH_START` table + month estimate** | **7.1** | **13.1** |
| B3 +1000 days | V1 | 31.8 | 47.3 |
| B3 | **V2/V4** | **6.4** | **11.4** |
| B4 +13 months | V1 (`PACKED_YEARS`) | 5.0 | 7.7 |
| B4 | **V2/V4 (`MONTH_INFO`)** | **2.3** | **7.1** |
| B5 long, Nepali, weekday | `write!` through `fmt::Formatter`, digit by digit | 176.7 | 172.4 |
| B5 | build bytes, `str::from_utf8`, one `write_str` | 150.3 | 148.8 |
| B5 | `write_str` per piece through `fmt::Formatter` | not run | 123.0 |
| B5 | **same pieces, `write_str` on a concrete `W: fmt::Write` (generic, no `dyn`)** | not run | **29.7** |
| B5 | `str::from_utf8` alone, 60 bytes of Devanagari | not run | 100.9 |
| B6 strict parse | byte scanner over `s.as_bytes()` | 22.8 | 31.1 |
| B7 patro info | sorted serials + `binary_search` (the earlier design) | 28.2 | 41.9 |
| B7 | **per-day tables (tithi byte, holiday bitset, event offsets)** | **8.1** | **12.1** |
| B7 | one packed `u32` record per day | 6.1 | 8.6 |
| B7 | per-day tables + one string blob with offsets | 12.1 | 16.5 |
| B8 month grid | binary search | 1640 | 1254 |
| B8 | **per-day tables** | **209** | **285** |
| B8 | packed record | 386 | 233 |

Same harness shape in C# (`Action<int>` per call, unpinned, busy, NepDate source at `cb05cb65`
compiled for .NET 8): B1 38.4 ns, B2 34.3 ns, B3 35.9 ns, B4 32.5 ns, B5 480 ns and 435 B,
B6 79.2 ns, B7 133.6 ns and 51 B, B8 7334 ns and 1429 B. The published C# column comes from the
hub benchmark with BenchmarkDotNet; these numbers only show that the chosen design has a wide
margin.

The hub has since published that column (hub file `docs/benchmarks/dotnet.md`,
.NET 10, BenchmarkDotNet, pinned to logical CPU 2, rerun 2026-10-09 with no other jobs). The Rust targets
below are what the published benchmark must show on the reference machine with `criterion`. Each is at least 1.6 times
faster than C# except B6 (1.08 times), and none is looser than the Go port's pinned prototype on the same machine
(`sanamhub/nepdate-go` ADR-0002 §8: B1 6.8, B2 5.6, B3 4.6, B4 4.2, B5 30.6, B6 17.8, B7 6.7, B8 226 ns),
since Rust and C++ are meant to be the fastest ports:

| ID | Operation | Rust quiet (best variant) | Rust target ns/op | bytes/op | C# .NET 10 ns/op | C# bytes/op |
|---|---|---:|---:|---:|---:|---:|
| B1 | BS to AD | 6.3 | ≤ 8 | 0 | 13.29 | 0 |
| B2 | AD to BS | 7.1 | ≤ 8 | 0 | 18.64 | 0 |
| B3 | add 1000 days | 6.4 | ≤ 8 | 0 | 17.50 | 0 |
| B4 | add 13 months, Clamp | 2.3 | ≤ 8 | 0 | 18.85 | 0 |
| B5 | long Nepali date with weekday | 29.7 (busy) | ≤ 40 | 0 | 153.02 | 435 |
| B6 | strict parse | 22.8 | ≤ 20 | 0 | 21.55 | 0 |
| B7 | patro info of a day | 8.1 | ≤ 10 | 0 | 47.93 | 51 |
| B8 | month grid with patro | 209 | ≤ 250 | 0 | 3133 | 1429 |

B6 is the one target below the quiet measurement (Go's byte parser does 17.8 ns). If the published benchmark measures it above 20 ns, the fix order is: one
pass over the bytes with no second validation step, then a fixed-width fast path for
`YYYY/MM/DD`; report it before anything else.

For comparison, the C++ port's prototype (`sanamhub/nepdate-cpp`, `docs/adr/0002-representation-and-performance.md`
§8, zig/Clang, same machine and pinning) measured B1 at 4.6 to 4.8 ns with Neri–Schneider and B3 at
3.6 to 4.6 ns. That is the same range as V4 here, so the remaining gap between the two languages is
harness noise, not design.

Two findings drove the design. First, Hinnant's algorithm in signed `i32` costs 15 to 17 ns here,
because signed division by a constant needs extra correction steps; the Neri-Schneider form
([Neri and Schneider 2023](https://doi.org/10.1002/spe.3172), also used by the
[`datealgo`](https://docs.rs/datealgo) crate, which reports 3.2 ns against Hinnant's 7.6 ns for
day-to-date) is 2 to 3 times faster, and a table lookup is faster still. Second, every call through
`fmt::Formatter` is a dynamic call; for an 11-piece Nepali date that costs about 90 ns, and
validating a built buffer with `str::from_utf8` costs about 100 ns.

## Decision

### 1. One integer timeline: the serial day

`serial` = days since 1901-01-01 BS (serial 0 = 1844-04-11 AD), `0..=109_211`
(ALGORITHM §2). Every conversion and all arithmetic go through it.

### 2. The value type

```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NepaliDate { year: u16, month: Month, day: u8 } // 4 bytes
```

- `Month` is the public `#[repr(u8)]` enum with discriminants 1 to 12. The 244 unused byte
  values give the compiler a niche, so `Option<NepaliDate>` and `Result<NepaliDate, Error>` are
  4 bytes too (measured: `Option` of the old `{u16, u8, u8}` layout is 6 bytes). Values travel in
  one register. Niche use for such enums is how rustc lays out data today but is not a language
  guarantee ([`core::option` representation](https://doc.rust-lang.org/core/option/index.html#representation)
  lists only the guaranteed cases), so it is a unit test, not a promise in the docs.
- Fields are private; every value is valid. Field order makes the derived `Ord` chronological.
- `(y, m, d)` is stored, not the serial: getters and formatting are free, and the serial is one
  table load away (§4). Storing both would make the type 8 bytes to save one load in B3.
- Nothing is cached in the value. C# caches the AD date (making its struct about 32 bytes); here
  AD is one or two table loads.
- Same choice as `jiff::civil::Date` (separate small fields). `chrono::NaiveDate` and `time::Date`
  pack year and day-of-year into one `NonZero<i32>`; that suits an unbounded proleptic calendar,
  where month tables are impossible, not a 299-year table calendar.
- Plain data with no interior mutability, so every public type is `Send + Sync + Unpin` (a test
  asserts it).

### 3. Tables

All tables are `static` items in read-only data, computed **at compile time**. Rust has no code
before `main` and evaluates a `static` initializer at compile time
([Rust reference: static items](https://doc.rust-lang.org/reference/items/static-items.html)),
so the first call costs the same as the millionth. C#'s type initializer, by contrast, builds its
tuple arrays on first use. `static`, not `const`, for the runtime tables: a `const` is inlined at
each use ([constant items](https://doc.rust-lang.org/reference/items/constant-items.html)), so a
large `const` array indexed at run time may be copied per use site.

Only `PACKED_YEARS` is generated by `cargo xtask codegen` (ADR-0006). The others are built from it
by `const fn`s in `crates/nepdate/src/calendar.rs`, so there is one source of truth and nothing
for codegen to keep in sync.

| Table | Type | Bytes | Content |
|---|---|---|---|
| `PACKED_YEARS` | `const [u32; 299]` | 1,196 | 2 bits per month: `length - 29`. Generated. `const` so that `const fn NepaliDate::new` can read it |
| `MONTH_INFO` | `static [u32; 3589]` | 14,356 | entry `i` = month `i` (`(y - 1901) * 12 + m - 1`): `start_serial << 2 \| (length - 29)`; entry 3588 = `109_212 << 2` |
| `BS_MONTH_AD` | `static [u32; 3588]` | 14,352 | AD date of each BS month's first day, `year << 9 \| month << 5 \| day` |
| `AD_MONTH_START` | `static [i32; 3589]` | 14,356 | serial of the first day of each AD month, 1844-04 (index 0, value −10) to 2143-04 (index 3588) |

Total 44.3 KB. A test binary grew by 43,008 bytes when the last three were added (measured).
C#'s two conversion tables are about 187 KB (47 % of its 397 KB DLL).

The `const fn` builders use only `while` loops and reads of `const` items (never of a `static`),
so they rely on no recent const-evaluation feature; the MSRV job proves it. Each builder that needs AD dates calls ALGORITHM §5 (Hinnant, in `i64`) at compile
time. A `const _: () = assert!(...)` checks `MONTH_INFO[3588] >> 2 == 109_212` and
`days_from_civil(1844, 4, 11) == -45_920` in every build.

### 4. Algorithms

| Operation | Steps | Branches that depend on data |
|---|---|---|
| `new(y, m, d)` | range checks, month length from `PACKED_YEARS` | none |
| `to_serial` | `MONTH_INFO[i] >> 2` + `day - 1` | none |
| `from_serial(s)` | `i = min((s * 137_797) >> 22, 3587)`; if `MONTH_INFO[i] >> 2 > s` then `i -= 1`, else if `MONTH_INFO[i + 1] >> 2 <= s` then `i += 1`; `y = 1901 + i / 12`, `m = i % 12 + 1`, `d = s - start + 1` | at most one correction (the estimate is never off by more than one month: checked for every serial by a unit test) |
| `to_gregorian` | unpack `BS_MONTH_AD[i]`, add `day - 1`, carry into the next AD month at most once | one |
| `from_gregorian(y, m, d)` | Gregorian validity (`InvalidGregorian`); `y` in 1844..=2143 and index in 0..3589 else `OutOfRange`; `s = AD_MONTH_START[idx] + d - 1`; `s` in `0..=109_211` else `OutOfRange`; `from_serial(s)` | none beyond the checks |
| `weekday` | `(to_serial + 4) % 7` | none |
| `checked_add_days(n)` | `from_serial(to_serial + n)` with `i64` range check | as `from_serial` |
| `checked_add_months` | ALGORITHM §8 with the length from `MONTH_INFO` | Clamp/Overflow |
| `GregorianDate::weekday` (any year) | ALGORITHM §5 `days_from_civil` in `i64` at run time | not a hot path |

`137_797 = round(2^22 / (109_212 / 3_588))`, the average BS month length in fixed point. If
the data ever changes so that the estimate is off by two, the unit test fails and the fix is a
second correction step, not a different design.

The results are those of ALGORITHM §4 to §6 exactly; only the storage differs. The exhaustive test
The exhaustive test compares the table path with a direct transcription of ALGORITHM §4 and §5 for every
serial.

### 5. Formatting and parsing never allocate, and never go through `dyn` per piece

- Every formatter (`short`, `long`, `format_pattern`, `Lang::digits`, `FiscalYear::label`) returns
  a small public `Copy` type (`docs/API.md`) with one method:
  `pub fn write_to<W: fmt::Write + ?Sized>(&self, out: &mut W) -> fmt::Result`. It writes
  `&'static str` pieces (names, separators, and Devanagari digits from a `[&str; 10]` table) with
  `out.write_str`. `Display::fmt` is `self.write_to(f)`. Callers on a hot path call `write_to`
  with their own buffer (any `fmt::Write`: `String`, `heapless::String`, `arrayvec::ArrayString`,
  or a hand-written stack buffer) and get the static-dispatch speed (B5 row above).
- No formatter builds bytes and then calls `str::from_utf8`, and none uses `unsafe` to skip it
  (`#![forbid(unsafe_code)]`, ADR-0005).
- Parsers read `s.as_bytes()`. Devanagari digits are the 3-byte sequences `E0 A5 A6` to
  `E0 A5 AF`, matched as bytes. Groups accumulate into `u32` with a digit counter. No `String`,
  no `Vec`, no `char` decoding for ASCII input.

### 6. Patro layout (`nepdate-patro`)

All tables are generated by `cargo xtask codegen` into
`crates/nepdate-patro/src/generated/patro.rs` and indexed by `o = serial - PATRO_FIRST`, where
`PATRO_FIRST` is the serial of the first day with any data (2077-01-01) and `PATRO_DAYS` the
number of days in the coverage (2793 today).

| Table | Type | Bytes today | Content |
|---|---|---|---|
| `DAY_TITHI` | `[u8; PATRO_DAYS]` | 2,793 | index into `TITHI_NAMES`, `NO_TITHI = u8::MAX` when the day has none |
| `HOLIDAY_BITS` | `[u64; (PATRO_DAYS + 63) / 64]` | 352 | bit `o` set = public holiday |
| `HOLIDAY_OFFSETS` | `[u16; 294]` | 588 | `o` of each holiday, ascending; only for the `public_holidays()` iterator |
| `DAY_EVENTS` | `[u16; PATRO_DAYS + 1]` | 5,588 | CSR offsets: the events of day `o` are `EVENT_IDS[DAY_EVENTS[o]..DAY_EVENTS[o + 1]]` |
| `EVENT_IDS` | `[u16; 3032]` | 6,064 | index into `NAMES`, file order per day |
| `NAMES` | `[(&str, &str); 721]` | 23,072 + text | event names `(np, en)`, deduplicated, first-seen order |
| `TITHI_NAMES` | `[(&str, &str); 32]` | 1,024 + text | tithi names `(np, en)` |

`info(date)` is one subtraction, one range check and three loads, O(1) for every kind. B8 (a month
grid) is about 30 of those. The `&str` table costs about 18 KB more than one string blob with
`u32` offsets (measured: 95 KB against 77 KB added to a test binary) and is about 4 ns faster per
call, because slicing a blob at run time needs bounds and UTF-8 boundary checks. Names are
returned as `&'static str`, never copied.

## Consequences

- B1, B2 and B3 are a handful of loads and integer operations, with no loops over data; B7 is O(1).
  On the busy machine the chosen variants measured 2.5 to 4.6 times faster than C# on B1 to B4
  and B6, 16 times on B5 (with `write_to`) and 11 times on B7.
- Nothing allocates in either crate. Every B-operation gets a zero-allocation test with a counting
  global allocator (ADR-0014).
- The core's read-only data is about 45 KB, against about 187 KB of tables in C#. The patro's is
  about 40 KB of tables plus about 59 KB of UTF-8 names.
- The ALGORITHM spec stays the reference for results. A port that has no compile-time evaluation
  can still copy this layout by generating the derived tables.

## Alternatives considered

- **Earlier design (V1):** `PACKED_YEARS` + `YEAR_START`, loops over months, binary search over years,
  Hinnant at run time. 2.4 KB of tables, but 4 to 5 times slower on B1 to B3. Rejected.
- **Neri-Schneider arithmetic instead of the two AD tables:** 28 KB smaller, 1 to 3 ns slower on
  B1 and B2. Kept as the fallback if binary size ever matters more than these nanoseconds.
- **C#'s two tables (V3):** B2 is as fast as V4, but B1 needs Gregorian arithmetic and AD to BS
  walks back over months; two tables of 4-byte tuples hold the same facts as `MONTH_INFO`.
  Rejected.
- **Dense tables with one entry per day (BS to AD and back):** 109,212 entries × 4 bytes × 2 =
  874 KB, 20 times the core tables, for at most 2 ns: V4 already does one month-table load plus one
  carry, and the C++ port, which rejected the same table, measured about 5 ns without it. Rejected.
  (Dense per-day tables are used where they pay: the patro, §6, at 2,793 days.)
- **`include_bytes!` of a packed blob:** zero start-up cost too, but every read needs
  `u32::from_le_bytes` on a slice with bounds checks, and the blob is opaque to a reader. Rejected.
- **Storing the serial (4 bytes) or serial plus fields (8 bytes):** B3 saves one load; getters and
  formatting pay a decomposition, `Debug` is unreadable, and 8 bytes loses the single-register
  return. Rejected.
- **Patro with sorted serials and binary search** (an earlier design and PATRO §3's suggestion):
  3.5 times slower on B7 and 6 times on B8. Rejected. **One packed `u32` per day:** about 2 ns
  faster on B7, but larger (11.2 KB against 8.7 KB) and harder to read. Rejected.
- **An existing crate:** none matches NepDate's range and data, C# parity, or `no_std` without
  dependencies (`docs/competitors.md`). Rejected.
