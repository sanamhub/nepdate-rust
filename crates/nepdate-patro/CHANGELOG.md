# Changelog

All notable changes to `nepdate-patro` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the crate follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `info`, `coverage` and `public_holidays`, with tithi, the public holiday flag and events for BS
  2077-01-01 to 2084-08-20.
- Every lookup is O(1), returns `&'static str` names, allocates nothing and is `no_std`.
