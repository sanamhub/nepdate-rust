# W3 spike results

Generated on Windows 11 with the MSVC toolchain. Each PNG is the popup rendered by
`--popup --snapshot <png> --theme light|dark --lang en|ne --today 2081-01-01`, with
`SLINT_SCALE_FACTOR=1` or `2`. The converter images add `--converter` and the settings images add
`--settings`; both flags exist only in the `snapshot` feature and the parser rejects them without
`--popup --snapshot`. `--today` pins the date and the clock to 12:00, so a re-render is stable.

The 18 converter and settings images and the two `tithi-after` images were re-rendered on
2026-10-19 with the software renderer (W6-02) and again after the W6-03 year buttons. The 16
converter and settings images were rendered once more for W7-01: the base is inert while an
overlay is open, so the controls behind the scrim draw disabled.
`tithi-before-light-en-1x.png` is the historical femtovg image and was not re-rendered; it
records the pre-W3-00b layout.

The agent cannot judge the images, so every verdict is **not verified**: the reviewer looks.

## Converter (W3-02), Convert tab

| File | Theme | Language | Scale | Verdict |
|---|---|---|---|---|
| `converter-light-en-1x.png` | light | en | 1x | not verified |
| `converter-light-en-2x.png` | light | en | 2x | not verified |
| `converter-light-ne-1x.png` | light | ne | 1x | not verified |
| `converter-light-ne-2x.png` | light | ne | 2x | not verified |
| `converter-dark-en-1x.png` | dark | en | 1x | not verified |
| `converter-dark-en-2x.png` | dark | en | 2x | not verified |
| `converter-dark-ne-1x.png` | dark | ne | 1x | not verified |
| `converter-dark-ne-2x.png` | dark | ne | 2x | not verified |

The card is 320×300, centered over a scrim. The sample input is 2081/01/01 and the English
result is 2024-04-13 / April 13, 2024; the Nepali images show the same numbers in Devanagari
(`२०२४-०४-१३` / `April १३, २०२४`). The "Convert" tab holds the BS → AD and AD → BS buttons, one
input and the result; the "Days" tab has the from/to inputs and starts with today on both sides.
The cards were not typed into; the converter logic is covered by `app::converter` tests.

## Settings (W3-04)

| File | Theme | Language | Scale | Verdict |
|---|---|---|---|---|
| `settings-light-en-1x.png` | light | en | 1x | not verified |
| `settings-light-en-2x.png` | light | en | 2x | not verified |
| `settings-light-ne-1x.png` | light | ne | 1x | not verified |
| `settings-light-ne-2x.png` | light | ne | 2x | not verified |
| `settings-dark-en-1x.png` | dark | en | 1x | not verified |
| `settings-dark-en-2x.png` | dark | en | 2x | not verified |
| `settings-dark-ne-1x.png` | dark | ne | 1x | not verified |
| `settings-dark-ne-2x.png` | dark | ne | 2x | not verified |

The card is 330×430 with a scroll bar. Visible without scrolling: language, theme, the two clock
switches, the time zone filter and the first zone rows. Below the fold: the no-extra-clock button,
launch at login, taskbar text, version, data coverage, the NepDate and font credits, and
`AboutSlint`. The zone ids stay Latin (they are IANA ids), the rest is translated.

## W3-00b: English tithi width

Kartik 2083, light, English, 100 %.

| File | What it shows | Verdict |
|---|---|---|
| `tithi-before-light-en-1x.png` | 12 px tithi with a 4 px inset; "Sapt…", "Asht…", "Chat…" | not verified |
| `tithi-after-light-en-1x.png` | 11 px tithi across the full cell; "Saptami", "Ashtami", "Navami" | not verified |
| `tithi-after-light-ne-1x.png` | the 12 px Devanagari caption is unchanged | not verified |

At 11 px and the full 46 px cell, these names fit whole in a cell without the today ring:
Saptami, Ashtami, Navami, Dashami, Ekadashi, Dwadashi, Purnima, Tritiya, Shashthi. These still
elide: Trayodashi, Chaturdashi, Pratipadha, Chaturthi, Panchami. The today cell keeps a 4 px
inset so its 2 px ring keeps 2 px clear of the glyphs; that cell shows "Sapta…" instead of
"Saptami". No abbreviation was invented and no font below 11 px Latin or 12 px Devanagari was
used.

## W3-00c: tithi marker

See `tithi-marker.md`. The `*` comes from the hub data and is not changed by the widget.
