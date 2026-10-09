# `nepdate` and `nepdate-patro`: public API contract (v0.1)

This is the complete public surface of both crates for v0.1. Implement exactly this: nothing
more, nothing renamed. Adding public items needs a spec update in the same PR, approved by the
human. Semantics refer to `shared/spec/ALGORITHM.md`, `FORMATTING.md`, `PARSING.md` and
`PATRO.md` (same folder). Conventions: ADR-0007. Features: ADR-0005 / ADR-0011. Why the patro
is a separate crate: ADR-0005 and hub ADR-0004.

## Crate `nepdate` (`crates/nepdate`)

```rust
#![no_std]
#![forbid(unsafe_code)]

// ---------- lib.rs re-exports ----------
pub use crate::date::NepaliDate;
pub use crate::gregorian::GregorianDate;
pub use crate::names::{Lang, Month, Weekday};
pub use crate::error::{Error, ParseErrorKind};
pub use crate::arith::{MonthOverflow, DateDiff};
pub use crate::format::{DateOrder, Separator, LongDate, ShortFormat, LongFormat, PatternFormat, Digits};
pub use crate::fiscal::{FiscalYear, Quarter, FiscalLabel};
pub use crate::range::{NepaliDateRange, Days};
// No patro data here: tithi, holidays and events live in the crate `nepdate-patro` (below).

// ---------- NepaliDate ----------
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NepaliDate { /* year: u16, month: Month, day: u8, private (ADR-0004 §2) */ }
// size_of::<NepaliDate>() == 4 and size_of::<Option<NepaliDate>>() == 4 (tested, not promised in rustdoc)

impl NepaliDate {
    pub const MIN: NepaliDate;                               // 1901-01-01
    pub const MAX: NepaliDate;                               // 2199-12-30
    pub const fn new(year: u16, month: u8, day: u8) -> Result<Self, Error>;
    pub fn from_gregorian(year: i32, month: u8, day: u8) -> Result<Self, Error>;
    pub fn parse_lenient(s: &str) -> Result<Self, Error>;

    pub const fn year(self) -> u16;
    pub const fn month(self) -> Month;
    pub const fn month_number(self) -> u8;                   // 1..=12
    pub const fn day(self) -> u8;
    pub fn weekday(self) -> Weekday;
    pub fn day_of_year(self) -> u16;                         // 1-based
    pub fn month_length(self) -> u8;                         // 29..=32
    pub fn first_day_of_month(self) -> Self;
    pub fn last_day_of_month(self) -> Self;
    pub fn to_gregorian(self) -> GregorianDate;

    pub fn checked_add_days(self, days: i32) -> Result<Self, Error>;
    pub fn checked_add_months(self, months: i32, overflow: MonthOverflow) -> Result<Self, Error>;
    pub fn checked_add_years(self, years: i32, overflow: MonthOverflow) -> Result<Self, Error>;
    pub fn days_until(self, other: Self) -> i32;             // other - self, signed
    pub fn diff(self, other: Self) -> DateDiff;              // ALGORITHM §8a
    pub fn succ(self) -> Option<Self>;                       // next day
    pub fn pred(self) -> Option<Self>;                       // previous day

    pub fn fiscal_year(self) -> FiscalYear;                  // may be FiscalYear(1900) for 1901-01..03
    pub fn quarter(self) -> Quarter;

    // formatting (never allocate; each returns a formatter value, see "Formatter values" below)
    pub const fn short(self, order: DateOrder, sep: Separator, pad: bool, lang: Lang) -> ShortFormat;
    pub const fn long(self, opts: LongDate, lang: Lang) -> LongFormat;
    pub const fn format_pattern(self, pattern: &str, lang: Lang) -> PatternFormat<'_>;

    #[cfg(feature = "jiff")] pub fn today() -> Result<Self, Error>;   // Nepal date: Asia/Kathmandu, or fixed +05:45 if tzdata lacks it
    #[cfg(feature = "jiff")] pub fn today_in(tz: &jiff::tz::TimeZone) -> Result<Self, Error>; // date in tz; OutOfRange outside the supported range
    #[cfg(feature = "jiff")] pub fn is_today(self) -> bool;          // self == today()

    pub(crate) fn to_serial(self) -> u32;                    // ALGORITHM §4 (not public)
    pub(crate) fn from_serial(s: u32) -> Result<Self, Error>;
}
impl core::fmt::Display for NepaliDate {}   // "YYYY/MM/DD"
impl core::fmt::Debug   for NepaliDate {}   // "NepaliDate(2081-04-15)"
impl core::str::FromStr for NepaliDate { type Err = Error; }   // strict, PARSING §1
impl TryFrom<GregorianDate> for NepaliDate { type Error = Error; }
impl From<NepaliDate> for GregorianDate {}
// feature "serde": Serialize/Deserialize as "YYYY-MM-DD" (deserialize = strict parse)
// feature "jiff":   From<NepaliDate> for jiff::civil::Date; TryFrom<jiff::civil::Date> for NepaliDate
// feature "chrono": From<NepaliDate> for chrono::NaiveDate; TryFrom<chrono::NaiveDate> for NepaliDate

// ---------- GregorianDate (minimal AD date, so the core needs no dependency) ----------
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct GregorianDate { /* year: i32, month: u8, day: u8, private, always valid */ }
impl GregorianDate {
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, Error>;   // real Gregorian date (any year)
    pub const fn year(self) -> i32;  pub const fn month(self) -> u8;  pub const fn day(self) -> u8;
    pub fn weekday(self) -> Weekday;
}
impl Display for GregorianDate {}   // "YYYY-MM-DD"

// ---------- names ----------
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Lang { #[default] En, Ne }
impl Lang { pub const fn digits(self, n: u32, min_width: u8) -> Digits; }  // localised number

#[repr(u8)] #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Month { Baishakh = 1, Jestha, Ashad, Shrawan, Bhadra, Ashoj, Kartik, Mangsir, Poush, Magh, Falgun, Chaitra }
impl Month {
    pub const ALL: [Month; 12];
    pub const fn number(self) -> u8;
    pub fn name(self, lang: Lang) -> &'static str;           // FORMATTING §1
    pub fn short_name(self, lang: Lang) -> &'static str;
    pub fn next(self) -> Month;  pub fn prev(self) -> Month; // wrapping
}
impl TryFrom<u8> for Month { type Error = Error; }           // Error::InvalidMonth

#[repr(u8)] #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Weekday { Sunday = 0, Monday, Tuesday, Wednesday, Thursday, Friday, Saturday }
impl Weekday {
    pub const ALL: [Weekday; 7];                             // Sunday first
    pub const fn index_from_sunday(self) -> u8;
    pub fn name(self, lang: Lang) -> &'static str;
    pub fn short_name(self, lang: Lang) -> &'static str;
}

// ---------- errors ----------
#[derive(Debug, Clone, Copy, PartialEq, Eq)] #[non_exhaustive]
pub enum Error {
    OutOfRange,
    InvalidMonth(u8),
    InvalidDay { month_length: u8, day: u8 },
    InvalidGregorian,                 // e.g. 2023-02-29
    Parse(ParseErrorKind),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)] #[non_exhaustive]
pub enum ParseErrorKind { InvalidCharacter, WrongGroupCount, NumberTooLong, Ambiguous, Unrecognized }
impl Display for Error {}   #[cfg(feature = "std")] impl std::error::Error for Error {}

// ---------- arithmetic ----------
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MonthOverflow { #[default] Clamp, Overflow }        // ALGORITHM §8
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DateDiff { pub negative: bool, pub years: u16, pub months: u8, pub days: u8, pub total_days: i32 }

// ---------- format options ----------
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DateOrder { #[default] Ymd, Ydm, Myd, Mdy, Dym, Dmy }
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Separator { #[default] Slash, Backslash, Dot, Underscore, Dash, Space }
impl Separator { pub const fn as_char(self) -> char; }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LongDate { pub weekday: bool, pub year: bool, pub pad: bool }
impl Default for LongDate { /* weekday: false, year: true, pad: true */ }

// ---------- formatter values (ADR-0004 §5) ----------
// Each is a small Copy + Debug struct with private fields. Display::fmt calls write_to(f).
// write_to is generic, so a caller with its own buffer gets static dispatch (about 4x faster
// than going through fmt::Formatter, ADR-0004 measurements). Neither allocates.
#[derive(Clone, Copy, Debug)] pub struct ShortFormat { /* date, order, sep, pad, lang */ }
#[derive(Clone, Copy, Debug)] pub struct LongFormat { /* date, opts, lang */ }
#[derive(Clone, Copy, Debug)] pub struct PatternFormat<'a> { /* date, pattern: &'a str, lang */ }
#[derive(Clone, Copy, Debug)] pub struct Digits { /* n, min_width, lang */ }
#[derive(Clone, Copy, Debug)] pub struct FiscalLabel { /* fiscal year, lang */ }
// For each of the five types T:
impl T { pub fn write_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result; }
impl core::fmt::Display for T {}

// ---------- fiscal ----------
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FiscalYear(u16);           // starting BS year: FY 2082/83 = FiscalYear(2082)
impl FiscalYear {
    pub const fn new(start_year: u16) -> Self;
    pub const fn start_year(self) -> u16;
    pub fn containing(date: NepaliDate) -> Self;
    pub fn start(self) -> Result<NepaliDate, Error>;
    pub fn end(self) -> Result<NepaliDate, Error>;
    pub fn range(self) -> Result<NepaliDateRange, Error>;
    pub fn quarter_range(self, q: Quarter) -> Result<NepaliDateRange, Error>;
    pub const fn label(self, lang: Lang) -> FiscalLabel;      // "2082/83" / "२०८२/८३"
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Quarter { Q1, Q2, Q3, Q4 }
impl Quarter { pub const fn months(self) -> [Month; 3]; }

// ---------- range ----------
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NepaliDateRange { /* start <= end, inclusive */ }
impl NepaliDateRange {
    pub const fn new(start: NepaliDate, end: NepaliDate) -> Option<Self>;   // None if start > end.
    // const so that nepdate-patro can build coverage() at compile time. Compare
    // (year, month, day) field by field: trait methods like `>` are not allowed in a const fn.
    pub fn month(year: u16, month: u8) -> Result<Self, Error>;
    pub fn year(year: u16) -> Result<Self, Error>;
    pub const fn start(self) -> NepaliDate;  pub const fn end(self) -> NepaliDate;
    pub fn len(self) -> u32;                                  // >= 1
    pub fn contains(self, date: NepaliDate) -> bool;
    pub fn days(self) -> Days;                                // iterator
}
// Deliberately not in v0.1 (YAGNI): overlaps, intersection, union, split by month.
impl IntoIterator for NepaliDateRange { type Item = NepaliDate; type IntoIter = Days; }
#[derive(Clone, Debug)] pub struct Days { /* serial cursor */ }
impl Iterator for Days { type Item = NepaliDate; }            // + DoubleEndedIterator + ExactSizeIterator + FusedIterator
```

## Crate `nepdate-patro` (`crates/nepdate-patro`, lib `nepdate_patro`)

Depends on `nepdate` only (`default-features = false`), has no features and no other
dependencies, and is `#![no_std]` with no allocation, like the core. The core knows nothing
about it (hub ADR-0004). Semantics: `shared/spec/PATRO.md` §2. Hub ADR-0004 names a type `Patro`
"where the language needs a type"; Rust doesn't: the crate path `nepdate_patro::` plays that
role, so the queries are free functions. Every lookup is O(1) over the per-day tables of
ADR-0004 §6 and every name is a `&'static str`.

```rust
#![no_std]
#![forbid(unsafe_code)]

use nepdate::{Lang, NepaliDate, NepaliDateRange};
// CalendarInfo, Events, Tithi and Event are defined at the crate root.

pub fn info(date: NepaliDate) -> CalendarInfo;               // never fails; no data = empty info; O(1)
pub fn coverage() -> NepaliDateRange;                        // 2077-01-01..=2084-08-20 today
pub fn public_holidays() -> impl DoubleEndedIterator<Item = NepaliDate> + ExactSizeIterator;   // ascending, over HOLIDAY_OFFSETS

#[derive(Clone, Copy, Debug)] pub struct CalendarInfo { /* tithi: u8, holiday: bool, events: (u16, u16): 6 bytes, private */ }
impl CalendarInfo {
    pub fn tithi(self) -> Option<Tithi>;
    pub fn is_public_holiday(self) -> bool;
    pub fn events(self) -> Events;                            // file order
    pub fn has_events(self) -> bool;
}
#[derive(Clone, Debug)] pub struct Events { /* range into EVENT_IDS, private */ }
impl Iterator for Events { type Item = Event; }               // + DoubleEndedIterator + ExactSizeIterator + FusedIterator
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Tithi { /* u8 index into TITHI_NAMES, private */ }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Event { /* u16 index into NAMES, private */ }
impl Tithi { pub fn name(self, lang: Lang) -> &'static str; }  // Ne → np, En → en
impl Event { pub fn name(self, lang: Lang) -> &'static str; }
```

The crate reaches the serial day through the public core API only:
`NepaliDate::MIN.days_until(date)` is the serial of `date` (ALGORITHM §4), and
`NepaliDate::MIN.checked_add_days(serial)` turns a generated serial back into a date. No
`pub` serial API is added to the core for it.

Changes on 2026-10-08 (ADR-0004 rewrite), so reviewers of older notes are not surprised:
`NepaliDate` stores `month: Month` (niche, 4-byte `Option`); `short`, `long`, `format_pattern`,
`Lang::digits` and `FiscalYear::label` return the named formatter types with `write_to` instead
of `impl Display`, and became `const fn` (they only copy their arguments);
`CalendarInfo::events` returns the named iterator `Events`.

Thread safety: every public type of both crates is `Send + Sync` because it is plain data and the
tables are immutable statics. Tests assert it (ADR-0014).

Notes for implementers:

- Formatter values write `&'static str` pieces only. Never build bytes and call
  `str::from_utf8` (about 100 ns for a Nepali date, ADR-0004), never `unsafe`.
- Every `pub fn` gets `#[must_use]` (except `Iterator::next`), a doc comment, and a doctest.
- `const fn` where marked is required (keeps compile-time construction possible, e.g.
  `const NEW_YEAR: NepaliDate = match NepaliDate::new(2081, 1, 1) { Ok(d) => d, Err(_) => panic!() };`).
