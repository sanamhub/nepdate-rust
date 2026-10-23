# Changelog

All notable changes to MitiBar are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and MitiBar follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0-beta.1] - 2026-10-23

First public pre-release, unsigned on Windows and macOS. Behaviour and settings may still change
before 0.1.0. The idle memory, CPU and cold-start targets are not confirmed yet: the last
measurements ran on a loaded machine ([benchmarks](https://github.com/sanamhub/nepdate-rust/blob/main/docs/benchmarks-widget.md)).

### Added

- Tray icon and menu: Open, Copy today (BS), Copy today (AD), Show bar (Windows), Settings and
  Quit, translated into Nepali.
- Floating bar on Windows: a small rounded pill with two lines you choose, such as the weekday,
  time, an extra clock, the BS date and the AD date. Drag it anywhere, lock it, or snap it to the
  taskbar's centre line. A click toggles the popup; right-click opens its menu.
- Popup with rounded corners on Windows 11: today's BS long date, weekday, AD date and clocks for
  the local zone and one extra IANA zone. It hides when it loses focus and on Escape, unless it is
  pinned.
- Month grid with AD days, tithi, event dots and holiday colours. Click the month title to zoom
  out to the year and decade views. A day tooltip, Go to date (Ctrl+G, BS or AD) and the next five
  holidays.
- Day detail with tithi, holiday, events and Copy BS / Copy AD.
- Converter for BS to AD and AD to BS, plus the number of days between two dates.
- Settings window: language, theme, launch at login, clock format, seconds, extra time zone, the
  bar and its line editor, and the global shortcut. Changes apply at once and are saved
  atomically, with a Saved note.
- Global shortcut to toggle the popup: Win+Shift+N on Windows, Cmd+Shift+N on macOS,
  Super+Shift+N on Linux X11. Not available on Wayland.
- Keyboard use: Tab and Shift+Tab follow the visual order, arrows move around the grid, and every
  focusable control shows a focus ring.
- Nepali digits in the year box and Copy AD when the language is Nepali, like Copy BS.
- Single instance: a second launch opens the running instance's popup and exits.
- Software rendering only, so no OpenGL or GPU driver is loaded.
- App icon: a calendar page with "सं".
