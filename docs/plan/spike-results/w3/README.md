# W3 spike results

Generated on Windows 11 with the MSVC toolchain on 2026-10-18. Each PNG is the popup rendered by
`--popup --snapshot <png> --theme light|dark --lang en|ne --today <BS date>`, with
`SLINT_SCALE_FACTOR=1` or `2`. `--today` pins the calendar date and the clock to 12:00.

The agent cannot judge the images, so every verdict is **not verified**: the reviewer looks.

## W3-00b: English tithi width

Kartik 2083, light, English, 100 %.

| File | What it shows | Verdict |
|---|---|---|
| `tithi-before-light-en-1x.png` | 12 px tithi with a 4 px inset; "Sapt…", "Asht…", "Chat…" | not verified |
| `tithi-after-light-en-1x.png` | 11 px tithi across the full cell; "Saptami", "Ashtami", "Navami" | not verified |
| `tithi-after-light-ne-1x.png` | the 12 px Devanagari caption is unchanged | not verified |

At 11 px and the full 46 px cell, these names fit whole in the image: Saptami, Ashtami, Navami,
Dashami, Ekadashi, Dwadashi, Purnima, Tritiya, Shashthi. These still elide: Trayodashi,
Chaturdashi, Pratipadha, Chaturthi, Panchami. No abbreviation was invented and no font below
11 px Latin or 12 px Devanagari was used.

## W3-00c: tithi marker

See `tithi-marker.md`. The `*` comes from the hub data and is not changed by the widget.
