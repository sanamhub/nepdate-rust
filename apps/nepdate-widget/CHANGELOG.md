# Changelog

All notable changes to the `nepdate-widget` app. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Slint window that opens with today's BS date in English and Nepali, in the bundled Noto Sans and Noto Sans Devanagari fonts.
- Debug-only `--font-test` window for the Devanagari rendering spike.
- Tray icon (W1): day-number icon on Windows/Linux, text title on macOS, with the Open / Copy today (BS) / Copy today (AD) / Settings / Quit menu, translated via bundled Nepali strings.
- Popup window (W1): borderless always-on-top 360x440 header with today's BS long date, weekday, AD date and the local plus extra-zone clocks; hides on Escape and focus loss.
- Once-per-minute timer that refreshes the tray and popup at the next minute boundary (every second with seconds shown).
- File logging and a panic hook that logs the message and location.
- `--popup --snapshot <png> --theme light|dark --lang en|ne` to render the popup for the W1 spike; `cargo xtask icons` pre-renders the tray day icons.
