# W1B spike results (taskbar text)

Generated on Windows 11 (build 26300) with the MSVC toolchain. The visual verdicts are the human
reviewer's (quality bar §2a); the agent could not view the images, so nothing here is marked PASS.

## Taskbar snapshots (`--taskbar-snapshot`)

Rendered with `render.rs` exactly as the controller sends the text to `UpdateLayeredWindow`, then
composited over a taskbar-coloured strip (`#F3F3F3` light, `#202020` dark, 48 px scaled high). One
PNG per theme, language and scale:

| File | What it shows |
|---|---|
| `taskbar-light-en-{1,1.5,2}.png` | light taskbar, English, scales 100 % / 150 % / 200 % |
| `taskbar-light-ne-{1,1.5,2}.png` | light taskbar, Nepali |
| `taskbar-dark-en-{1,1.5,2}.png` | dark taskbar, English |
| `taskbar-dark-ne-{1,1.5,2}.png` | dark taskbar, Nepali |

Line 1 is `weekday | time` (WR-05 AC2 format), line 2 `BS long | AD date` (WR-12 AC1). Line 1 is
regular, line 2 semibold, 12 px at 100 % scale, 2 px line gap (widget-design.md §5).

## Real taskbar grabs

- `taskbar-real.png`: full-screen grab of the real taskbar (dark, `SystemUsesLightTheme` = 0) with
  the text window visible at `[12, 1038, 171x35]`. The C# NepDateWidget reference app was also
  running on this machine during part of the work and shows its own text in the same area; check
  which text the grab shows before judging.

The reviewer looks at these PNGs and gives the verdicts.
