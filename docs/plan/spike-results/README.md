# Devanagari spike results (W0-03)

One screenshot per OS per scheme, produced by `cargo run -p nepdate-widget -- --font-test
--snapshot <path.png> --theme light|dark` (Slint's `Window::take_snapshot` API, no manual
screen capture). The pass criteria are visual: conjuncts joined (क्ष, ज्ञ, श्र, द्ध), reph
above (र्क, धर्म), the pre-base matra ि before its consonant (कि), no tofu boxes, digits ०–९
correct.

Glyph coverage, GSUB-table presence and pixel counts are **not** a visual verdict (quality bar
§2a), so no OS is marked PASS here until a human has looked at the PNGs.

| OS | Verdict | Screenshot | Notes |
|---|---|---|---|
| Windows | not verified | `windows-light.png`, `windows-dark.png` | Generated 2026-10-15 with the MSVC toolchain and the bundled fonts. Text is readable in both schemes: ink on paper, light and dark. |
| macOS | pending | - | Needs the human or CI (no macOS runner on this machine). |
| Linux | pending | - | Needs the human or CI (no display on this machine). |

## Windows, per string

| # | Text | Verdict | What the reviewer checks |
|---|---|---|---|
| 1 | digits ०–९ | not verified | all ten digits correct |
| 2 | months 1–6 | not verified | vowel signs ै, ौ placed |
| 3 | months 7–12 | not verified | reph र्त, anusvara ं placed |
| 4 | weekdays | not verified | ि before consonant, conjunct क्र joined |
| 5 | क + matras | not verified | ि appears left of क |
| 6 | conjuncts | not verified | क्ष, ज्ञ, श्र, द्ध joined |
| 7 | reph | not verified | र् above ध, र्म |
| 8 | tithi names | not verified | ञ्च, ष्ठ, प्र, द्वि joined |
| 9 | chandrabindu | not verified | ँ placed |
| 10 | header line | not verified | Devanagari digits + month + weekday |
| 11 | Latin line | not verified | Noto Sans fallback in the same line |

If any string looks wrong, the fallback is the Skia renderer (W0-03), a feature flag change
with no code change (ADR-0010).
