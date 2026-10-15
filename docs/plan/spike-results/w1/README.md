# W1 spike results (popup and tray icons)

Generated with the W1 snapshot tool (`--popup --snapshot <png> --theme light|dark --lang en|ne`)
and `cargo xtask icons`, on Windows with the MSVC toolchain. The popup PNGs show today's header
and clock lines; the tray PNGs are the pre-rendered day-number icons.

The visual verdicts are the human reviewer's (quality bar §2a); the agent could not view the
images, so nothing here is marked PASS.

| File | What it shows |
|---|---|
| `popup-light-en.png` | popup, light theme, English |
| `popup-light-ne.png` | popup, light theme, Nepali |
| `popup-dark-en.png` | popup, dark theme, English |
| `popup-dark-ne.png` | popup, dark theme, Nepali |
| `tray-en-1.png`, `tray-en-15.png`, `tray-en-32.png` | tray day icons, English digits (1, 15, 32) |
| `tray-ne-1.png`, `tray-ne-15.png`, `tray-ne-32.png` | tray day icons, Devanagari digits (१, १५, ३२) |

The popup header is the `nepdate` long format (`Ashoj 8, 2083` / `असोज ८, २०८३`), the weekday
and AD date come from the same local date (WR-05 AC4), and the clock lines follow WR-05 AC1 to
AC3. The tray icon is white ink on a transparent background, the notification-area convention.
