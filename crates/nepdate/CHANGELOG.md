# Changelog

All notable changes to `nepdate` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the crate follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
