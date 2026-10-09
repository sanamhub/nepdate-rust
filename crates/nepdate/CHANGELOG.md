# Changelog

All notable changes to the `nepdate` crate. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- `NepaliDate::checked_add_days`, `checked_add_months` and `checked_add_years` with `MonthOverflow::{Clamp, Overflow}` (ALGORITHM §8).
- `NepaliDate::diff` returning `DateDiff` (ALGORITHM §8a).
