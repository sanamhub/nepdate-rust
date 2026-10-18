# W2 spike results (month grid, navigation, day detail)

Generated on Windows 11 with the MSVC toolchain on 2026-10-18. Each PNG is the popup rendered by
`--popup --snapshot <png> --theme light|dark --lang en|ne --today <BS date>`, with
`SLINT_SCALE_FACTOR=1` or `2`. `--today` pins the calendar date and the clock to 12:00, so
re-rendering an image does not change it. The eight month grids show Kartik 2083 (`--today
2083-07-01`); the six-row pair shows Baishakh 2081 (`--today 2081-01-15`); the detail pair shows
Baishakh 1, 2081, a Saturday and a public holiday (`--today 2081-01-01 --detail 2081-01-01`); the
max-events card shows Ashoj 1, 2081 (`--today 2081-06-01 --detail 2081-06-01`), the day with nine
events.

`--today` and `--detail` exist only in the `snapshot` feature, and the parser rejects them without
`--popup --snapshot`, so the normal app can never pin its date.

The agent cannot judge the images, so every verdict is **not verified**: the reviewer looks.

| File | What it shows | Verdict |
|---|---|---|
| `month-grid-light-en-1x.png` | light theme, English, 100 % | not verified |
| `month-grid-light-en-2x.png` | light theme, English, 200 % | not verified |
| `month-grid-light-ne-1x.png` | light theme, Nepali, 100 % | not verified |
| `month-grid-light-ne-2x.png` | light theme, Nepali, 200 % | not verified |
| `month-grid-dark-en-1x.png` | dark theme, English, 100 % | not verified |
| `month-grid-dark-en-2x.png` | dark theme, English, 200 % | not verified |
| `month-grid-dark-ne-1x.png` | dark theme, Nepali, 100 % | not verified |
| `month-grid-dark-ne-2x.png` | dark theme, Nepali, 200 % | not verified |
| `month-grid-6row-light-en-1x.png` | 6-row Baishakh 2081, light, English, 100 % | not verified |
| `month-grid-6row-light-ne-2x.png` | 6-row Baishakh 2081, light, Nepali, 200 % | not verified |
| `day-detail-light-en-1x.png` | day detail, 3 events, light, English, 100 % | not verified |
| `day-detail-dark-ne-2x.png` | day detail, 3 events, dark, Nepali, 200 % | not verified |
| `day-detail-max-light-en-1x.png` | day detail, 9 events, light, English, 100 % | not verified |

Layout: a two-line header, the title row (previous / title / next), the month and year pickers plus
Today, the Sunday-first weekday row, the 7-column grid, and the next-holiday line. The popup is
360×520. Each cell is 44 px high with 4 px padding and a 2 px grid gap: line 1 has the BS day
(16 px) at the left and the AD day (13 px Latin, 12 px Devanagari) at the right, with the 11 px
short AD month replacing the day on a 1st; line 2 is the tithi (12 px). The 2 px gap is narrower
than the 4 px grid because a 4 px gap left the "Nov" marker touching the numeral at 1x. The today
ring is 2 px and keeps 2 px clear of the glyphs. The month picker is 128 px and the year 112 px,
which keeps Today inside the 24 px margins in both languages.

The day detail is an inline card over a scrim, not a new window. The cards show the BS and AD long
dates, weekday, tithi, the public-holiday badge and the event list. The event items sit at the top
with 2 px spacing and the list scrolls inside its 120 px box only when it is taller; the max card
shows nine events. The copy buttons and the Escape order were not run.
