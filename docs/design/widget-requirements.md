# MitiBar: Functional requirements (v0.1)

Source of truth for the widget (ADR-0008). NepDateWidget may be read for reference, but
only what is listed here gets built. Every requirement has an ID
(`WR-xx`) and testable acceptance criteria (`AC`). Commits and PRs reference these IDs.
Anything not listed here is out of scope.

Glossary: **BS** = Bikram Sambat (Nepali) date. **AD** = Gregorian date.
**Popup** = main window opened from the tray, the bar or the shortcut. **Bar** = the floating
mini bar on Windows (WR-08).

Revised on 2026-10-22 after the owner tested the first beta build: WR-01 AC5, WR-02, WR-03 AC6,
WR-06, WR-08, WR-09 AC2, WR-10 AC2 and WR-12 changed, and WR-13 to WR-15 are new (ADR-0019,
ADR-0020).

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
- AC5: Menu items, in order: Open · Copy today (BS) · Copy today (AD) · Show bar (a check
  item, Windows only, WR-08) · Settings · Quit.
  "Copy" writes `YYYY/MM/DD` (BS) or `YYYY-MM-DD` (AD) to the clipboard, both in the active
  language's digits, so the clipboard matches the screen.
- AC6: If no tray is available (e.g. GNOME without the extension), the app opens the
  popup on start and logs a warning. It must not exit.

## WR-02 Popup window

- AC1: Borderless and non-resizable, with a fixed logical size (default 360×520). The window
  title is `MitiBar` (shown in Alt+Tab). Opened from the tray, it sits at the primary monitor's
  tray corner (Windows bottom-right above the taskbar, macOS top-right below the menu bar, Linux
  X11 top-right; Wayland: compositor placement). Opened from the bar, it sits above the bar when
  there is room and below it otherwise, left-aligned with it. Opened by the shortcut (WR-13), it
  sits by the bar when the bar is shown, otherwise at the tray corner. Always fully on-screen;
  never shown in the Windows taskbar.
- AC2: Closes (hides) on focus loss or Escape, unless pinned (WR-14 AC1). Focus moving to the
  Settings window (WR-06) is not a focus loss. Re-opening shows the last viewed month, except
  when the date changed while hidden: then it shows today's month.
- AC3: Header shows the BS long date, weekday, AD date, local time and the extra zone
  time (WR-05), and three header actions as pill buttons: Convert, Settings and Pin
  (widget-design §7).
- AC4: Follows theme (WR-06). Latin text uses the bundled UI font, Devanagari the
  bundled Noto Sans Devanagari (OFL). No system font dependency.
- AC5: Rounded corners. On Windows 11 the window asks DWM for rounded corners
  (`DWMWA_WINDOW_CORNER_PREFERENCE` = round), which also gives the system border and shadow.
  Windows 10 can't round a window's corners, so there the popup keeps square corners with a
  1 px `hairline` border. On macOS and Linux the corners are rounded with a 10 px radius when
  the window can be transparent, and square with the hairline border when it can't.
- AC6: No third-party branding anywhere in the app (ADR-0020).

## WR-03 Month grid

- AC1: Shows one BS month as a 7-column grid, Sunday first. Leading and trailing cells
  outside the month are blank. The grid has 5 or 6 rows as needed
  (`ceil((first_weekday + month_length) / 7)`).
- AC2: Each cell shows the BS day (large) and the AD day of month (small). On the cell for
  day 1 of an AD month, the short AD month name replaces the AD day number.
- AC3: Today's BS day number sits on a filled `sindoor` disc. Saturday cells use the Saturday
  colour; public holidays (from `nepdate-patro`) use the holiday colour, which wins on a
  Saturday.
- AC4: When patro data exists: tithi shown small under the day; a dot marks days with
  events. Outside `nepdate_patro::coverage()` nothing is shown and nothing errors.
- AC5: Title: `Ashoj 2083 · Sep/Oct 2026` (ne: `असोज २०८३ · Sep/Oct २०२६`). AD month names
  stay Latin; every number follows the language's digits (widget-design §4).
- AC6: Navigation has three zoom levels, like the Windows clock calendar.
  - **Month view:** previous and next month buttons, and mouse wheel over the grid (one notch =
    one month). The title is a button.
  - **Year view:** clicking the month title shows the 12 BS months of that year as a 4×3 grid,
    with the viewed month marked. Previous and next buttons and the wheel change the year.
    Clicking a month opens it in the month view.
  - **Decade view:** clicking the year title shows 12 years as a 4×3 grid (the decade, with
    one year on each side). The buttons and the wheel move by ten years. Clicking a year opens
    its year view.
  - Escape steps back one level towards the month view, then closes the popup.
  - The Today pill is shown only when the view is not today's month in the month view; it
    returns to today's month from any level.
  - The month and year dropdowns and the separate previous-year and next-year buttons are
    removed.
  - The range is 1901 to 2199 at every level. At an edge the control that would leave the
    range is disabled, and years outside the range are not shown as choices; no crash.
- AC7: Clicking a day opens the day detail (WR-04).
- AC8: A line under the grid reads `Next holiday: <name> in N days` (or "today" /
  "tomorrow"), searching forward at most 400 days within the patro data coverage. Hidden if
  none is found.
- AC9: Rendering a month allocates no more than one `Vec` of ≤42 cells, and month
  navigation redraws in < 16 ms (debug-build measurement not required).

## WR-04 Day detail

- AC1: Inline panel or overlay inside the popup (no new OS window): BS long date, AD
  long date, weekday, tithi, holiday badge, event list. All in the current language.
- AC2: Buttons "Copy BS" / "Copy AD", both in the active language's digits.
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
| `schema` | int | 2 |
| `language` | `"en" \| "ne"` | `"en"` |
| `theme` | `"system" \| "light" \| "dark"` | `"system"` |
| `clock_24h` | bool | false |
| `show_seconds` | bool | false |
| `extra_time_zone` | IANA string or `""` | `"Asia/Kathmandu"` |
| `launch_at_login` | bool | true |
| `bar` | bool | true (Windows only, WR-08; ignored elsewhere) |
| `bar_locked` | bool | false |
| `bar_top` | array of field ids (WR-08 AC2) | `["weekday", "time", "zone_time"]` |
| `bar_bottom` | array of field ids | `["bs_date", "ad_short"]` |
| `bar_monitor` | string, monitor device name, `""` = default position | `""` |
| `bar_x`, `bar_y` | int, logical px from that monitor's top-left | 0, 0 |
| `shortcut` | string like `"Win+Shift+N"`, `""` = none (WR-13) | `"Win+Shift+N"` (`"Cmd+Shift+N"` on macOS, `"Super+Shift+N"` on Linux) |

Schema 1 files are read and rewritten as schema 2: `taskbar_text = false` becomes `bar = false`,
and the other new keys take their defaults.

- AC1: Settings opens in its own window, never on top of the popup. It sits beside the popup
  on the side with more room, top edges aligned, 8 px apart. When neither side has room, or the
  popup is closed, it opens centred on the popup's monitor. It has the title `MitiBar Settings`,
  a close button and Escape to close. It's created when opened and dropped when closed.
- AC2: Missing file → defaults are written. Unknown keys are ignored. Invalid values fall back to
  that key's default and a warning is logged. A file that fails to parse is renamed to
  `settings.toml.broken-<unix-ts>` and defaults are used.
- AC3: Writes are atomic (write temp file in same dir, then rename).
- AC4: The time zone picker is one field: it shows the chosen zone, and typing filters a
  drop-down list of all IANA zones from `jiff`'s bundled or system tzdb by city or region. Up,
  Down and Enter pick from the list with the keyboard; Escape closes the list without changing
  the zone. A "None" entry clears the extra clock.
- AC5: Every change applies at once to the popup, the bar, the tray and the menus while
  Settings stays open, with no restart and no Save button. The save is written 300 ms after the
  last change (AC3), and a `Saved` note with a check mark shows in the Settings header for
  2 s after the write succeeds. If the write fails, the note says `Couldn't save settings` and
  stays until the next successful write; the log has the reason.
- AC6: Sections, in order: **General** (language, theme, launch at login), **Clock** (24-hour,
  seconds, extra time zone), **Bar** (Windows only: show bar, lock position, the line editor of
  WR-08 AC3, Reset position), **Shortcut** (WR-13), **About** (version, MIT license, the
  third-party notices for Slint, the fonts and NepDate, as plain text). Choices of two or three
  values are segmented controls, on/off values are switches (widget-design §8).

## WR-07 Converter

- AC1: Two modes: "BS → AD", "AD → BS". BS input uses `nepdate` lenient parsing
  (`shared/spec/PARSING.md` §2: numbers or month names, Devanagari accepted). AD input is
  `YYYY-MM-DD` / `YYYY/MM/DD` (ASCII or Devanagari digits). Result: short + long form.
- AC2: Converts as the user types. An error is shown only when the input has at least
  8 characters or contains 2 separators; otherwise the result area is empty.
- AC3: Out-of-range input shows "Supported range: 1901–2199 BS (1844–2143 AD)".
- AC4: Days-between: two BS dates (default today / today). Shows the signed total days,
  and a years/months/days breakdown (`nepdate` crate function, not widget logic).

## WR-08 Mini bar (Windows)

A floating, draggable pill with one or two lines of date and time (ADR-0019). Windows 10
(1809+) and 11. macOS shows the bar's top line as the menu-bar title instead (WR-01 AC2);
Linux has the tray icon only.

- AC1: With `bar = true` the bar is shown at start. It never takes focus, has no taskbar button
  and no Alt+Tab entry, stays above the taskbar and other windows, and stays visible through
  Win+D. It hides while a full-screen app is in front (ADR-0019 §4) and shows again within
  1 s after. After Explorer restarts it's back within 5 s.
- AC2: Each line is a list of fields, shown in order with ` · ` between them (the dot in
  `ink-muted`). A field with nothing to show is left out together with its separator. If the
  top line is empty, the bottom line is the only line. Fields, with en and ne examples for BS
  2083 Kartik 5 = 2026-10-22, a Thursday:

  | Id | en | ne | Notes |
  |---|---|---|---|
  | `weekday` | `Thu` | `बिहि` | short name from `nepdate` |
  | `weekday_long` | `Thursday` | `बिहिवार` | |
  | `time` | `6:32 PM` | `६:३२ PM` | local, WR-05 AC2 format |
  | `zone_time` | `Singapore 8:46 PM` | `Singapore ८:४६ PM` | extra zone, city part only; empty when WR-05 AC1 hides it |
  | `bs_date` | `Kartik 5, 2083` | `कार्तिक ५, २०८३` | |
  | `bs_short` | `Kartik 5` | `कार्तिक ५` | |
  | `ad_date` | `Oct 22, 2026` | `Oct २२, २०२६` | AD month names stay Latin |
  | `ad_short` | `Oct 22` | `Oct २२` | |
  | `tithi` | `Ekadashi` | `एकादशी` | empty outside the patro coverage |
  | `holiday` | today's public holiday | same, in Nepali | empty when today isn't one |

  Defaults: top `weekday, time, zone_time`; bottom `bs_date, ad_short`. Example (en):
  `Thu · 6:32 PM · Singapore 8:46 PM` / `Kartik 5, 2083 · Oct 22`. The weekday and BS date come
  from the same local date (WR-05 AC4). Names and digits come from `nepdate`.
- AC3: The line editor in Settings (WR-06 AC6) shows a Top line row and a Bottom line row of
  field chips. Each chip has a remove button. "Add field" lists the fields not used on either
  line; a field appears at most once. A focused chip moves left or right with its arrow buttons
  or Alt+Left and Alt+Right, and to the other line with Alt+Up and Alt+Down. At least one field
  stays: the last chip's remove button is disabled. "Reset to default" restores the AC2
  defaults. The bar updates on every change.
- AC4: Dragging follows ADR-0019 §5: past the system drag distance it moves with the pointer,
  stays fully on one monitor, and snaps to the taskbar's centre line within 8 logical px.
  Release saves the position (`bar_monitor`, `bar_x`, `bar_y`). With `bar_locked = true` a
  drag does nothing and a click still toggles the popup. After a restart the bar is where it
  was left.
- AC5: The default position (ADR-0019 §7) is used on first run, when the saved monitor is
  gone, and after "Reset position" in Settings or the bar menu.
- AC6: A click toggles the popup (WR-02 AC1 placement). A double-click opens it and leaves it
  open. A right-click opens the menu: Open · Copy today (BS) · Copy today (AD) · Customize
  bar · Lock position (a check item) · Hide bar · Settings · Quit, in the current language.
  Customize bar opens Settings at its Bar section. Hide bar sets `bar = false`; the tray
  menu's Show bar turns it back on.
- AC7: The look is widget-design §5. Colours follow the **system** theme
  (`SystemUsesLightTheme`), as the taskbar does, and change within 1 s of a light/dark switch.
- AC8: It follows DPI and scale changes (100 % to 200 %) and monitor changes with no restart,
  keeping its place on the same monitor (AC4) or moving fully on-screen.
- AC9: Idle cost: it redraws only when its text, colour, DPI, size or position changes (at most
  once a minute with seconds hidden). Top-most upkeep and the full-screen check run on
  foreground-window changes, not on a timer. No measurable idle CPU (ADR-0008 §C).
- AC10: Any failure hides the bar and logs one line. The tray icon and the popup keep working,
  and the app never ends because of the bar.

## WR-09 Lifecycle

- AC1: Single instance: a second launch signals the first (local IPC) to open the popup,
  then exits with code 0.
- AC2: Launch at login toggles a per-user login item (Windows `HKCU\...\Run`, macOS
  Login Items / LaunchAgent, Linux `~/.config/autostart/*.desktop`). No admin rights. It is on
  by default, so a fresh install starts at the next login; the user turns it off in Settings or
  in the OS (Task Manager's Startup apps on Windows). On start with `launch_at_login = true`,
  the login item is written when it's missing or points at a file that no longer exists. On
  Windows a portable copy (one not in `%LOCALAPPDATA%\Programs\MitiBar`) never replaces an item
  that points at an existing installed copy. Turning the setting on in Settings always points
  the item at the running copy.
- AC3: Quit from the menu exits in < 1 s and leaves no background process.
- AC4: Logs go to `dirs::data_local_dir()/mitibar/`, `mitibar.log`, rotated at 1 MB, keeping 2
  files. No personal data is logged.
- AC5: Crash: a panic hook logs the panic message and location before exit.

## WR-10 Non-functional

- AC1: Targets of ADR-0008 §C (size, RAM, start time, idle CPU) are measured by
  `cargo xtask widget-idle`.
- AC2: Accessibility: every interactive control has an accessible name. The popup and the
  Settings window are keyboard-operable. Popup keys:

  | Key | Action |
  |---|---|
  | Tab, Shift+Tab | move through the controls; the grid is one stop |
  | Arrow keys | move between days in the grid (months or years in the zoom views, WR-03 AC6) |
  | Enter, Space | open the focused day's detail, or pick the focused month or year |
  | PageUp, PageDown | previous or next month (year in the year view, ten years in the decade view) |
  | Home or T | today |
  | C | Convert |
  | Ctrl+G | Go to date (WR-14 AC3) |
  | P | pin or unpin (WR-14 AC1) |
  | Ctrl+, | Settings |
  | Escape | close the open panel, then step out of a zoom view, then close the popup |

  Single-letter keys act only when focus is not in a text field. Each header pill's tooltip
  names its key.
- AC3: Supported platforms: Windows 10 (1809+) and 11, x64 and ARM64; macOS 12+ (Apple
  silicon and Intel); Linux x64 with X11 or Wayland (tested on Ubuntu 24.04 GNOME + the
  AppIndicator extension, and KDE Plasma 6).

## WR-11 Privacy

- AC1: The app opens no network connections (verify with the OS firewall, `lsof` or
  Resource Monitor). No telemetry, no crash upload, no update check.
- AC2: Logs contain no personal data (no user names, paths inside the home folder are
  shown as `~`).

## WR-12 Windows taskbar text (withdrawn)

Replaced by the mini bar, WR-08 (ADR-0019 supersedes ADR-0016). ID kept so later IDs don't
change.

## WR-13 Global shortcut

- AC1: One system-wide shortcut toggles the popup from any app (WR-02 AC1 placement). Default
  Win+Shift+N on Windows, Cmd+Shift+N on macOS, Super+Shift+N on Linux X11 (ADR-0019).
- AC2: In Settings the shortcut field records the next key combination pressed while it has
  focus. A combination needs a non-modifier key and either Win/Cmd/Super or at least two other
  modifiers. Ctrl+Alt combinations are refused with the note `Ctrl+Alt is AltGr on many
  keyboards`. "Clear" removes the shortcut; "Reset" restores the default. A change takes effect
  at once.
- AC3: When the OS refuses the registration (another app owns it), the field shows `This
  shortcut is used by another app`, the log has one line, and the rest of the app works.
- AC4: On Wayland the field is disabled with the note `Global shortcuts aren't available on
  Wayland`.
- AC5: No idle cost: the OS delivers the key; nothing polls.

## WR-14 Popup helpers

- AC1: **Pin.** The Pin pill keeps the popup open on focus loss until it's unpinned, Escape is
  pressed, or the popup is toggled closed. The pill shows its pinned state, and its accessible
  name says pinned or not pinned. Pinning isn't saved; the popup opens unpinned.
- AC2: **Day tooltip.** Resting the pointer on a day for 500 ms shows a small card: BS long
  date, AD long date, weekday, tithi, holiday name and the number of events, each line only when
  it has data. It hides when the pointer leaves the day or a key is pressed. The day's
  accessible name already carries the same text (WR-10 AC2).
- AC3: **Go to date.** Ctrl+G, or the Go to date entry in the month title's menu, opens one
  field in the header. It accepts a BS date (lenient parsing, as WR-07 AC1) or an AD date
  (`YYYY-MM-DD` or `YYYY/MM/DD`; with four-digit years, a year 1844 to 2143 that is not a valid
  BS year is read as AD). Enter opens that month with the day's detail. Errors follow WR-07
  AC2 and AC3. Escape closes the field.
- AC4: **Upcoming holidays.** The next-holiday line (WR-03 AC8) is a button. It expands a list of
  the next five public holidays within the patro coverage, each with its name, BS date and `in N
  days`. Clicking one opens its month and detail. Escape or a second click collapses it.

## WR-15 No branding in the app

- AC1: The app shows no third-party logo or badge (ADR-0020). Settings → About lists the
  third-party notices as plain text.
- AC2: The "Made with Slint" badge is in `apps/mitibar/README.md`, in the root `README.md`
  MitiBar section, and in every `mitibar-v*` release's notes. The release workflow adds it to
  the notes, and `xtask`'s workflow test fails if it doesn't.
