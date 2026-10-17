# W2 spike results (month grid, navigation, day detail)

Generated on Windows 11 with the MSVC toolchain on 2026-10-18. Each PNG is the popup rendered by
`--popup --snapshot <png> --theme light|dark --lang en|ne`, with `SLINT_SCALE_FACTOR=1` or `2`.
The grid shows today's month (Kartik 2083) and the header uses the same local date.

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

Layout: a two-line header, the title row (previous / title / next), the month and year pickers plus
Today, the Sunday-first weekday row, the 7-column grid (31 px cells), and the next-holiday line.
A 6-row month (2081 Baishakh) was rendered locally and fits the 440 px popup with the holiday line;
that preview is not committed.

The day detail is an inline card over a scrim, not a new window; it was rendered in local previews
(light English and dark Nepali, 2x) and is not committed. The reviewer opens it from a day cell.
