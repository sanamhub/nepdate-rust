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
  the text window visible at `[12, 1038, 171x35]`.
- `taskbar-real-light.png`: full-screen grab of the real taskbar (light, `SystemUsesLightTheme`
  toggled to 1, then reverted to 0 after the grab) with the text window shown.

The C# NepDateWidget reference app runs on this machine and draws its own text into the same free
area, so the real grabs may show both texts overlapping and are not reliable evidence of our
render. The `taskbar-*.png` snapshots (this directory) are unambiguous: they composite only our
render. The reviewer looks at all of them and gives the verdicts.

## W1B-06 manual checklist (what the agent ran)

- Left click on the text: the window procedure handled `WM_LBUTTONUP` and delivered
  `TaskbarEvent::Toggle`; a temporary log confirmed the popup went from hidden to
  `is_visible() = true` (the log was reverted). The popup's visual position is not verified.
- Right-click menu: `run_menu` is entered and runs `TrackPopupMenuEx` (temporary log, reverted).
  The menu's visual appearance is not verified.
- Open apps until the text hides: not triggered. 21 distinct apps moved the Start button from
  x=850 to x=476, but on this 1920 px centred taskbar the icon group overflows before it reaches
  the text (`place` hides at `start.left < 195`), so `NoRoom` never fired (log stayed on `shown`).
  The agent opened and closed: Calculator, Command Prompt, Windows PowerShell, PowerShell ISE,
  Snipping Tool, Remote Desktop, Character Map, Registry Editor, Control Panel, System
  Configuration, DirectX Diagnostic, Disk Cleanup, Settings, Clock, Photos, Chrome, VS Code,
  Microsoft Store. Notepad++ and Task Manager were already open (the human's) and were left open.
  The `NoRoom` path itself is covered by the W1B-01 unit tests.

## W1B-07 crops and z-order (after the bridge fix)

W1B-07 fixed the z-order: `controller.rs` now raises the text window with
`SetWindowPos(HWND_TOP, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)` after every show, because
Explorer's `Windows.UI.Composition.DesktopWindowContentBridge` sibling covers the whole taskbar
and is raised above our window whenever it redraws. The crops below are the evidence that the
text is actually on top now. All visual verdicts are the reviewer's; the agent could not view the
images.

| File | What it shows | Verdict |
|---|---|---|
| `w1b07-crop-dark-en.png` | dark taskbar, English, 420x60 crop at the taskbar's left end (BitBlt + CAPTUREBLT) | not verified |
| `w1b07-crop-dark-ne.png` | dark taskbar, Nepali (`--lang ne`), same crop | not verified |
| `w1b07-zorder.txt` | z-order dump of Shell_TrayWnd's children (class, visible, rect), top to bottom, our window first | see below |

The z-order dump shows `nepdate-taskbar-text` at `[0]` (topmost), above
`Windows.UI.Composition.DesktopWindowContentBridge` at `[1]`, which covers `[0,1032,1920x48]`.
A light-taskbar crop was not taken: the agent does not change `SystemUsesLightTheme`; the human
would need to switch the theme first.
