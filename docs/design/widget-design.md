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
| `title` | 16 px | day detail heading, year and decade cell labels |
| `display` | 20 px | header BS long date |

Spacing on a 4 px grid: 4, 8, 12, 16, 24. Popup padding 12, day cell content inset 1.5 to 2, gap between
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
- Shape: a rounded rectangle with a 6 px radius (scaled with DPI), which sits with the Windows 11
  shell better than a capsule; the owner picked it after seeing both on 2026-10-23. Padding 10 px
  left and right, 2 px top and bottom, so two lines stay under 44 px and fit a 48 px taskbar. One
  line gives a 26 px bar.
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
| The mini bar | 6 px |
| Buttons, segmented controls, switches, chips | full (half the height) |

Hover is a `surface-raised` fill. Focus is the 2 px `ink` ring from W7, drawn outside the
shape with the same radius.

### Day cells (W8-4a)

Decided after the owner's first run of the W8-3 build, with the numbers each choice came from.

- **Size.** Seven columns with no gap between them, so a cell is 48 px wide instead of 46.3 px.
  The rows share the height the grid is given, 64 px in a five-row month and 53 px in a six-row
  one, and the content block (28 px for the numeral row, up to 26 px for the tithi) sits in the
  middle of the cell. Nothing is left over above the holiday line.
- **The numeral row.** The BS numeral is 16 px, its middle 14 px from the cell's left edge, and
  today's 28 px disc is flush with the edge. The AD day (13 px, 12 px in Devanagari) is
  right-aligned 1.5 px from the right edge. Both stand on one baseline: each text starts one font
  ascent above a line 20 px down, read from the font's own metrics, so the baseline holds in both
  scripts. The gap between the numeral's box and the AD day's box is at least 4 px, and at least
  2 px where an AD month stands in for the day.
- **The AD month** (WR-03 AC2) is 11 px with -0.5 px tracking, so `May`, the widest at 22 px, still
  clears a two-digit numeral by 2 px. **Today's cell keeps the AD day:** the disc's right edge is
  at 28 px and the month would start at 24 px, so it would be drawn half on the disc, and the
  header already gives today's AD date. This is the one place AC2 is not followed to the letter.
- **The event dot** is 4 px, 1 px after the numeral's last digit and level with the numeral's top,
  so it is the same distance from a one-digit and a two-digit day. On today's cell it clears the
  disc instead.
- **The tithi.** The longest Latin names do not fit a cell on one line at the 11 px caption step
  (`Chaturdashi` is 63 px, `Chaturdashi*` 69 px; the room is about 44 px between the focus ring's
  2 px on each side), and they cannot shrink: 11 px is the Latin floor, 12 px the Devanagari one.
  An ellipsis made `Chaturthi` and `Chaturdashi` both read `Chatur…`. `nepdate` has short forms
  for months and weekdays but none for tithis, and inventing them would be a second spelling of
  every name. So a name that does not fit breaks at a syllable on two lines (`Chatur-` over
  `dashi`), and `nepdate-patro`'s ` (K)` for a skipped tithi is shown as the `*` it is already in
  Devanagari. The full name stays in the tooltip, the day detail and the cell's accessible name.
  Devanagari names fit one line at 12 px. The table of breaks is `app/tithi_label.rs`; a test
  puts every name of the data on screen in both scripts and measures it.
- **What the choice costs.** Two lines of tithi need the cell's 54 px of content, which a
  six-row month's 53 px cell just holds, so the line spacing is 0.78 of the font's (12 px at
  11 px) and the first line starts 28 px down.

## 7. Popup layout (WR-02, WR-03)

```
╭───────────────────────────────────────────────╮
│ Kartik 5, 2083          (⇄ Convert) (⚙) (⌖)   │  display 20 px; pills right
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

- Header: left-aligned text, the three pill actions on the right of the first line. The date is
  `display`, 20 px, the largest size that fits "Baishakh 31, 2081" beside the pills; the Convert
  label is 13 px for the same reason. A click on the date opens Go to date (WR-14 AC3); on hover
  it gets the borderless pill of the month title. Convert has an icon and a label; Settings and Pin are
  icon-only pills with tooltips and accessible names. A pinned Pin pill is filled with `ink`, its
  icon in `surface`.
- Pills: 28 px high, `surface-raised` fill, 1 px `hairline` border, `base` text. They are the only
  button style in the popup, so actions are easy to spot. Text pills pad 10 px each side. With an
  icon the left padding is 8 px, because an icon's drawing leaves about 2 px of air inside its
  16 px box, so the ink is 10 px from each edge; an icon-only pill centres the icon.
- The month title is plain `base` text that turns into a pill on hover and focus. It is 14 px, not
  the 16 px `title` step, so the longest month title, the two arrows and the Today pill share one
  336 px row. The previous and next arrows are borderless icon buttons. The Today pill shows only
  away from today's month in the month view, and always in the year and decade views.
- Grid: the BS number is the largest text in each cell, the AD day sits top-right in `caption`
  `ink-muted`, the tithi under the number in `caption`. Today's number sits on a filled 28 px
  `sindoor` disc with the numeral in `surface-raised`; the pair must reach 4.5:1 in both themes.
  Holidays are `sindoor` numerals. The event dot is `marigold`. The cells share the height the
  grid is given (§6, Day cells), so a five-row month has no empty band above the holiday line. The
  grid is one Tab stop: its cursor, drawn with the 2 px `ink` ring, moves with the arrow keys.
- Year and decade views use the same cell radius and a 4×3 grid. Today's month or year gets the
  `sindoor` treatment; the viewed one gets a `hairline` ring.
- Day tooltip: a `surface-raised` card, 6 px radius, `hairline` border, `body` text, placed so
  it never covers the hovered day.

## 8. Settings window (WR-06)

420 × 560 logical px of content in the system's window frame (title `MitiBar Settings`, the close
button), one left-aligned column that scrolls when it needs more room.

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
  Extra clock       [ Asia/Singapore         ]
Bar
  Show bar                              [●──]
  Lock position                         [──○]
  Top line                         (+ Add field)
  (Weekday ×) (Time ×) (Extra clock time ×)
  Bottom line                      (+ Add field)
  (BS date ×) (AD date, short ×)
  (Reset to default) (Reset position)
Shortcut
  Open MitiBar
  [ Win+Shift+N                 ] (Clear) (Reset)
About
  MitiBar 0.1.0 · MIT license
  Third-party notices: Slint, the fonts, NepDate
```

- Section headings are `title`, sentence case, no rules between rows; one `hairline` divider
  under the header only.
- Rows: label left in `ink`, control right-aligned. Help text, when a row needs it, is `body`
  `ink-muted` under the label. The line editor and the shortcut field are wider than a row, so
  their label sits above them in `body` `ink-muted`.
- Segmented control: a `surface-raised` pill track, the selected segment filled with `ink` and its
  text in `surface`. It is one Tab stop; the arrow keys move the choice.
- Switch: 36 × 20 px pill. On is an `ink` track with a `surface` knob; off is a `hairline` track
  with an `ink-muted` knob. No other colour: `sindoor` stays reserved for the calendar.
- Chips: full radius, `surface-raised`, `hairline` border, `body` text, a remove button inside. A
  focused chip also shows its two arrow buttons. A line with more chips than fit scrolls sideways:
  the focused chip is kept in view, and the mouse wheel moves the line while it has room to move,
  then leaves the wheel to the page. "Add field" lists the unused fields under its line.
- The extra clock is one field: its drop-down is a `surface-raised` card of 28 px rows under it,
  6 px radius, the highlighted row filled with `hairline`.
- The shortcut field shows the combination; while it has focus it says `Press the keys`.
- The `Saved` note is `body` `ink-muted` with a check mark; the failure note is `ink`.
- A focused control is scrolled into view, with 8 px to spare.

## 9. Principles

- Vermillion only for "today" and public holidays.
- Flat surfaces, no decorative shadows or gradients. Borders (`hairline`) do the structural work.
  The only shadow is the one Windows 11 draws for every window.
- One memorable element: the month grid. Keep the header, the bar and Settings quiet.
- Controls are monochrome (`ink` on `surface`), so colour in the window always means calendar
  data.
