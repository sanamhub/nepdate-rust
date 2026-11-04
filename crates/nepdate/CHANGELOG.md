# Changelog

All notable changes to `nepdate` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the crate follows
[Semantic Versioning](https://semver.org/).

## [1.0.0-rc.1] - 2026-11-04

### Changed

- The API is stable from 1.0 (hub ADR-0009). No code change in this crate; released with the family on the 1.0 line.

## [0.2.0-rc.1] - 2026-11-03

### Added

- `NepaliDate::year_length`, the length of the BS year from 364 to 367 days (D-12).
- `NepaliDate::from_ymd_int` and `to_ymd_int`, the `YYYYMMDD` integer form.
- `NepaliDate::try_parse` and `try_parse_lenient`, named non-throwing parsers.
- `NepaliDate::is_today`, `is_yesterday` and `is_tomorrow`, each with an explicit time zone.
- `Sub` for `NepaliDate`, returning a `DateDiff` in the C# direction, `this - other`.
- Ordered batch conversion with `to_nepali_dates`, `to_english_dates` and
  `to_english_dates_from_strings`, behind the new `alloc` feature.
- Range factories `single`, `from_day_count`, the `current_*` clock factories, containment of a
  range, overlap, adjacency, intersection, union, difference, the month and fiscal-quarter
  splits, the interval, working-day and weekend-day iterators, `Display`, `to_string_with` and
  `RangeFormat`.
- `alloc` feature. The default `std` feature turns it on. `except`, `split_by_month` and
  `split_by_fiscal_quarter` return `Vec` and need it.
- `Display` for `Month` and `Weekday`, using the English full names.
- `DatesWithInterval`, the named iterator behind `dates_with_interval`.

### Changed

- `Error` implements `core::error::Error` in every build, so `no_std` callers get the error trait
  without the `std` feature.
- A pattern that ends with a single backslash now writes a literal `\` (FORMATTING section 5).
- The public API page and the README now cover every group, including ranges, fiscal values,
  errors, batch conversion and the `alloc` feature.

### Breaking

- `is_today` now takes the time zone explicitly. Migration: replace `date.is_today()` with
  `date.is_today(&zone)`, where `zone` is a `jiff::tz::TimeZone`; for Nepal use
  `TimeZone::get("Asia/Kathmandu")`.
- A pattern that ends with a single backslash now writes that backslash instead of nothing.
  Migration: remove the trailing `\` when the old output without it was intended.

## [0.1.0-rc.1] - 2026-11-03

### Changed

- `verify-published.yml` also runs on a schedule: weekly against the newest crates.io version and
  daily against the registry metadata, reporting a failure as an issue labelled `published-check`.

## [0.1.0-beta.1] - 2026-10-22

First public pre-release. The API may still change before 0.1.0.

### Added

- Bikram Sambat dates for BS 1901-01-01 to 2199-12-30, with `NepaliDate`, `Month`, `Weekday` and
  `Lang`.
- BS to AD and AD to BS conversion, weekday, day of year, month length and range navigation.
- Checked date arithmetic: `checked_add_days`, `checked_add_months`, `checked_add_years`,
  `days_until`, `diff`, `succ` and `pred`.
- Fiscal year, quarters and the `2082/83` label.
- Formatting in English and Nepali, including patterns, long dates and Devanagari digits.
- Strict parsing through `FromStr` and lenient parsing for month names and era markers.
- Features `serde`, `chrono` and `jiff`, including `today()` in `Asia/Kathmandu`.
- A `no_std` crate (`std` is a default feature) with no required dependencies, no allocation
  and no `unsafe`.
