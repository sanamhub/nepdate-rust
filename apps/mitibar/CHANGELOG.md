# Changelog

All notable changes to MitiBar are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and MitiBar follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Tray icon and menu: Open, Copy today (BS), Copy today (AD), Settings and Quit, translated into
  Nepali.
- Popup with today's BS long date, weekday, AD date and clocks for the local zone and one extra
  IANA zone. It hides when it loses activation and on Escape.
- Month grid with AD days, tithi, event dots, today and holiday colours, month and year pickers,
  wheel navigation and a next-holiday line. The year box has previous-year and next-year buttons,
  disabled at 1901 and 2199.
- Day detail with tithi, holiday, events and Copy BS / Copy AD.
- Converter for BS to AD and AD to BS, plus the number of days between two dates.
- Settings for language, theme, clock format, seconds, extra time zone, launch at login and the
  Windows 11 taskbar text. Changes apply at once and are saved atomically.
- Windows 11 taskbar text: two lines of date and time inside the taskbar, with a click to open
  the popup and a right-click menu.
- Keyboard use: Tab and Shift+Tab follow the visual order, Return or Space activates the focused
  control, and every focusable control shows a focus ring.
- Nepali digits in the year box and Copy AD when the language is Nepali, like Copy BS.
- Single instance: a second launch opens the running instance's popup and exits.
- Software rendering only, so no OpenGL or GPU driver is loaded.
- App icon: a calendar page with "सं".
