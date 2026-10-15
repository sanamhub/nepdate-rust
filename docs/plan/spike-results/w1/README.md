# W1 spike results (popup and tray icons)

Generated on Windows with the MSVC toolchain: the popup with
`--popup --snapshot <png> --theme light|dark --lang en|ne`, the tray icons with
`cargo xtask icons` (`tray-icons.png` puts days 1, 15, 22 and 32 of each digit set on a light
taskbar colour, top row, and a dark one, bottom row, at 2x).

Reviewed by eye on 2026-10-15 (quality bar section 2a).

| File | What it shows | Verdict |
|---|---|---|
| `popup-light-en.png` | popup, light theme, English | PASS |
| `popup-light-ne.png` | popup, light theme, Nepali | PASS |
| `popup-dark-en.png` | popup, dark theme, English | PASS |
| `popup-dark-ne.png` | popup, dark theme, Nepali | PASS |
| `tray-icons.png` | tray day icons, en and ne, light and dark taskbar | PASS, also checked at 16 px |

The popup header is the `nepdate` long format (`Ashoj 29, 2083` / `असोज २९, २०८३`); weekday and
AD date come from the same local date (WR-05 AC4). These snapshots show one clock line because
this machine's zone is Asia/Kathmandu, the default extra zone, so WR-05 AC1 hides it; the unit
tests cover the two-line case.

Review history: the first W1 icons were white only (invisible on a light taskbar) and thin at
16 px, and the popup stretched its four rows over the full height. The icons now come in two
inks, `ink` for each taskbar scheme, thickened with a stroke, and the popup stacks from the top.
