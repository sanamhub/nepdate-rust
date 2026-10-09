# Changelog

All notable changes to the `nepdate` crate. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- `NepaliDate::checked_add_days`, `checked_add_months` and `checked_add_years` with `MonthOverflow::{Clamp, Overflow}` (ALGORITHM §8).
- `NepaliDate::diff` returning `DateDiff` (ALGORITHM §8a).
- `NepaliDateRange` (`new`, `month`, `year`, `start`, `end`, `len`, `contains`, `days`) and the `Days` iterator, which is double-ended, exact-size and fused (D-09).
- `FiscalYear`, `Quarter` and `FiscalLabel`, plus `NepaliDate::fiscal_year` and `quarter` (ALGORITHM §8b, D-08).
- `Lang::digits` returning `Digits`, ASCII or Devanagari digits without allocation.
