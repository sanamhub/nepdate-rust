# ADR-0008: Widget scope (minimal) and use of NepDateWidget

- Status: Accepted
- Date: 2026-09-24 (revised 2026-09-25: permission from the NepDateWidget author, see A)
- Deciders: Sanam Pakuwal

## Context

### Upstream projects

| Upstream | License | Our use |
|---|---|---|
| NepDate (library, data) | **MIT** | Port code and data, keep the copyright notice (`shared/data/NOTICE`, `LICENSE-THIRD-PARTY`) |
| NepDateWidget (WPF app) | **PolyForm Strict 1.0.0** | The author, Raju Prasai, has given permission to use it as the reference for this Rust widget. The shared goal from the start is to bring NepDate to other platforms. Recorded in `docs/legal/README.md` |

### Scope

NepDateWidget has ~44k lines: calendar, converters, unit/text/banking/network tools,
notes, reminders, documents, RunBox launcher, ten theme presets, twenty fonts,
backup/restore, etc. The goal here is a **lightweight, fast, cross-platform** widget
with only the essentials ("no feature slop").

## Decision

### A. How NepDateWidget is used

1. The source of truth for the widget is `docs/plan/widget-requirements.md`, plus the
   `nepdate` and `nepdate-patro` crates. Agents build what it says, and nothing more.
2. Agents **may** read NepDateWidget (https://github.com/RajuPrasai/NepDateWidget) as a
   reference for behaviour, wording and Nepali translations. It is WPF/C#, so nothing is
   ported line by line. Its extra features stay out of scope (B).
3. UI strings live in the `.slint` files as `@tr("English text")`,
   with the Nepali translation in `apps/nepdate-widget/i18n/ne/LC_MESSAGES/nepdate-widget.po`,
   bundled at build time (Slint bundled translations, since 1.9). The language switches at runtime with
   `slint::select_bundled_translation("ne")`. This needs no extra crate, and `@tr` bindings
   update live. A unit test fails if any `msgstr` in the `.po` file is empty. Rust code
   produces **data** (numbers, dates formatted by `nepdate`), never UI sentences.
   Month/weekday names come from the `nepdate` crate.

### B. Widget MVP scope (v0.1)

In scope:

1. **Tray / menu-bar icon** (primary entry point on all OSes). Shows today's BS day
   (icon image on Windows/Linux; text title on macOS). Menu: Open, Copy today (BS),
   Copy today (AD), Settings, Quit.
2. **Popup window** (borderless, opens next to the tray icon, closes on focus loss):
   - Today header: BS long date, AD date, weekday, clock(s).
   - **Month grid** for BS months: Sunday-first, AD day number in each cell, Saturday
     and public-holiday tint, tithi (when data exists), dot for events.
   - Prev / next / today navigation, month + year pickers, mouse-wheel month change.
   - Day detail on click: tithi, holiday flag, events list.
   - "Next holiday in N days" line when the patro data covers it.
3. **Converter** panel: BS→AD (lenient BS parsing, `shared/spec/PARSING.md` §2) and AD→BS,
   and days-between two dates.
4. **Clock**: local time + one extra IANA time zone (default `Asia/Kathmandu`),
   12h/24h, optional seconds.
5. **Settings**: language (en/ne), theme (system/light/dark), clock format, seconds,
   extra time zone, launch at login.
6. Single instance: launching again opens the popup of the running instance.
7. Launch at login (per-user; no admin rights).
8. **Privacy**: the app makes no network requests at all: no telemetry, no update
   checks, no remote data.

Out of scope for v0.1 (each needs a new ADR to add): **floating always-on-top mini bar**
(first v0.2 candidate; deferred because Wayland can't position windows or keep them on top, and
skip-taskbar needs per-OS hacks, while the tray already keeps today's date visible),
unit/text/banking/network tools,
notes, reminders, notifications, documents, launcher/hotkeys, theme presets, font
picker, backup/restore, fiscal-year footer, auto-update, taskbar-embedding hacks,
custom holiday editing.

### C. Non-functional targets (verified in plan phase W4)

| Metric | Target |
|---|---|
| Release binary size (Windows x64, stripped) | ≤ 15 MB (stretch ≤ 8 MB) |
| Idle RSS with popup closed | ≤ 40 MB |
| Cold start to tray icon visible | ≤ 500 ms on a mid-range laptop |
| Idle CPU | ~0 % (wake at most once per minute when seconds hidden) |

## Consequences

- The widget is a small cross-platform sibling of NepDateWidget, not a full port.
- Tray-first design is new relative to the original (which had no tray icon) and
  works on all three OSes, unlike taskbar overlay tricks.
- Keeping the scope list here means agents can reject feature requests that are
  not in it.
