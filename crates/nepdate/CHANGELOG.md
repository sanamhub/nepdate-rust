# Changelog

All notable changes to the `nepdate` crate. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- `NepaliDate` (4 bytes, `const fn new`, `MIN`, `MAX`, getters, month length, first and last day of month), `Month`, `Weekday`, `Lang`, `Error` and `ParseErrorKind`.
- BS and AD conversion without loops or searches: `to_gregorian`, `from_gregorian`, `TryFrom<GregorianDate>`, plus `GregorianDate`, `weekday`, `day_of_year`, `succ`, `pred` and `days_until`, checked against all 109,212 supported days.
- `NepaliDate::checked_add_days`, `checked_add_months` and `checked_add_years` with `MonthOverflow::{Clamp, Overflow}` (ALGORITHM §8).
- `NepaliDate::diff` returning `DateDiff` (ALGORITHM §8a).
- `NepaliDateRange` (`new`, `month`, `year`, `start`, `end`, `len`, `contains`, `days`) and the `Days` iterator, which is double-ended, exact-size and fused (D-09).
- `FiscalYear`, `Quarter` and `FiscalLabel`, plus `NepaliDate::fiscal_year` and `quarter` (ALGORITHM §8b, D-08).
- `Lang::digits` returning `Digits`, ASCII or Devanagari digits without allocation.
- Formatter values `ShortFormat`, `LongFormat` and `PatternFormat` from `NepaliDate::short`, `long` and `format_pattern`, with `DateOrder`, `Separator` and `LongDate` (FORMATTING §3 to §5). Each has a generic `write_to` that writes without allocating; English or Nepali output (D-15).
- Strict parsing through `FromStr` (ASCII and Devanagari digits, D-05) and `NepaliDate::parse_lenient` for month names, day-month-year order and era markers (PARSING, D-06). Neither allocates.
