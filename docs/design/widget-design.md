# Widget design tokens

Design decisions for `apps/mitibar`, fixed before the first `.slint` file. Requirements:
ADR-0008 and [`widget-requirements.md`](widget-requirements.md). Fonts: ADR-0010. License:
ADR-0013. Sections 5 to 9 were revised on 2026-10-22 after the beta test (ADR-0019,
ADR-0020).

## 1. Subject

A small tray-first Nepali calendar widget. The month grid is the memorable element: a red-letter
Nepali patro (पात्रो), where "today" and public holidays are marked in vermillion, the way a
printed Nepali calendar marks them. Everything around the grid stays quiet and flat.

## 2. Colour tokens

One accent. Vermillion (`sindoor`) is reserved for "today" and public holidays. Nothing else is
red. Marigold marks events. Saturday gets a cool slate so weekend and holiday never share a hue.

| Token | Light | Dark | Use |
|---|---|---|---|
| `surface` | `#FDFBF8` | `#16120E` | window background (lokta paper white) |
| `surface-raised` | `#FFFFFF` | `#201B16` | cards, day cells, input fields |
| `ink` | `#211C17` | `#F1ECE4` | primary text |
| `ink-muted` | `#6E655C` | `#9B9288` | secondary text, AD dates, labels |
| `sindoor` | `#C12B2B` | `#E05B5B` | today's outline, public holiday text |
| `marigold` | `#E8A33D` | `#E8A33D` | event dot, holiday badge |
| `saturday` | `#5B7C99` | `#7FA3C4` | Saturday day numbers |
| `hairline` | `#E7E0D6` | `#2C2620` | borders, dividers |

Dark `sindoor` is lighter than light `sindoor` so it keeps enough contrast on the near-black
surface. The other accents stay the same in both modes.

## 3. Type scale and spacing

Two bundled families, one role each: Noto Sans for Latin (AD dates, clock, chrome), Noto Sans
Devanagari for Nepali. Both OFL-1.1 (ADR-0010).

| Step | Size | Use |
|---|---|---|
| `caption` | 11 px | event labels, small AD month names |
| `body` | 13 px | AD day in a cell, secondary text |
| `base` | 14 px | clock, buttons, input text |
| `title` | 16 px | month title, day detail heading |
| `display` | 22 px | header BS day and long date |

Spacing on a 4 px grid: 4, 8, 12, 16, 24. Popup padding 12, grid cell padding 4, gap between
sections 16.

## 4. Devanagari and Latin together

- Devanagari is first class. Latin (AD dates, clock) uses `ink-muted` so the BS date reads as
  primary.
- Line height: Latin 1.35, Devanagari 1.55. Matras, the reph and conjuncts need the extra
  vertical room.
- Minimum Devanagari size is 12 px. Below that, vowel signs clip. The 11 px caption step is
  Latin only; a Devanagari caption uses 12 px.
- No uppercase on Latin labels (sentence case). Nepali has no case, so this keeps the two
  scripts visually level.
- Digits follow the language: Devanagari for `ne`, ASCII for `en`. `nepdate` provides both via
  `Lang::digits`.

## 5. Mini bar (WR-08, ADR-0019)

A pill that reads like the system clock, not like a badge. It floats over the taskbar or the
desktop, so it carries its own quiet surface instead of relying on the taskbar's material.

```
╭──────────────────────────────────────╮
│  Thu · 6:32 PM · Singapore 8:46 PM   │   line 1: regular
│  Kartik 5, 2083 · Oct 22             │   line 2: semibold
╰──────────────────────────────────────╯
```

- Type: 12 px at 100 % scale, line 1 regular, line 2 semibold, line gap 2 px. Noto Sans for
  Latin, Noto Sans Devanagari for Nepali (12 px is the §4 minimum, so it's never reduced).
- Text is left-aligned. Separators are ` · ` in `ink-muted`; everything else is `ink`.
- Shape: radius is half the pill's height. Padding 10 px left and right, 2 px top and bottom,
  so two lines stay under 44 px and fit a 48 px taskbar. One line gives a 26 px pill.
- Surface: `surface` at 90 % opacity with a 1 px `hairline` border, for the **system**
  (taskbar) theme. Hover raises it to `surface-raised`. No shadow; dragging doesn't change the
  look, only the position.
- Width follows the text; it's measured once per text change, never per frame.

## 6. Shapes

One radius per role, so hierarchy shows in the shape:

| Role | Radius |
|---|---|
| Popup and Settings windows | 8 px (Windows 11's own window radius; 10 px on macOS and Linux) |
| Day cells, month and year cells, cards, list rows | 6 px |
| Buttons, segmented controls, switches, chips, the bar | full (half the height) |

Hover is a `surface-raised` fill. Focus is the 2 px `ink` ring from W7, drawn outside the
shape with the same radius.

## 7. Popup layout (WR-02, WR-03)

```
╭───────────────────────────────────────────────╮
│ Kartik 5, 2083          (⇄ Convert) (⚙) (⌖)   │  display 22 px; pills right
│ Thursday · Oct 22, 2026                        │  ink-muted
│ 6:31 PM Kathmandu · 8:46 PM Singapore          │  base 14 px
│───────────────────────────────────────────────│
│ ‹  Kartik 2083  ›                  (Today)     │  title is the zoom button
│ Sun  Mon  Tue  Wed  Thu  Fri  Sat              │
│  1    2    3    4   (5)   6    7               │  (5) = sindoor disc
│ ...                                            │
│───────────────────────────────────────────────│
│ Next holiday: Dhanteras in 14 days         ⌄  │  expands to five
╰───────────────────────────────────────────────╯
```

- Header: left-aligned text, the three pill actions on the right of the first line. Convert has
  an icon and a label; Settings and Pin are icon-only pills with tooltips and accessible names.
  A pinned Pin pill is filled with `ink`, its icon in `surface`.
- Pills: 28 px high, `surface-raised` fill, 1 px `hairline` border, `base` text. They are the only
  button style in the popup, so actions are easy to spot.
- The month title is plain `title` text that turns into a pill on hover and focus. The previous
  and next arrows are borderless icon buttons. The Today pill shows only away from today.
- Grid: the BS number is the largest text in each cell, the AD day sits top-right in `caption`
  `ink-muted`, the tithi under the number in `caption`. Today's number sits on a filled 28 px
  `sindoor` disc with the numeral in `surface-raised`; the pair must reach 4.5:1 in both themes.
  Holidays are `sindoor` numerals. The event dot is `marigold`.
- Year and decade views use the same cell radius and a 4×3 grid. Today's month or year gets the
  `sindoor` treatment; the viewed one gets a `hairline` ring.
- Day tooltip: a `surface-raised` card, 6 px radius, `hairline` border, `body` text, placed so
  it never covers the hovered day.

## 8. Settings window (WR-06)

420 × 560 logical px, one left-aligned column, scrolls when needed.

```
Settings                                  Saved ✓
─────────────────────────────────────────────────
General
  Language          ( English | नेपाली )
  Theme             ( System | Light | Dark )
  Launch at login                       [●──]
Clock
  24-hour clock                         [──○]
  Show seconds                          [──○]
  Extra clock       [ Singapore              ⌄ ]
Bar
  Show bar                              [●──]
  Lock position                         [──○]
  Top line     (Thu ×) (Time ×) (Zone ×) (+ Add field)
  Bottom line  (BS date ×) (AD short ×)  (+ Add field)
  Reset to default      Reset position
Shortcut
  Open MitiBar      [ Win+Shift+N ]  Clear  Reset
About
  MitiBar 0.1.0 · MIT license · Third-party notices
```

- Section headings are `title`, sentence case, no rules between rows; one `hairline` divider
  under the header only.
- Rows: label left in `ink`, control right-aligned. Help text, when a row needs it, is `body`
  `ink-muted` under the label.
- Segmented control: a `surface-raised` pill track, the selected segment filled with `ink` and its
  text in `surface`.
- Switch: 36 × 20 px pill. On is an `ink` track with a `surface` knob; off is a `hairline` track
  with an `ink-muted` knob. No other colour: `sindoor` stays reserved for the calendar.
- Chips: full radius, `surface-raised`, `hairline` border, `body` text, a remove button inside.
- The `Saved` note is `body` `ink-muted` with a check mark; the failure note is `ink`.

## 9. Principles

- Vermillion only for "today" and public holidays.
- Flat surfaces, no decorative shadows or gradients. Borders (`hairline`) do the structural work.
  The only shadow is the one Windows 11 draws for every window.
- One memorable element: the month grid. Keep the header, the bar and Settings quiet.
- Controls are monochrome (`ink` on `surface`), so colour in the window always means calendar
  data.
