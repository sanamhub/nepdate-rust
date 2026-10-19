# MitiBar: Functional requirements (v0.1)

Source of truth for the widget (ADR-0008). NepDateWidget may be read for reference, but
only what is listed here gets built. Every requirement has an ID
(`WR-xx`) and testable acceptance criteria (`AC`). Commits and PRs reference these IDs.
Anything not listed here is out of scope.

Glossary: **BS** = Bikram Sambat (Nepali) date. **AD** = Gregorian date.
**Popup** = main window opened from the tray.

---

## WR-01 Tray / menu-bar icon

- AC1: On start, an icon appears in the Windows notification area, the macOS menu bar,
  and on Linux via StatusNotifierItem (KDE, GNOME with AppIndicator extension, etc.).
- AC2: Windows/Linux icon image shows today's BS day number (Devanagari digits when
  language = ne, ASCII otherwise), regenerated when the date changes.
  macOS shows a text title instead, e.g. `८ असोज` / `8 Ashoj`.
- AC3: Tooltip (where the OS supports it): `MitiBar` on the first line, then the BS long date and
  AD date on the second, e.g. `Ashoj 8, 2083 · 2026-09-24`.
- AC4: Left-click (Windows/Linux) toggles the popup. On macOS, clicking opens the menu,
  whose first item is "Open".
- AC5: Menu items, in order: Open · Copy today (BS) · Copy today (AD) · Settings · Quit.
  "Copy" writes `YYYY/MM/DD` (BS) or `YYYY-MM-DD` (AD) to the clipboard, both in the active
  language's digits (W5-04: the clipboard is exactly what the screen shows).
- AC6: If no tray is available (e.g. GNOME without the extension), the app opens the
  popup on start and logs a warning. It must not exit.

## WR-02 Popup window

- AC1: Borderless and non-resizable, with a fixed logical size (default 360×520). The window
  title is `MitiBar` (shown in Alt+Tab). It opens at the primary monitor's tray corner (Windows
  bottom-right above the taskbar, macOS top-right below the menu bar, Linux X11 top-right;
  Wayland: compositor placement), fully on-screen, and doesn't show in the Windows taskbar.
- AC2: Closes (hides) on focus loss or Escape. Re-opening shows the last viewed month,
  except when the date changed while hidden: then it shows today's month.
- AC3: Header shows the BS long date, weekday, AD date, local time and the extra zone
  time (WR-05).
- AC4: Follows theme (WR-06). Latin text uses the bundled UI font, Devanagari the
  bundled Noto Sans Devanagari (OFL). No system font dependency.

## WR-03 Month grid

- AC1: Shows one BS month as a 7-column grid, Sunday first. Leading and trailing cells
  outside the month are blank. The grid has 5 or 6 rows as needed
  (`ceil((first_weekday + month_length) / 7)`).
- AC2: Each cell shows the BS day (large) and the AD day of month (small). On the cell for
  day 1 of an AD month, the short AD month name replaces the AD day number.
- AC3: Today's cell is outlined. Saturday cells use the Saturday colour; public holidays
  (from `nepdate-patro`) use the holiday colour, which wins on a Saturday.
- AC4: When patro data exists: tithi shown small under the day; a dot marks days with
  events. Outside `nepdate_patro::coverage()` nothing is shown and nothing errors.
- AC5: Title: `Ashoj 2083 · Sep/Oct 2026` (ne: `असोज २०८३ · Sep/Oct २०२६`). AD month names
  stay Latin; every number follows the language's digits (widget-design §4, W3 review).
- AC6: Prev/next buttons, a "Today" button, month + year dropdowns (full range
  1901–2199) and mouse wheel over the grid (one notch = one month) all navigate. The year
  control also has visible previous-year and next-year buttons. At the range edges the
  corresponding control is disabled; no crash.
- AC7: Clicking a day opens the day detail (WR-04).
- AC8: A line under the grid reads `Next holiday: <name> in N days` (or "today" /
  "tomorrow"), searching forward at most 400 days within the patro data coverage. Hidden if
  none is found.
- AC9: Rendering a month allocates no more than one `Vec` of ≤42 cells, and month
  navigation redraws in < 16 ms (debug-build measurement not required).

## WR-04 Day detail

- AC1: Inline panel or overlay inside the popup (no new OS window): BS long date, AD
  long date, weekday, tithi, holiday badge, event list. All in the current language.
- AC2: Buttons "Copy BS" / "Copy AD", both in the active language's digits (W5-04).
- AC3: Escape or clicking outside closes the detail first, then the popup.

## WR-05 Clock and time zone

- AC1: Local time is always shown. One extra IANA zone is configurable (default
  `Asia/Kathmandu`). If the extra zone has the same UTC offset as local, it is hidden.
- AC2: Format per setting: 12h `2:05 PM` / 24h `14:05`, optional seconds.
  Nepali language uses Devanagari digits.
- AC3: Label = city part of the IANA id (`Asia/Kathmandu` → `Kathmandu`), followed by
  the UTC offset `+05:45`.
- AC4: "Today" (BS date everywhere in the app) is derived from the **local** calendar
  date. The weekday is always computed from the same date as the BS date shown next to
  it (never from a different zone).
- AC5: Timer: with seconds hidden, the UI wakes at the next minute boundary (+50 ms),
  not every second. Date rollover at midnight updates tray and popup within 61 s.
- AC6: After system sleep/resume or a clock change, the display is correct within 61 s.

## WR-06 Settings

Stored per user at `dirs::config_dir()/mitibar/settings.toml`: Windows
`%APPDATA%\mitibar\`, macOS `~/Library/Application Support/mitibar/`, Linux
`$XDG_CONFIG_HOME/mitibar/` (usually `~/.config/mitibar/`).

| Key | Type | Default |
|---|---|---|
| `schema` | int | 1 |
| `language` | `"en" \| "ne"` | `"en"` |
| `theme` | `"system" \| "light" \| "dark"` | `"system"` |
| `clock_24h` | bool | false |
| `show_seconds` | bool | false |
| `extra_time_zone` | IANA string or `""` | `"Asia/Kathmandu"` |
| `launch_at_login` | bool | true |
| `taskbar_text` | bool | true (Windows 11 only, WR-12; ignored elsewhere) |

- AC1: Settings UI is a panel inside the popup. Changes apply immediately, with no
  restart.
- AC2: Missing file → defaults are written. Unknown keys are ignored. Invalid values fall back to
  that key's default and a warning is logged. A file that fails to parse is renamed to
  `settings.toml.broken-<unix-ts>` and defaults are used.
- AC3: Writes are atomic (write temp file in same dir, then rename).
- AC4: The time zone picker lists all IANA zones from `jiff`'s bundled or system tzdb
  and supports a type-to-filter search.

## WR-07 Converter

- AC1: Two modes: "BS → AD", "AD → BS". BS input uses `nepdate` lenient parsing
  (`shared/spec/PARSING.md` §2: numbers or month names, Devanagari accepted). AD input is
  `YYYY-MM-DD` / `YYYY/MM/DD` (ASCII or Devanagari digits). Result: short + long form.
- AC2: Converts as the user types. An error is shown only when the input has at least
  8 characters or contains 2 separators; otherwise the result area is empty.
- AC3: Out-of-range input shows "Supported range: 1901–2199 BS (1844–2143 AD)".
- AC4: Days-between: two BS dates (default today / today). Shows the signed total days,
  and a years/months/days breakdown (`nepdate` crate function, not widget logic).

## WR-08 Mini bar (deferred to v0.2)

Not in v0.1 (ADR-0008 §B). ID kept so later IDs don't change.

## WR-09 Lifecycle

- AC1: Single instance: a second launch signals the first (local IPC) to open the popup,
  then exits with code 0.
- AC2: Launch at login toggles a per-user login item (Windows `HKCU\...\Run`, macOS
  Login Items / LaunchAgent, Linux `~/.config/autostart/*.desktop`). No admin rights.
- AC3: Quit from the menu exits in < 1 s and leaves no background process.
- AC4: Logs go to `dirs::data_local_dir()/mitibar/`, `mitibar.log`, rotated at 1 MB, keeping 2
  files. No personal data is logged.
- AC5: Crash: a panic hook logs the panic message and location before exit.

## WR-10 Non-functional

- AC1: Targets of ADR-0008 §C (size, RAM, start time, idle CPU) are measured in W4.
- AC2: Accessibility: every interactive control has an accessible name. The popup is
  keyboard-operable: Tab/Shift+Tab move through controls and day cells, Enter opens day
  detail, PageUp/PageDown change month, Home jumps to today, Escape closes.
- AC3: Supported platforms: Windows 10 (1809+) and 11, x64 and ARM64; macOS 12+ (Apple
  silicon and Intel); Linux x64 with X11 or Wayland (tested on Ubuntu 24.04 GNOME + the
  AppIndicator extension, and KDE Plasma 6).

## WR-11 Privacy

- AC1: The app opens no network connections (verify in W4 with the OS firewall/`lsof`/
  Resource Monitor). No telemetry, no crash upload, no update check.
- AC2: Logs contain no personal data (no user names, paths inside the home folder are
  shown as `~`).

## WR-12 Windows taskbar text

Windows 11 only (ADR-0016). Elsewhere the tray icon (WR-01) is the only always-visible date.

- AC1: With a bottom, centre-aligned Windows 11 taskbar, two lines are drawn inside the
  taskbar of the primary monitor, in the free area left of the Start button (right of the
  Widgets button if it is shown), 12 px from that area's left edge, vertically centred.
  Line 1: weekday, ` | `, local time (WR-05 AC2 format). Line 2: BS long date, ` | `, AD date
  as `Oct 15, 2026`. Example (en): `Thursday | 1:50 PM` / `Ashoj 29, 2083 | Oct 15, 2026`.
  With language = ne the weekday, the time digits and the BS date are Nepali
  (`बिहिवार | १:५० PM` / `असोज २९, २०८३ | Oct १५, २०२६`); the AD month name stays Latin and
  its numbers use Devanagari digits (widget-design §4, W3 review). The
  weekday and BS date come from the same local date (WR-05 AC4), formatted by `nepdate`.
- AC2: Text colour is the `ink` token for the **taskbar** theme (`SystemUsesLightTheme`), not
  the app theme, and changes within 1 s of a system light/dark switch, without restart.
- AC3: Left click toggles the popup, which opens above the text, left-aligned with it, fully
  on-screen. Right click opens a menu with the WR-01 AC5 items in the current language.
- AC4: The text never overlaps the Start button, Widgets button or taskbar icons. When the
  icons grow towards it (apps opened), it hides within 1 s, and shows again when room
  returns. It is hidden, with one log line giving the reason, on Windows 10, with a
  left-aligned or non-bottom taskbar, or when it does not fit.
- AC5: It moves with an auto-hiding taskbar and is not visible over a full-screen app.
- AC6: After Explorer restarts, the text is back within 5 s. Any failure hides the text and
  logs once; the app and the tray icon keep running.
- AC7: It follows DPI and scale changes (100 % to 200 %) and monitor changes with no restart.
- AC8: Setting `taskbar_text` (WR-06) turns it on and off immediately. Idle cost: it redraws
  only when its text, colour, DPI or position changes (at most once per minute with seconds
  hidden), and adds no measurable idle CPU.
