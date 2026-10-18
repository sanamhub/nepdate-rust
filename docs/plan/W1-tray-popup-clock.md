# W1: Tray icon, popup window, header and clock

**Goal:** the app lives in the tray/menu bar. Clicking opens a popup with today's date and
clocks. Idle cost is near zero.

Requirements: WR-01, WR-02 (AC1–AC4), WR-05, WR-09 AC3–AC5.
Read first: ADR-0010 (tray fallback), ADR-0011, ADR-0015 (state, time injection).
Human reads: `docs/learn/09-event-loop-and-state.md`.

## Tasks

### W1-01 App state + translations
- `src/app/settings.rs`: `Settings` struct with the keys and defaults from WR-06 (serde derive
  comes in W3; plain struct now). `Default` impl = the WR-06 defaults.
- Translations: UI text in `.slint` as `@tr("...")`, Nepali in
  `i18n/ne/LC_MESSAGES/nepdate-widget.po` (ADR-0008 A.3). `build.rs` uses
  `slint_build::CompilerConfiguration::new().with_bundled_translations("i18n")`. On language
  change, call `slint::select_bundled_translation(code)` (after the first component exists).
- Test `po_file_has_no_empty_translations` (reads the `.po` file, fails on `msgstr ""`
  except the header).

### W1-02 Clock model (`src/app/clock.rs`)
Returns data, not sentences; labels like "Local" are `@tr` in `.slint`.
```rust
pub struct ClockLine { pub zone_city: Option<String>, pub time: String, pub offset: String } // None = local
pub fn clock_lines(now: &jiff::Zoned, s: &Settings) -> Vec<ClockLine>;   // 1 or 2 lines (WR-05 AC1)
pub fn next_wakeup(now: &jiff::Zoned, show_seconds: bool) -> std::time::Duration; // WR-05 AC5
pub fn header(now: &jiff::Zoned, lang: Lang) -> Header;  // BS long + weekday + AD (WR-02 AC3, WR-05 AC4)
```
All pure; tests use fixed `Zoned` values (e.g. `"2026-09-24T23:59:30+05:45[Asia/Kathmandu]".parse()`).

AC (tests named after the requirement):
- `wr05_ac1_hides_extra_zone_with_same_offset`
- `wr05_ac2_12h_and_24h_and_seconds` (incl. Nepali digits)
- `wr05_ac3_label_is_city_with_offset` (`Asia/Kathmandu` → `Kathmandu`, `+05:45`;
  `America/Argentina/Buenos_Aires` → `Buenos Aires`)
- `wr05_ac4_weekday_matches_bs_date` (the BS date and weekday both come from `now.date()`)
- `wr05_ac5_wakeup_is_next_minute_plus_50ms`

### W1-03 Tray (`src/platform/tray.rs`)
- A trait `Tray { fn set_title(&self, text: &str); fn set_icon_day(&self, day: &str); fn set_tooltip(&self, t: &str); }`
  plus events delivered through a callback: `Open`, `CopyBs`, `CopyAd`, `Settings`, `Quit`.
- Implementation A: Slint `SystemTrayIcon` (in `.slint`). It has `icon`, `tooltip`, `title`
  (macOS menu-bar text) and one child `Menu` of `MenuItem`s (`activated`). `clicked` fires on
  Windows/Linux, and on macOS only when there is no menu, so macOS uses the menu's "Open" item (WR-01 AC4).
- Menu labels live once, in a Slint global `MenuText` with `@tr` out properties (open, copy BS,
  copy AD, settings, quit). The tray menu binds to it, and `ui_bridge.rs` reads it so W1B's
  native taskbar menu shows the same translated labels.
- If A can't meet WR-01 on some OS, implementation B uses `tray-icon` + `muda` (ADR-0010).
  Document which OS uses which implementation in `src/platform/tray.rs` module docs.
- Day-number icon (Windows/Linux): `cargo xtask icons` pre-renders `ui/tray/{en,ne}/{1..32}.png`
  (32×32, from one SVG template + the bundled Noto fonts, via `resvg`), committed to the repo.
  The `.slint` file holds `[image]` arrays of `@image-url(...)`, and Rust sets the index. No runtime rendering.

### W1-04 Popup window (`ui/popup.slint`, `src/ui_bridge.rs`)
- `no-frame: true`, `always-on-top: true`, fixed 360×520 logical px (440 until the W2 review
  needed taller grid rows; WR-02 AC1 carries the current value).
- Position (WR-02 AC1): Slint exposes no tray-icon position, so use a fixed corner of the
  primary monitor. Windows: bottom-right, 12 px from the right and 56 px (logical) from the bottom,
  above a standard taskbar. macOS: top-right, 32 px from the top. Linux X11: top-right. Wayland: the
  compositor decides.
- Keep the popup out of the Windows taskbar: Slint feature `unstable-winit-030`,
  `slint::BackendSelector::new().with_winit_window_attributes_hook(|a| a.with_skip_taskbar(true))`
  under `#[cfg(windows)]` (`winit::platform::windows::WindowAttributesExtWindows`). macOS: the
  packaged app sets `LSUIElement` (no Dock icon). Linux: a taskbar entry is acceptable.
- Placement is one function, `show_popup_at(anchor: PopupAnchor)` with `PopupAnchor::TrayCorner`
  (the rule above) and `PopupAnchor::Above(rect)` (used by W1B: above a rectangle, left-aligned,
  clamped to the monitor's work area). Only `TrayCorner` is wired in W1; `Above` gets unit tests
  on the clamping maths now.
- Hide on focus loss and on Escape (WR-02 AC2).
- Header + clock lines bound to properties set from `app::clock`.

### W1-05 Timer + rollover (`src/main.rs`)
One `slint::Timer` in `SingleShot` mode, re-armed with `next_wakeup(now)` each time it fires.
It updates the header, clocks and tray title/icon. Resume/clock-change correctness comes
from the ≤ 61 s re-arm (WR-05 AC6); no OS hooks needed.

### W1-06 Logging + panic hook (`src/platform/logging.rs`)
`log` + `simplelog` `WriteLogger` into `dirs::data_local_dir()/nepdate-widget/`,
size-rotated at 1 MB with 2 files kept (a hand-written check at startup is enough: if the
file is > 1 MB, rename it to `.1`). `std::panic::set_hook` logs message + location (WR-09 AC4–AC5).

## Definition of Done
Global DoD. Manual checklist items for WR-01/WR-02 are added to
`docs/runbooks/widget-manual-test.md` and ticked on at least one OS in the PR description.

## Human review checklist
- [ ] Run the app, then check CPU in Task Manager / Activity Monitor for 2 minutes. Is it ~0 %?
- [ ] Switch the language to ne and back. Do all labels change without a restart? (`@tr`, ADR-0008)
- [ ] Find where `Zoned::now()` is called. It should appear only in `main.rs`/`ui_bridge.rs`, never in `app/`. Why? (ADR-0015)
- [ ] Look at `Rc<RefCell<AppState>>`. What happens if two callbacks borrow it mutably at once? (learn/09 §3)

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes; `app/` unit tests cover the clock model and the day rollover.
- [ ] `Zoned::now()` is called only in `main.rs` or `ui_bridge.rs`, never in `app/` (ADR-0015).
- [ ] The timer wakes at most once per second and idle CPU is about 0 % (number in the PR).
- [ ] Formatting in the UI uses the `nepdate` formatter values (`write_to` into a reused buffer
      or `to_string` once per tick), not hand-built strings.
- [ ] WR-01 and WR-02 manual checklist lines are added and ticked on at least one OS.
- [ ] Logging rotates at 1 MB and the panic hook logs message and location.
