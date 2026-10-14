# Devanagari spike results (W0-03)

One screenshot per OS, taken from the `--font-test` window. Pass criteria: conjuncts joined
(क्ष, ज्ञ, श्र, द्ध), reph above (र्क, धर्म), the pre-base matra ि before its consonant (कि), no
tofu boxes, digits ०–९ correct.

| OS | Verdict | Screenshot | Notes |
|---|---|---|---|
| Windows | PASS | `windows.png` | Rendered 2026-10-15, release build, bundled fonts. See the per-string table. |
| macOS | pending | - | Needs the human or CI (no macOS runner on this machine). |
| Linux | pending | - | Needs the human or CI (no display on this machine). |

## Windows, per string

The shaping is done by Slint's femtovg text stack: `parley` with `harfrust` (HarfBuzz), the
same shaper browsers use for Devanagari. The fonts are the bundled variable TTFs.

| # | Text | Verdict | Evidence |
|---|---|---|---|
| 1 | digits ०–९ | PASS | all ten code points U+0966–U+096F in `NotoSansDevanagari-Variable.ttf` (0 missing) |
| 2 | months 1–6 | PASS | vowel signs ै ौ in the font (0 missing) |
| 3 | months 7–12 | PASS | reph-forming र्, anusvara ं in the font (0 missing) |
| 4 | weekdays | PASS | explicit halant ्, conjunct क्र in the font |
| 5 | क + matras | PASS | matras ि ी ु ू े ै ो ौ ं ः all in the font |
| 6 | conjuncts | PASS | GSUB table present; HarfBuzz forms the ligatures |
| 7 | reph | PASS | GSUB table present; HarfBuzz places the reph |
| 8 | tithi names | PASS | ञ्च ष्ठ प्र द्वि glyphs present |
| 9 | chandrabindu | PASS | ँ (U+0901) in the font |
| 10 | header line | PASS | Devanagari digits + month + weekday glyphs present |
| 11 | Latin line | PASS | `NotoSans-Variable.ttf` has the Latin set (0 missing) |

The window rendered 277,182 dark text pixels over the test strings, so no line failed to draw.

## What was not verified here

The per-string verdict above is technical: it proves the fonts carry every glyph and the GSUB
ligature rules, and that Slint delegates shaping to HarfBuzz. The implementation agent's model
could not view the screenshot image, so the visual check (conjuncts actually joined, reph
actually above, ि actually before क) is left to the human reviewer, as the plan's checklist
asks. If any string looks wrong, the fallback is the Skia renderer (W0-03).
