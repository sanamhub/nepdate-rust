# W1B: Windows taskbar text

**Goal:** two lines of date and time inside the Windows 11 taskbar, left of the centred icons,
as NepDateWidget does, with no flicker, no z-order fight and no idle cost.

Requirements: WR-12 (all AC), WR-06 key `taskbar_text`.
Read first: ADR-0016 (the design; follow it exactly), ADR-0015, ADR-0010, `widget-design.md` §5.
Starts after the W1 checkpoint is reviewed and merged (needs the popup, the timer, the tray
menu labels and logging).

## Tasks

### W1B-00 Snapshot behind a feature
- Move the W0 `--snapshot` flag and the `image` dependency behind a cargo feature `snapshot`
  (off by default, ADR-0016). Without the feature, `--snapshot` is an unknown argument.
- The CI `snapshot` job and the reviewer build with `--features snapshot`. `cargo xtask ci`
  builds the widget once with the feature so it can't rot.
- Report the release binary size with and without the feature.

### W1B-01 Pure model (`src/app/taskbar.rs`, no Win32, no Slint)
```rust
pub struct TaskbarLines { pub line1: String, pub line2: String }
pub fn taskbar_lines(now: &jiff::Zoned, s: &Settings) -> TaskbarLines;          // WR-12 AC1

pub struct Rect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 } // physical px
pub enum Edge { Bottom, Top, Left, Right }
pub struct TaskbarLayout { pub taskbar: Rect, pub edge: Edge, pub start: Option<Rect>,
                           pub widgets: Option<Rect>, pub scale: f32 }           // scale = dpi / 96
pub enum Hidden { NotBottom, NoStartButton, NoRoom, Disabled }
pub fn place(layout: &TaskbarLayout, text: (i32, i32)) -> Result<Rect, Hidden>;  // WR-12 AC4
```
- `place` returns the rectangle in **taskbar client coordinates** (the child window's parent).
  Left bound = `widgets.right` if a Widgets button is present and left of Start, else
  `taskbar.left`. `x = left + round(12 * scale)`, vertically centred. `NoRoom` if
  `x + width + round(12 * scale) > start.left`. A left-aligned taskbar puts Start at the far
  left, so it ends as `NoRoom` with no special case.
- Line text uses the `nepdate` formatter (BS long date and weekday in the chosen language) and
  `app::clock` for the time; the AD part is `jiff` `%b %-d, %Y`. No hand-built date strings.
- Tests (named after the AC):
  - `wr12_ac1_lines_en_12h`, `wr12_ac1_lines_en_24h`, `wr12_ac1_lines_ne`
    (`बिहिवार | १:५० PM` / `असोज २९, २०८३ | Oct 15, 2026` for 2026-10-15 13:50 local),
    `wr12_ac1_weekday_and_bs_from_same_date` (23:59:30 and 00:00:30 around midnight).
  - `wr12_ac4_place_free_area`, `wr12_ac4_place_after_widgets_button`,
    `wr12_ac4_left_aligned_is_no_room`, `wr12_ac4_not_bottom`, `wr12_ac4_no_start_button`,
    `wr12_ac4_exact_fit_and_one_px_short` (boundary), `wr12_ac7_scale_150_and_200`.

### W1B-02 Windows platform module (`src/platform/windows/`)
Module root has `#![allow(unsafe_code)]` and `#![deny(clippy::undocumented_unsafe_blocks)]`;
nothing outside this folder uses `unsafe`. The rest of the app sees only a trait in
`src/platform/taskbar.rs`:
```rust
pub trait TaskbarText {
    fn set_lines(&self, lines: &TaskbarLines);
    fn set_enabled(&self, on: bool);
    fn set_menu_labels(&self, labels: &MenuLabels);   // from @tr strings, via ui_bridge.rs
}
pub enum TaskbarEvent { Toggle { anchor: Rect }, Menu(MenuItem) }   // delivered by callback
```
with a no-op implementation on macOS and Linux. Files:
- `controller.rs`: hidden top-level controller window and the layered child text window
  (ADR-0016 points 1, 7, 8). Handles `TaskbarCreated`, `WM_SETTINGCHANGE`
  (`ImmersiveColorSet`, `TraySettings`), `WM_DISPLAYCHANGE`, `WM_DPICHANGED`, mouse input, and
  the worker's posted rectangles.
- `uia.rs`: worker thread, COM multithreaded, owns every UI Automation object (ADR-0016
  point 3). Finds the Start and Widgets buttons under `Shell_TrayWnd`, subscribes to
  `BoundingRectangle` changes of the Start button, posts rectangles with `PostMessageW`. The UI
  thread never calls UIA. Find the real AutomationIds with a UIA tree dump on this machine and
  put them, with the Windows build number, in the module docs.
- `render.rs`: Direct2D + DirectWrite into a premultiplied BGRA buffer (ADR-0016 point 4),
  fonts from the bundled TTFs through the in-memory loader, grayscale antialiasing, minimum
  alpha 1 on every pixel. Returns the buffer and its size, so it is testable without a window.
- Taskbar scheme: reuse `src/platform/taskbar_scheme.rs` from W1 (`windows-registry`, missing
  value = dark); do not add a second reader. `theme.rs` here holds only the two `ink` colours as
  constants. On `WM_SETTINGCHANGE` "ImmersiveColorSet" the controller re-reads the scheme and
  updates both the text window and the tray icon (`Tray::set_dark_taskbar`) at once.
- `build.rs`: `embed-manifest` with Windows 8 to 11 `supportedOS`, per-monitor-v2 DPI and the
  UTF-8 code page (ADR-0016). Check that winit/Slint still start and the popup still opens with
  the manifest.

Tests (Windows-only, `#[cfg(windows)]`):
- The DWORD-to-scheme mapping is already tested in W1 (`registry_value_to_scheme`).
- `wr12_ac2_ink_matches_design`: the two constants equal the `ink` row of `widget-design.md`
  (same style as the W0 theme drift test). Negative control: change one, see it fail, revert.
- `render_is_premultiplied_and_hit_testable`: render both lines, then every pixel has
  `a >= max(r, g, b)` and `a >= 1`, and some pixels are opaque text.
- `render_shapes_devanagari`: rendering `क्ष` gives fewer glyphs than code points (DirectWrite
  glyph run count), so shaping happened; a Latin-only font would not.

### W1B-03 Wiring (`main.rs`, `ui_bridge.rs`)
- The W1 timer calls `set_lines` with `taskbar_lines(now, settings)`; the text window redraws
  only on change (ADR-0016 point 6).
- `TaskbarEvent::Toggle` opens or hides the popup above the anchor rectangle, left-aligned,
  clamped to the monitor's work area. `Menu(..)` runs the same actions as the tray menu.
- `taskbar_text` setting on and off without restart (W3 builds the settings UI; for now a
  `--no-taskbar-text` flag and the `Settings` default).

### W1B-04 Visual check tool
`--taskbar-snapshot <path.png> [--taskbar-theme light|dark] [--lang en|ne] [--scale 1|1.5|2]`
(feature `snapshot`): renders the text with `render.rs` exactly as it would go to
`UpdateLayeredWindow`, composited over a taskbar-coloured strip (`#F3F3F3` light, `#202020`
dark, 48 px × scale high), and writes a PNG. The CI `snapshot` job runs it on windows-latest
for both themes and both languages and uploads the PNGs.

## Definition of Done
Global DoD, plus `docs/runbooks/widget-manual-test.md` WR-12 lines, ticked on Windows 11 only
for what was actually done:
- [ ] light and dark taskbar, switch live
- [ ] open 10 apps until the icons reach the text: text hides; close them: text returns
- [ ] Widgets button on and off; taskbar alignment left and centre
- [ ] auto-hide taskbar; full-screen video
- [ ] `taskkill /f /im explorer.exe` then `start explorer`: text back within 5 s
- [ ] scale 100 %, 150 %, 200 %
- [ ] left click opens the popup above the text; right click menu works; Quit leaves no process

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes, with and without `--features snapshot`.
- [ ] `unsafe` appears only under `src/platform/windows/`, every block has `// SAFETY:`, and
      clippy `undocumented_unsafe_blocks` is denied there.
- [ ] No UIA call on the UI thread; no process spawned for OS state; no polling timer for
      placement (events plus the W1 tick only).
- [ ] Taskbar PNGs (light, dark, en, ne, scales) looked at; Devanagari shaped.
- [ ] Full-screen grabs of the real taskbar, light and dark, looked at.
- [ ] Idle CPU over 2 minutes with the text on, method given; binary size and startup versus W1.
