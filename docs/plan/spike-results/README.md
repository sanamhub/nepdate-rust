# Devanagari spike results (W0-03)

One screenshot per OS per scheme, produced by `cargo run -p nepdate-widget -- --font-test
--snapshot <path.png> --theme light|dark` (Slint's `Window::take_snapshot` API, no manual
screen capture). The pass criteria are visual: conjuncts joined (क्ष, ज्ञ, श्र, द्ध), reph
above (र्क, धर्म), the pre-base matra ि before its consonant (कि), no tofu boxes, digits ०–९
correct.

Glyph coverage, GSUB-table presence and pixel counts are **not** a visual verdict (quality bar
§2a). An OS is marked PASS only after a reviewer has looked at its PNGs.

| OS | Verdict | Screenshot | Notes |
|---|---|---|---|
| Windows | PASS | `windows-light.png`, `windows-dark.png` | Generated 2026-10-15 with the MSVC toolchain and the bundled fonts. Reviewed by eye on 2026-10-15, both schemes, rows 6 to 9 also at 2x zoom. |
| macOS | pending | - | Needs the human or CI (no macOS runner on this machine). |
| Linux | pending | - | Needs the human or CI (no display on this machine). |

## Windows, per string (reviewed by eye, 2026-10-15)

| # | Text | Verdict | What the reviewer checks |
|---|---|---|---|
| 1 | digits ०–९ | PASS | all ten digits correct |
| 2 | months 1–6 | PASS | vowel signs ै, ौ placed |
| 3 | months 7–12 | PASS | reph र्त, anusvara ं placed |
| 4 | weekdays | PASS | ि before consonant, conjunct क्र joined |
| 5 | क + matras | PASS | ि appears left of क |
| 6 | conjuncts | PASS | क्ष, ज्ञ, श्र, द्ध joined |
| 7 | reph | PASS | र् above ध, र्म |
| 8 | tithi names | PASS | ञ्च, ष्ठ, प्र, द्वि joined |
| 9 | chandrabindu | PASS | ँ placed |
| 10 | header line | PASS | Devanagari digits + month + weekday |
| 11 | Latin line | PASS | Noto Sans fallback in the same line |

If any string looks wrong, the fallback is the Skia renderer (W0-03), a feature flag change
with no code change (ADR-0010).
