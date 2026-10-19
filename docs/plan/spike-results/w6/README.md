# W6-01 Devanagari exit check

Result: **pass** on Windows x64. No conjunct, reph or matra was wrong, so the condition in
ADR-0018 is met and W6-02 may switch the renderer.

The check renders every string in `docs/plan/devanagari-test-strings.md` through the font-test
window and every committed W2/W3 popup snapshot with Slint's software renderer. Each software
image is placed left of the committed femtovg image in `sheets/<name>.png`. The sheets are made
by `render_sheets.py`, not by hand.

## How the images were made

Built on 2026-10-19 on Windows 11 x64 with the MSVC toolchain:

```text
cargo build -p mitibar --features snapshot
python docs/plan/spike-results/w6/render_sheets.py \
    --binary D:/devcache/cargo-target-w6/debug/mitibar.exe
```

The script sets `SLINT_BACKEND=winit-software` and `SLINT_SCALE_FACTOR=1` or `2` for every
software image. It uses the same snapshot flags recorded in the W2 and W3 READMEs, for example
`--popup --snapshot <png> --theme light --lang ne --today 2083-07-01`. It writes a temporary
file per render and keeps only the sheets and `diff.json`.

The right side is the femtovg image preserved under `femtovg/`. There is one reference for each
of the 55 sheets: the 33 images committed at W6-01, the 18 W2 combinations that were not
committed before, and the four font-test images at 1x/2x in light and dark. W6-02 and W6-03
later replaced the W2/W3 PNGs with the software render and the visible year buttons; these
references keep this check reproducible. Every sheet labels the
right side "femtovg (W6-01 reference)".

The font-test window has no language flag. It holds both the Devanagari strings and the Latin
fallback line, so its four sheets cover the strings at 1x and 2x in light and dark. The
English/Nepali split is covered by the W2/W3 popup sheets.

## What was rendered

| Group | Sheets | Combinations |
|---|---|---|
| Month grid, Kartik 2083 | `month-grid-*.png` | 8: light/dark, en/ne, 1x/2x |
| Six-row month, Baishakh 2081 | `month-grid-6row-*.png` | 8: light/dark, en/ne, 1x/2x |
| Day detail, Baishakh 1, 2081 | `day-detail-*.png` | 8: light/dark, en/ne, 1x/2x |
| Day detail, Ashoj 1, 2081, 9 events | `day-detail-max-*.png` | 8: light/dark, en/ne, 1x/2x |
| Converter | `converter-*.png` | 8: light/dark, en/ne, 1x/2x |
| Settings | `settings-*.png` | 8: light/dark, en/ne, 1x/2x |
| Tithi width | `tithi-*.png` | 3: before light en 1x, after light en 1x, after light ne 1x |
| Font test | `font-test-*.png` | 4: light/dark, 1x/2x |

55 sheets in total: 48 popup sheets at all eight combinations of theme, language and scale,
three tithi-width popup images, and four font-test images. The 33 committed W2/W3 images keep
their original flags; the other 18 fill the combinations the committed set did not cover.

`tithi-before-light-en-1x.png` is a historical layout image, not a current snapshot. The left
side is the current code, so it differs from the right side by the known W3-00b change: the
tithi sits at a 4 px inset and elides earlier in the historical image. The glyph shapes are
still readable in both. Every other W2/W3 image is reproduced by the current code.

## Per-glyph list

"Same" means the letterform, join, advance, line break and position match. The two rasterizers
are not pixel-identical: software antialiasing is slightly heavier at 1x, and at 2x the
difference is hard to see at native size. `diff.json` records 0.5 to 5.2 mean absolute channel
delta per sheet. That is rasterization, not a different glyph.

| Element | Where | 1x | 2x | Verdict and reason |
|---|---|---|---|---|
| क्ष | font test row 6 | same | same | joined, same width and baseline |
| ज्ञ | font test row 6 | same | same | joined, same width and baseline |
| श्र | font test row 6 | same | same | joined, same width and baseline |
| र्क (in अर्को) | month grid ne, next-holiday line | same | same | reph attached above क; no detached mark |
| कि | font test row 5 | same | same | pre-base ि sits left of क |
| की | font test row 5 | same | same | pre-base ि and length mark placed |
| कु | font test row 5 | same | same | below-base ु attached |
| कू | font test row 5 | same | same | below-base ू attached |
| के | font test row 5 | same | same | े above |
| कै | font test row 5 | same | same | ै above |
| को | font test row 5 | same | same | ो right |
| कौ | font test row 5 | same | same | ौ right |
| कं | font test row 5 | same | same | anusvara above |
| कः | font test row 5 | same | same | visarga right |
| क्र (in शुक्रवार) | font test row 4 | same | same | क and र joined below the shirorekha |
| ङ्ग (in मङ्गलवार) | font test row 4 | same | same | explicit halant kept, ग attached |
| बि (in बिहिवार) | font test row 4 | same | same | ि left of ब |
| र्म (in धर्म, कर्म) | font test row 7 | same | same | reph over म, same height |
| त्र | font test row 6 | same | same | joined |
| द्ध | font test row 6 | same | same | joined |
| द्व | font test row 6 | same | same | joined |
| ह्म | font test row 6 | same | same | joined |
| ट्ट | font test row 6 | same | same | joined |
| ञ्च (in पञ्चमी) | font test row 8, grid ne | same | same | joined |
| ष्ठ (in षष्ठी) | font test row 8, grid ne | same | same | joined |
| प्र (in प्रतिपदा) | font test row 8, grid ne | same | same | joined |
| द्वि (in द्वितीया) | font test row 8, grid ne | same | same | joined, ि left |
| ँ (in नयाँ) | font test row 9 | same | same | chandrabindu above |
| ं (in संक्रान्ति) | month grid ne | same | same | anusvara above |
| ० १ २ ३ ४ ५ ६ ७ ८ ९ | font test row 1, headers | same | same | ten digits, same shapes |

No entry is "differs, acceptable" and none is "wrong".

## Evidence and limits

- [diff.json](diff.json) has the size, changed-pixel count, maximum channel delta and mean
  absolute delta for every sheet. It is a check that the layouts line up, not a quality score.
- [sheets/](sheets/) holds the 55 side-by-side images, software left and femtovg right.
- [render_sheets.py](render_sheets.py) is the generator. It needs Pillow and runs from the
  repository root.
- Only Windows x64 was checked. macOS and Linux Devanagari rendering is still **not verified**
  (the W0 table keeps those OS rows pending).
- The images use the bundled Noto Sans and Noto Sans Devanagari fonts. System-font fallback was
  not part of this check.
- 1x and 2x are Slint scale factors, not a per-monitor DPI change. The W6-05 manual checklist
  asks for a 100 % and 200 % look on the three OSes.
- The 1x software text is slightly heavier than femtovg. It is not blurry, clipped or misplaced,
  and the reviewer should still look at the sheets before calling the visual quality accepted.
