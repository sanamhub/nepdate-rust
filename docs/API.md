# `nepdate` and `nepdate-patro`: public API (0.1)

This is the public surface of both crates for 0.1. A new public item needs a spec change, an ADR,
and a line in this file. Semantics refer to `shared/spec/ALGORITHM.md`, `FORMATTING.md`,
`PARSING.md` and `PATRO.md` (same folder). Conventions: ADR-0007. Features: ADR-0005 and
ADR-0011. Why the patro is a separate crate: ADR-0005 and hub ADR-0004.

The crate is `#![no_std]`. The default `std` feature turns on `alloc`, heap-using helpers and
`std::error::Error` through `core::error::Error`. The `serde`, `chrono` and `jiff` features are
optional. Everything that returns a formatter value writes through `core::fmt::Write` and never
allocates.

## Crate `nepdate`

```rust
#![no_std]
#![forbid(unsafe_code)]

pub use crate::arith::{DateDiff, MonthOverflow};
pub use crate::date::NepaliDate;
pub use crate::error::{Error, ParseErrorKind};
pub use crate::fiscal::{FiscalLabel, FiscalYear, Quarter};
pub use crate::format::{
    DateOrder, Digits, LongDate, LongFormat, PatternFormat, Separator, ShortFormat,
};
pub use crate::gregorian::GregorianDate;
pub use crate::names::{Lang, Month, Weekday};
pub use crate::range::{DatesWithInterval, Days, NepaliDateRange, RangeFormat};

#[cfg(feature = "alloc")]
pub use crate::batch::{to_english_dates, to_english_dates_from_strings, to_nepali_dates};
```

## 1. Dates, identity and comparison

`NepaliDate` is a valid BS date from 1901-01-01 to 2199-12-30, four bytes and `Copy`. Invalid
dates cannot be built, so there is no default value and no `IsDefault`.

```rust
use nepdate::{Month, NepaliDate};

let date = NepaliDate::new(2081, 4, 15)?;
assert_eq!(date.year(), 2081);
assert_eq!(date.month(), Month::Shrawan);
assert_eq!(date.month_number(), 4);
assert_eq!(date.day(), 15);
assert_eq!(date.month_length(), 32);
assert_eq!(date.year_length(), 366);
assert_eq!(date.day_of_year(), 109);
assert_eq!(date.to_string(), "2081/04/15");
Ok::<(), nepdate::Error>(())
```

```rust
use nepdate::NepaliDate;
pub const MIN: NepaliDate; // 1901-01-01
pub const MAX: NepaliDate; // 2199-12-30

impl NepaliDate {
    pub const fn new(year: u16, month: u8, day: u8) -> Result<Self, Error>;
    pub fn from_gregorian(year: i32, month: u8, day: u8) -> Result<Self, Error>;
    pub fn from_ymd_int(ymd: i32) -> Result<Self, Error>;
    pub fn try_parse(s: &str) -> Result<Self, Error>;
    pub fn parse_lenient(s: &str) -> Result<Self, Error>;
    pub fn try_parse_lenient(s: &str) -> Result<Self, Error>;

    pub const fn year(self) -> u16;
    pub const fn month(self) -> Month;
    pub const fn month_number(self) -> u8;
    pub const fn day(self) -> u8;
    pub fn weekday(self) -> Weekday;
    pub fn day_of_year(self) -> u16;
    pub fn month_length(self) -> u8;
    pub fn year_length(self) -> u16;
    pub fn first_day_of_month(self) -> Self;
    pub fn last_day_of_month(self) -> Self;
    pub fn to_gregorian(self) -> GregorianDate;
    pub fn to_ymd_int(self) -> i32;

    pub fn checked_add_days(self, days: i32) -> Result<Self, Error>;
    pub fn checked_add_months(self, months: i32, overflow: MonthOverflow) -> Result<Self, Error>;
    pub fn checked_add_years(self, years: i32, overflow: MonthOverflow) -> Result<Self, Error>;
    pub fn days_until(self, other: Self) -> i32;
    pub fn diff(self, other: Self) -> DateDiff;
    pub fn succ(self) -> Option<Self>;
    pub fn pred(self) -> Option<Self>;

    pub fn fiscal_year(self) -> FiscalYear;
    pub fn quarter(self) -> Quarter;

    pub const fn short(self, order: DateOrder, sep: Separator, pad: bool, lang: Lang) -> ShortFormat;
    pub const fn long(self, opts: LongDate, lang: Lang) -> LongFormat;
    pub const fn format_pattern(self, pattern: &str, lang: Lang) -> PatternFormat<'_>;

    #[cfg(feature = "jiff")] pub fn today() -> Result<Self, Error>;
    #[cfg(feature = "jiff")] pub fn today_in(tz: &jiff::tz::TimeZone) -> Result<Self, Error>;
    #[cfg(feature = "jiff")] pub fn is_today(self, tz: &jiff::tz::TimeZone) -> bool;
    #[cfg(feature = "jiff")] pub fn is_yesterday(self, tz: &jiff::tz::TimeZone) -> bool;
    #[cfg(feature = "jiff")] pub fn is_tomorrow(self, tz: &jiff::tz::TimeZone) -> bool;
}

impl Display for NepaliDate {}      // "YYYY/MM/DD"
impl Debug for NepaliDate {}        // "NepaliDate(2081-04-15)"
impl FromStr for NepaliDate {}      // strict, PARSING §1
impl TryFrom<GregorianDate> for NepaliDate {}
impl From<NepaliDate> for GregorianDate {}
impl Sub for NepaliDate { type Output = DateDiff; }   // this - other
```

The clock methods are gated by `jiff` because the crate carries no time zone database. `today()`
uses `Asia/Kathmandu` or a fixed UTC+05:45 when the database lacks it. `today_in` takes an
explicit zone; the three relative predicates take the same explicit zone and return `false` when
the clock is outside the supported range. `Now` is not a separate member: it is the same date as
`today(zone)`.

```rust
use jiff::tz::TimeZone;
use nepdate::NepaliDate;

let date = NepaliDate::new(2081, 4, 16)?;
assert!(date.is_yesterday(&TimeZone::UTC) || date.is_today(&TimeZone::UTC));
Ok::<(), nepdate::Error>(())
```

Comparison and equality use `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash`. The operator `-`
returns a `DateDiff` in the C# direction, `this - other`; the named `days_until` keeps the
opposite direction, `other - this` (D-07).

## 2. Conversion and serialization

```rust
use nepdate::{GregorianDate, NepaliDate};

let bs = NepaliDate::new(2081, 4, 15)?;
let ad = bs.to_gregorian();
assert_eq!((ad.year(), ad.month(), ad.day()), (2024, 7, 30));
assert_eq!(NepaliDate::try_from(ad)?, bs);
assert_eq!(NepaliDate::from_gregorian(2024, 7, 30)?, bs);
assert_eq!(bs.to_ymd_int(), 20810415);
assert_eq!(NepaliDate::from_ymd_int(20810415)?, bs);
Ok::<(), nepdate::Error>(())
```

`GregorianDate` is a minimal always-valid AD date so the core needs no date dependency.

```rust
use nepdate::NepaliDate;
pub struct GregorianDate;
impl GregorianDate {
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, Error>;
    pub const fn year(self) -> i32;
    pub const fn month(self) -> u8;
    pub const fn day(self) -> u8;
    pub fn weekday(self) -> Weekday;
}
impl Display for GregorianDate {}   // "YYYY-MM-DD"
```

Ordered batch conversion keeps the input order. It needs the `alloc` feature, which `std` turns
on by default.

```rust
use nepdate::{to_english_dates, to_nepali_dates, GregorianDate, NepaliDate};

let ad = [GregorianDate::new(2024, 7, 30)?];
let bs = to_nepali_dates(ad)?;
assert_eq!(bs, [NepaliDate::new(2081, 4, 15)?]);
assert_eq!(to_english_dates(bs)?[0], ad[0]);
Ok::<(), nepdate::Error>(())
```

```rust
use core::borrow::Borrow;

pub fn to_nepali_dates<I, T>(dates: I) -> Result<Vec<NepaliDate>, Error>
where I: IntoIterator<Item = T>, T: Borrow<GregorianDate>;

pub fn to_english_dates<I, T>(dates: I) -> Result<Vec<GregorianDate>, Error>
where I: IntoIterator<Item = T>, T: Borrow<NepaliDate>;

pub fn to_english_dates_from_strings<I, S>(strings: I) -> Result<Vec<GregorianDate>, Error>
where I: IntoIterator<Item = S>, S: AsRef<str>;
```

Rust has no function overloading, so the string form has its own name. The result is the same
ordered batch. C# `useParallel` and `batchSize` do not cross into this API.

`serde` is behind the `serde` feature: one string, `"YYYY-MM-DD"`, on the way in and out. The
object form, Newtonsoft, XML and the converter type are not ported (D-11).

```rust
#[cfg(feature = "serde")]
{
let date = nepdate::NepaliDate::new(2081, 4, 15)?;
let json = serde_json::to_string(&date)?;
assert_eq!(json, "\"2081-04-15\"");
assert_eq!(serde_json::from_str::<nepdate::NepaliDate>(&json)?, date);
}
Ok::<(), Box<dyn std::error::Error>>(())
```

## 3. Arithmetic

```rust
use nepdate::{MonthOverflow, NepaliDate};

let date = NepaliDate::new(2081, 4, 32)?;
assert_eq!(date.checked_add_days(1)?.to_string(), "2081/05/01");
assert_eq!(date.checked_add_months(2, MonthOverflow::Clamp)?.to_string(), "2081/06/30");
assert_eq!(date.checked_add_months(2, MonthOverflow::Overflow)?.to_string(), "2081/07/02");
assert_eq!(date.checked_add_years(1, MonthOverflow::Clamp)?.to_string(), "2082/04/31");
assert_eq!(date.days_until(NepaliDate::MAX), 109_211 - 105);
Ok::<(), nepdate::Error>(())
```

Months and days are integers (D-02, D-03). A relative overflow either clamps to the target
month's last day or spills into the next month. `AddYears` is `12 * years` with the same choice.

`days_until(other)` is `other - self`. `date - other` is `self - other` and returns `DateDiff`.
`diff(other)` reports years, months and days with `MonthOverflow::Clamp`.

```rust
use nepdate::{DateDiff, MonthOverflow, NepaliDate};
pub enum MonthOverflow { Clamp, Overflow }

pub struct DateDiff {
    pub negative: bool,
    pub years: u16,
    pub months: u8,
    pub days: u8,
    pub total_days: i32,
}

impl NepaliDate {
    pub fn checked_add_days(self, days: i32) -> Result<Self, Error>;
    pub fn checked_add_months(self, months: i32, overflow: MonthOverflow) -> Result<Self, Error>;
    pub fn checked_add_years(self, years: i32, overflow: MonthOverflow) -> Result<Self, Error>;
    pub fn days_until(self, other: Self) -> i32;
    pub fn diff(self, other: Self) -> DateDiff;
    pub fn succ(self) -> Option<Self>;
    pub fn pred(self) -> Option<Self>;
}
```

## 4. Formatting

```rust
use nepdate::{DateOrder, Lang, LongDate, NepaliDate, Separator};

let date = NepaliDate::new(2081, 4, 15)?;
assert_eq!(date.to_string(), "2081/04/15");
assert_eq!(date.short(DateOrder::Dmy, Separator::Dash, true, Lang::En).to_string(), "15-04-2081");
assert_eq!(date.short(DateOrder::Ymd, Separator::Slash, true, Lang::Ne).to_string(), "२०८१/०४/१५");
assert_eq!(date.long(LongDate::default(), Lang::En).to_string(), "Shrawan 15, 2081");
assert_eq!(date.format_pattern("dd.MM.yy", Lang::En).to_string(), "15.04.81");
assert_eq!(date.format_pattern("yyyy\\", Lang::En).to_string(), "2081\\");
Ok::<(), nepdate::Error>(())
```

The short form has six orders and six separators, with or without padding. The long form is
`[Weekday, ]Month day[, year]`. `format_pattern` reads `y`, `M` and `d` runs, single-quoted
literal text and `\` escapes; a trailing `\` writes a literal `\`. The Unicode forms are the same
formatters with `Lang::Ne`.

Every formatter returns a small `Copy` value with `write_to`, so a caller with a buffer gets no
allocation. `Display` writes through the same path.

```rust
use nepdate::{DateOrder, Lang, LongDate, NepaliDate, Separator};
pub enum DateOrder { Ymd, Ydm, Myd, Mdy, Dym, Dmy }
pub enum Separator { Slash, Backslash, Dot, Underscore, Dash, Space }
pub struct LongDate { pub weekday: bool, pub year: bool, pub pad: bool }

pub struct ShortFormat;
pub struct LongFormat;
pub struct PatternFormat<'a>;
pub struct Digits;
impl ShortFormat { pub fn write_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result; }
impl LongFormat { pub fn write_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result; }
impl PatternFormat<'_> { pub fn write_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result; }
impl Digits { pub fn write_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result; }
```

`Lang::digits(n, min_width)` returns a `Digits` value. `Month` and `Weekday` implement `Display`
with their English names; `name(lang)` and `short_name(lang)` give both languages.

```rust
use nepdate::{Lang, Month, Weekday};
pub enum Lang { En, Ne }
impl Lang {
    pub const fn digits(self, n: u32, min_width: u8) -> Digits;
}

pub enum Month { Baishakh, Jestha, Ashad, Shrawan, Bhadra, Ashoj, Kartik, Mangsir, Poush, Magh, Falgun, Chaitra }
impl Month {
    pub const ALL: [Month; 12];
    pub const fn number(self) -> u8;
    pub fn name(self, lang: Lang) -> &'static str;
    pub fn short_name(self, lang: Lang) -> &'static str;
    pub fn next(self) -> Month;
    pub fn prev(self) -> Month;
}
impl TryFrom<u8> for Month { type Error = Error; }
impl Display for Month {}

pub enum Weekday { Sunday, Monday, Tuesday, Wednesday, Thursday, Friday, Saturday }
impl Weekday {
    pub const ALL: [Weekday; 7];
    pub const fn index_from_sunday(self) -> u8;
    pub fn name(self, lang: Lang) -> &'static str;
    pub fn short_name(self, lang: Lang) -> &'static str;
}
impl Display for Weekday {}

pub struct Digits;
impl Digits { pub fn write_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result; }
impl Display for Digits {}
```

`Separator::as_char` gives the separator as a `char`. `DateOrder`, `Separator` and `LongDate`
are the option values used by the formatters above.

## 5. Parsing

Strict parsing accepts year, month and day in that order, separated by `- / . _ \ space | ।`.
It accepts ASCII and Devanagari digits, mixed, with at most four digits per group (D-05). Lenient
parsing adds day-month-year order, month names, `,`, era markers and the words `गते` and `मिति`;
it never guesses, so an ambiguous order or a two-digit year fails (D-06).

```rust
use nepdate::{Error, NepaliDate, ParseErrorKind};

assert_eq!("2081/04/15".parse::<NepaliDate>()?, NepaliDate::new(2081, 4, 15)?);
assert_eq!(NepaliDate::try_parse("२०८१/०४/१५")?, NepaliDate::new(2081, 4, 15)?);
assert_eq!(NepaliDate::parse_lenient("15 Shrawan 2081")?, NepaliDate::new(2081, 4, 15)?);
assert_eq!(
    NepaliDate::try_parse_lenient("15 Shrawan 81"),
    Err(Error::Parse(ParseErrorKind::Ambiguous))
);
Ok::<(), nepdate::Error>(())
```

```rust
use nepdate::NepaliDate;
impl NepaliDate {
    pub fn try_parse(s: &str) -> Result<Self, Error>;
    pub fn parse_lenient(s: &str) -> Result<Self, Error>;
    pub fn try_parse_lenient(s: &str) -> Result<Self, Error>;
}
impl FromStr for NepaliDate { type Err = Error; }
```

## 6. Ranges

A `NepaliDateRange` is inclusive and never empty, so `start > end` is refused. `Days` iterates
every day and implements `DoubleEndedIterator`, `ExactSizeIterator` and `FusedIterator`.

```rust
use nepdate::{DateOrder, Lang, NepaliDate, NepaliDateRange, Separator};

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
Ok::<(), nepdate::Error>(())
```

```rust
use nepdate::{NepaliDate, NepaliDateRange};

let r = NepaliDateRange::month(2081, 4)?;
let other = NepaliDateRange::single(NepaliDate::new(2081, 4, 15)?);
assert!(r.contains_range(other));
assert!(r.overlaps(other));
assert_eq!(r.intersect(other), Some(other));
assert_eq!(r.union(other), r);
assert_eq!(r.split_by_month().len(), 1);
assert_eq!(r.split_by_fiscal_quarter().len(), 1);
assert_eq!(r.except(other).len(), 2);
assert_eq!(r.dates_with_interval(3)?.count(), 11);
assert!(r.working_days(false).count() < r.len() as usize);
assert!(r.weekend_days(true).count() >= r.weekend_days(false).count());
Ok::<(), nepdate::Error>(())
```

`except`, `split_by_month` and `split_by_fiscal_quarter` return `Vec` and need the `alloc`
feature. `dates_with_interval` rejects an interval below 1 with `Error::OutOfRange`. The three
clock factories take a `jiff::tz::TimeZone`.

```rust
use nepdate::{DateOrder, Lang, NepaliDate, NepaliDateRange, Separator};
impl NepaliDateRange {
    pub const fn new(start: NepaliDate, end: NepaliDate) -> Option<Self>;
    pub const fn single(date: NepaliDate) -> Self;
    pub fn from_day_count(start: NepaliDate, days: u32) -> Result<Self, Error>;
    pub const fn start(self) -> NepaliDate;
    pub const fn end(self) -> NepaliDate;
    pub fn len(self) -> u32;
    pub fn contains(self, date: NepaliDate) -> bool;
    pub fn contains_range(self, other: Self) -> bool;
    pub fn overlaps(self, other: Self) -> bool;
    pub fn is_adjacent_to(self, other: Self) -> bool;
    pub fn intersect(self, other: Self) -> Option<Self>;
    pub fn union(self, other: Self) -> Self;
    pub fn days(self) -> Days;
    pub fn dates_with_interval(self, interval: u32) -> Result<DatesWithInterval, Error>;
    pub fn working_days(self, exclude_sunday: bool) -> impl Iterator<Item = NepaliDate> + Clone;
    pub fn weekend_days(self, include_sunday: bool) -> impl Iterator<Item = NepaliDate> + Clone;
    pub const fn to_string_with(self, order: DateOrder, sep: Separator, lang: Lang) -> RangeFormat;

    pub fn month(year: u16, month: u8) -> Result<Self, Error>;
    pub fn year(year: u16) -> Result<Self, Error>;

    #[cfg(feature = "alloc")] pub fn except(self, other: Self) -> Vec<Self>;
    #[cfg(feature = "alloc")] pub fn split_by_month(self) -> Vec<Self>;
    #[cfg(feature = "alloc")] pub fn split_by_fiscal_quarter(self) -> Vec<Self>;

    #[cfg(feature = "jiff")] pub fn current_month(tz: &jiff::tz::TimeZone) -> Result<Self, Error>;
    #[cfg(feature = "jiff")] pub fn current_fiscal_year(tz: &jiff::tz::TimeZone) -> Result<Self, Error>;
    #[cfg(feature = "jiff")] pub fn current_calendar_year(tz: &jiff::tz::TimeZone) -> Result<Self, Error>;
}
impl IntoIterator for NepaliDateRange { type Item = NepaliDate; type IntoIter = Days; }
pub struct Days;
impl Iterator for Days { type Item = NepaliDate; }
impl DoubleEndedIterator for Days {}
impl ExactSizeIterator for Days {}
impl FusedIterator for Days {}
impl Display for NepaliDateRange {}
impl PartialEq for NepaliDateRange {}
impl Eq for NepaliDateRange {}
impl Hash for NepaliDateRange {}

pub struct RangeFormat;
impl RangeFormat {
    pub fn write_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result;
}
impl Display for RangeFormat {}

pub struct DatesWithInterval;
impl Iterator for DatesWithInterval { type Item = NepaliDate; }
impl ExactSizeIterator for DatesWithInterval {}
impl FusedIterator for DatesWithInterval {}
```

`Intersect` returns `None` when the ranges do not overlap. At 2199-12-30 the day iterators stop
after the last date, the splits return their last piece, and `is_adjacent_to` returns `false`
instead of throwing (D-16).

## 7. Fiscal year

A fiscal year starts on 1 Shrawan and ends on the last day of Ashad of the next BS year.
`FiscalYear::new(2082)` names the year that starts in 2082.

```rust
use nepdate::{FiscalYear, Lang, NepaliDate, Quarter};

let fy = NepaliDate::new(2083, 2, 10)?.fiscal_year();
assert_eq!(fy, FiscalYear::new(2082));
assert_eq!(fy.start()?.to_string(), "2082/04/01");
assert_eq!(fy.end()?.to_string(), "2083/03/32");
assert_eq!(fy.quarter_range(Quarter::Q1)?.start().to_string(), "2082/04/01");
assert_eq!(fy.label(Lang::En).to_string(), "2082/83");
Ok::<(), nepdate::Error>(())
```

```rust
use nepdate::{Lang, NepaliDate, NepaliDateRange};
pub struct FiscalYear(u16);
impl FiscalYear {
    pub const fn new(start_year: u16) -> Self;
    pub const fn start_year(self) -> u16;
    pub fn containing(date: NepaliDate) -> Self;
    pub fn start(self) -> Result<NepaliDate, Error>;
    pub fn end(self) -> Result<NepaliDate, Error>;
    pub fn range(self) -> Result<NepaliDateRange, Error>;
    pub fn quarter_range(self, q: Quarter) -> Result<NepaliDateRange, Error>;
    pub const fn label(self, lang: Lang) -> FiscalLabel;
}
pub enum Quarter { Q1, Q2, Q3, Q4 }
pub struct FiscalLabel;
impl FiscalLabel { pub fn write_to<W: core::fmt::Write + ?Sized>(&self, out: &mut W) -> core::fmt::Result; }
```

The C# `yearOffset` instance methods and the static `(fiscalYear, month)` quarter helpers are not
ported; the fiscal-year value and its quarter range cover them (D-08).

## 8. Patro

The patro data is a separate crate, `nepdate-patro`, which depends on `nepdate`. It has no
features, no dependencies and no allocation, and it is `#![no_std]`.

```rust
use nepdate::{Lang, NepaliDate};

let info = nepdate_patro::info(NepaliDate::new(2081, 1, 1)?);
assert_eq!(info.tithi().map(|t| t.name(Lang::En)), Some("Panchami"));
assert!(info.is_public_holiday());
let events: Vec<&str> = info.events().map(|e| e.name(Lang::En)).collect();
assert_eq!(events, ["New Year", "Mesh Sankranti", "Bisket Jatra"]);
Ok::<(), nepdate::Error>(())
```

```rust
use nepdate::{Lang, NepaliDate, NepaliDateRange};
pub fn info(date: NepaliDate) -> CalendarInfo;
pub fn coverage() -> NepaliDateRange;
pub fn public_holidays() -> impl DoubleEndedIterator<Item = NepaliDate> + ExactSizeIterator;

pub struct CalendarInfo;
impl CalendarInfo {
    pub fn tithi(self) -> Option<Tithi>;
    pub fn is_public_holiday(self) -> bool;
    pub fn events(self) -> Events;
    pub fn has_events(self) -> bool;
}
pub struct Tithi;
impl Tithi { pub fn name(self, lang: Lang) -> &'static str; }
pub struct Event;
impl Event { pub fn name(self, lang: Lang) -> &'static str; }
pub struct Events;
impl Iterator for Events { type Item = Event; }
```

`info` never fails. Outside the data range it returns an empty `CalendarInfo`. The data covers
BS 2077-01-01 to 2084-08-20.

## Errors

One `Error` type covers construction, conversion, arithmetic, ranges and parsing. It implements
`core::error::Error` without the `std` feature, so `?` works in `no_std` code with an error trait
object where the target supports it.

```rust
use nepdate::{Error, NepaliDate, ParseErrorKind};

assert_eq!(NepaliDate::new(1900, 1, 1), Err(Error::OutOfRange));
assert_eq!(NepaliDate::new(2081, 13, 1), Err(Error::InvalidMonth(13)));
assert_eq!(
    NepaliDate::new(2081, 4, 33),
    Err(Error::InvalidDay { month_length: 32, day: 33 })
);
assert_eq!(
    "2081/04".parse::<NepaliDate>(),
    Err(Error::Parse(ParseErrorKind::WrongGroupCount))
);
```

```rust
use nepdate::NepaliDate;
pub enum Error {
    OutOfRange,
    InvalidMonth(u8),
    InvalidDay { month_length: u8, day: u8 },
    InvalidGregorian,
    Parse(ParseErrorKind),
}
pub enum ParseErrorKind {
    InvalidCharacter,
    WrongGroupCount,
    NumberTooLong,
    Ambiguous,
    Unrecognized,
}
```

Validation reports the first failure in the order year, month, day.
