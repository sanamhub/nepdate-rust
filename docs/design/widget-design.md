# Widget design tokens

Design decisions for `apps/mitibar`, fixed before the first `.slint` file. Requirements:
ADR-0008 and [`widget-requirements.md`](widget-requirements.md). Fonts: ADR-0010. License:
ADR-0013.

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

## 5. Windows taskbar text (WR-12, ADR-0016)

Two lines, 12 px at 100 % scale, line 1 regular, line 2 semibold, line gap 2 px. Latin in Noto
Sans, Devanagari in Noto Sans Devanagari (12 px is the Devanagari minimum of §4, so it is not
reduced at any scale). Colour: `ink` for the taskbar's theme (light taskbar: light `ink`, dark
taskbar: dark `ink`), no accent colours, no background (the taskbar's material shows through).
Left inset 12 px. It should read like the system clock, not like a badge.

## 6. Principles

- Vermillion only for "today" and public holidays.
- Flat surfaces, no decorative shadows or gradients. Borders (`hairline`) do the structural work.
- One memorable element: the month grid. Keep the header and footer quiet.
