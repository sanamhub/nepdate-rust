# Changelog

All notable changes to the `nepdate-patro` crate. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-10-15

### Added

- `info`, `coverage` and `public_holidays` with `CalendarInfo`, `Events`, `Tithi` and `Event`: tithi, public holidays and events for BS 2077-01-01 to 2084-08-20 (PATRO §2). Every lookup is O(1) over per-day tables and returns `&'static str` names; nothing allocates.
