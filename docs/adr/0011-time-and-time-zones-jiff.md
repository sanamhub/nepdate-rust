# ADR-0011: Time and time zones: `jiff` in the widget, nothing in the core

- Status: Accepted
- Date: 2026-09-24 (revised 2026-10-10: `today()` uses `Asia/Kathmandu` and `today_in` takes a
  zone, to match the hub PORT-PLAN phase 4 and `docs/API.md`; it used the local zone before)
- Deciders: Sanam Pakuwal

## Context

- The library core is `no_std` and needs only Gregorian *dates* (ADR-0004, ADR-0005).
  "Today" needs the local calendar date (feature `std`).
- The widget needs local time, one extra IANA zone (default `Asia/Kathmandu`, UTC+05:45),
  zone listing for a picker, and correct DST handling for other zones.

Options: `jiff` 0.2.x (IANA tzdb built in: bundled on Windows, system zoneinfo on Unix;
DST-safe; `Zoned` type), `chrono` 0.4 + `chrono-tz` (compiles tzdb into the binary,
clunkier API), `time` 0.3 (no IANA db, so not enough).

## Decision

1. **Widget**: `jiff` (`jiff = "0.2"`, features `std`, `tz-system`, `tzdb-bundle-platform`).
   - Local time: `jiff::Zoned::now()`.
   - Extra zone: `now.with_time_zone(jiff::tz::TimeZone::get(id)?)`.
   - Zone list: `jiff::tz::db().available()`.
2. **Library `std` feature, `NepaliDate::today()`**: needs a zone. `std` has no
   time-zone API, so the library does **not** implement zone lookup itself.
   Decision: "today" is provided **only with the `jiff` feature**, following the hub
   PORT-PLAN phase 4 ("today" with an explicit zone defaulting to `Asia/Kathmandu`) and
   PARITY D-14:
   - `NepaliDate::today_in(&jiff::tz::TimeZone) -> Result<Self, Error>`: the date in that zone.
   - `NepaliDate::today() -> Result<Self, Error>`: `today_in` with `Asia/Kathmandu`, or a
     fixed +05:45 offset when the tz database lacks it.
   - `is_today(self)`: `Ok(self) == today()`.
   Without `jiff`, callers construct from their own date source
   (`NepaliDate::from_gregorian(y, m, d)`). This keeps `std` dependency-free
   (matches the feature table in ADR-0005).
3. **Interop features** in the library: `jiff` (`From<NepaliDate> for jiff::civil::Date`,
   `TryFrom<jiff::civil::Date>`) and `chrono` (`NaiveDate` equivalents). Both optional.
4. Timezone IDs are always IANA strings (never Windows IDs like "Nepal Standard Time").

## Consequences

- The core stays zero-dependency; users who want `today()` opt into `jiff`.
- The widget depends on `nepdate` with feature `jiff`, and on `nepdate-patro`.
- `Asia/Kathmandu`'s +05:45 offset is handled by tzdb, no special casing.
