# Changelog

All notable changes to MitiBar. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.1.0] - 2026-10-18

### Added

- Slint window that opens with today's BS date in English and Nepali, in the bundled Noto Sans and Noto Sans Devanagari fonts.
- Debug-only `--font-test` window for the Devanagari rendering spike.
- Tray icon (W1): day-number icon on Windows/Linux, text title on macOS, with the Open / Copy today (BS) / Copy today (AD) / Settings / Quit menu, translated via bundled Nepali strings.
- Popup window (W1): borderless always-on-top 360x440 header with today's BS long date, weekday, AD date and the local plus extra-zone clocks; hides on Escape and focus loss.
- Once-per-minute timer that refreshes the tray and popup at the next minute boundary (every second with seconds shown).
- File logging and a panic hook that logs the message and location.
- `--popup --snapshot <png> --theme light|dark --lang en|ne` to render the popup for the W1 spike; `cargo xtask icons` pre-renders the tray day icons.
- Taskbar text (W1B, Windows 11): two lines of date and time inside the taskbar, left of the Start button, in the free area, redrawn only on change; left click toggles the popup above the text, right click opens the WR-01 AC5 menu. Hidden with one log line on Windows 10, a non-bottom or left-aligned taskbar, or when it does not fit. `--no-taskbar-text` disables it.
- `--taskbar-snapshot <png> --taskbar-theme light|dark --lang en|ne --scale 1|1.5|2` (snapshot feature) renders the taskbar text over a taskbar-coloured strip for the W1B spike.
- Month grid and day detail (W2): one BS month with AD days, tithi, event dots, today and holiday colours, prev/next/Today, month and year pickers, wheel navigation and the next-holiday line. Clicking a day opens the detail card with the copy buttons.
- Converter (W3): BS → AD and AD → BS over the lenient BS parser and the strict AD form, plus days-between; results appear as the user types, and the range message follows WR-07 AC3.
- Settings (W3): language, theme, 12/24-hour clock, seconds, extra IANA time zone with a filter, launch at login and the Windows 11 taskbar text. Changes apply at once and are saved atomically 300 ms after the last one; a corrupt file is moved aside.
- Launch at login (W3) and single instance (W3): a second launch opens the running instance's popup and exits 0.
- `--converter` and `--settings` (snapshot feature) open those cards for the W3 review images.
- App icon (a calendar page with "सं") and `cargo xtask icons` output: PNG sizes 16 to 1024, a multi-image `.ico` and an `.icns`.
- Packaging and release preparation (W4): `apps/mitibar/Packager.toml`, `cargo xtask package` (NSIS plus a portable zip on Windows, `.app`/`.dmg` on macOS, `.AppImage`/`.deb` on Linux), `release-mitibar.yml` with a draft release, `SHA256SUMS`, a CycloneDX SBOM and build-provenance attestations, and the release runbook.
- Windows ARM64 (`aarch64-pc-windows-msvc`) release build and asset names (W5-05).
- `cargo xtask widget-idle [--minutes N]` and the app's `--settings-file <path>` test flag (W5-01); the harness writes `docs/benchmarks/widget/<date>/idle.json` and never touches the real settings or login item.

### Changed

- The popup is created on first open instead of at startup, and the single-instance listener wakes the UI thread instead of being polled every 250 ms (W5-02, W5-03).
- MitiBar renders with Slint's software renderer; femtovg and its OpenGL stack leave the dependency (W6-02, ADR-0018).
- The year box has visible previous-year and next-year buttons, translated and disabled at 1901 and 2199 (W6-03).
- The month header's year box and Copy AD use the active language's digits, like Copy BS (W5-04).
- Windows signing runs on `mitibar.exe` before packaging and on the NSIS setup afterward; macOS signs `MitiBar.app` before the DMG and notarizes and staples the DMG (W5-05).
- Renamed the app to MitiBar (ADR-0017): the tray tooltip's first line, the popup window title and the binary are MitiBar, and the crate, folders, gettext domain, window classes and release tag use `mitibar`.
