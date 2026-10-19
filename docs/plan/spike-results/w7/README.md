# W7 focus snapshots

Eight images of the popup with the **Next year** button focused. Generated on Windows 11 with
the MSVC toolchain, the software renderer and the debug `snapshot` build:

```text
mitibar.exe --popup --snapshot <png> --theme light|dark --lang en|ne --today 2081-01-01 --focus-year-next
```

`SLINT_SCALE_FACTOR=1` or `2` selects the scale. `--today 2081-01-01` pins the date and the clock
to 12:00, so a re-render is stable. `--focus-year-next` exists only in the `snapshot` feature and
the parser rejects it without `--popup --snapshot`.

The ring is `Theme.ink`, the same token as primary text: `#211C17` in light and `#F1ECE4` in dark.
The custom year button draws a 2 logical px border; at 2x it is 4 px. Standard Slint widgets use a
3 px `FocusFrame` overlay over their own 2 px outer plus 1 px inner border.

Every verdict is **not verified** until a human looks.

| File | Theme | Language | Scale | Viewport | Bytes | Verdict |
|---|---|---|---|---|---|---|
| `year-button-focus-light-en-1x.png` | light | en | 1x | 360x520 | 81,584 | not verified |
| `year-button-focus-light-en-2x.png` | light | en | 2x | 720x1040 | 196,198 | not verified |
| `year-button-focus-light-ne-1x.png` | light | ne | 1x | 360x520 | 77,768 | not verified |
| `year-button-focus-light-ne-2x.png` | light | ne | 2x | 720x1040 | 186,550 | not verified |
| `year-button-focus-dark-en-1x.png` | dark | en | 1x | 360x520 | 80,032 | not verified |
| `year-button-focus-dark-en-2x.png` | dark | en | 2x | 720x1040 | 193,897 | not verified |
| `year-button-focus-dark-ne-1x.png` | dark | ne | 1x | 360x520 | 76,423 | not verified |
| `year-button-focus-dark-ne-2x.png` | dark | ne | 2x | 720x1040 | 185,246 | not verified |

## Determinism

Each image was rendered twice in a row with the same command. The two files had the same SHA-256,
so the table records the hash once and `same` for the second render.

| File | SHA-256 | Second render |
|---|---|---|
| `year-button-focus-light-en-1x.png` | `197B0E0DA997FBEAED237EB31CB2A239AD07968C6BD583A0F6E60CCFE609126F` | same |
| `year-button-focus-light-en-2x.png` | `0857D6FC36C5DC403AC51AF9DD4DF894CF9785A6FEAA7C5BBE451255C493A774` | same |
| `year-button-focus-light-ne-1x.png` | `867BCCAA793FF47ADF379DD3CAA2A53FE8721F4EC7EAAB26E2BD1D4AEF781A35` | same |
| `year-button-focus-light-ne-2x.png` | `ED9D7F1BFC6B32AEBE9DDA95ECA64C8AC85F741A278FF1080FD2A6779B29627E` | same |
| `year-button-focus-dark-en-1x.png` | `AD861EF6FE7B691722951AA8AC4C6CDC8FF4D83656A5CCBC3BDFDC585B777DB4` | same |
| `year-button-focus-dark-en-2x.png` | `D0E83A69C242EF2900785BB7AE728B88CF9630CC25B4AB56F15B1CF768F590C6` | same |
| `year-button-focus-dark-ne-1x.png` | `EF17D0C210B736D43DDE3D4B2998F40625D1B9B713AB1C81E73A8DA8999C24B2` | same |
| `year-button-focus-dark-ne-2x.png` | `14BAE54A047C5A2F6F2E244DC2BD8F57AEEF41B8834C6991666BE2BA7480A4CD` | same |

## Measured contrast

The colours were sampled from the 1x PNGs. The ring sits inside the year box, so the adjacent
background is the Fluent control background, `#FEFDFC` in light and `#231F1C` in dark.

| Theme | Ring | Adjacent control | Ratio | Ring | Window surface | Ratio |
|---|---|---|---|---|---|---|
| light | `#211C17` | `#FEFDFC` | 16.63:1 | `#211C17` | `#FDFBF8` | 16.35:1 |
| dark | `#F1ECE4` | `#231F1C` | 13.91:1 | `#F1ECE4` | `#16120E` | 15.85:1 |

`cargo test -p mitibar focus_ring_ink_has_at_least_3_to_1_contrast` also checks the design-token
pairs and the Fluent composite: 16.89:1 and 16.74:1 in light, 14.52:1 and 13.75:1 in dark. Every
ratio is above the 3:1 minimum.

## Limits

- Windows x64 only. macOS and Linux are unverified.
- The images show the year button ring. The other controls use the same `FocusFrame` component or
  the same `Theme.ink` border, but no committed image shows each one; their focus order is
  covered by `tab_and_shift_tab_follow_the_focus_order`.
- The images are not verified by a human.
