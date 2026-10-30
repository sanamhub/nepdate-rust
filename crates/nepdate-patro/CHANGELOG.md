# Changelog

All notable changes to `nepdate-patro` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the crate follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- `verify-published.yml` also runs on a schedule: weekly against the newest crates.io version and
  daily against the registry metadata, reporting a failure as an issue labelled `published-check`.

## [0.1.0-beta.1] - 2026-10-22

First public pre-release. The API may still change before 0.1.0.

### Added

- `info`, `coverage` and `public_holidays`, with tithi, the public holiday flag and events for BS
  2077-01-01 to 2084-08-20.
- Every lookup is O(1), returns `&'static str` names, allocates nothing and is `no_std`.
