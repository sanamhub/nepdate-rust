# ADR-0016: Windows taskbar text

- **Status:** accepted
- **Date:** 2026-10-15
- **Approved by:** Sanam
- **Amends:** ADR-0008 §B (removes "taskbar-embedding hacks" from the out-of-scope list, for
  Windows 11 only), ADR-0015 (dependency list, unsafe code in one module)

## Context

NepDateWidget's best-liked feature is two lines of date and time drawn inside the Windows 11
taskbar, in the empty area left of the centred icons:

```
Thursday | 1:50 PM
Ashoj 29, 2083 | Oct 15, 2026
```

The tray icon (WR-01) shows only a 16 px day number on Windows, because a notification-area
icon can't show text. The maintainer wants the taskbar text in v0.1, while keeping the rest of the
widget minimal. Requirement: WR-12.

Constraints:

- Windows gives no API for third-party content in the taskbar. Every tool that does it either
  parents a window into the taskbar (`Shell_TrayWnd`) or floats a top-most window over it.
- Devanagari must shape correctly (conjuncts, reph, pre-base matra), with the bundled Noto
  fonts, as in the font test.
- The workspace denies `unsafe_code`; Win32 calls are `unsafe`.
- Idle cost must stay at about 0 % CPU (WR-05 AC5, WR-10).

## Options

**A. Slint window over the taskbar.** A second frameless, transparent, skip-taskbar Slint window,
positioned over the taskbar and kept top-most. Same renderer as the popup, no Win32 drawing
code. But the taskbar is itself top-most and raises itself when clicked or when the Start menu
opens, so an overlay keeps losing the z-order fight and must re-raise itself (flicker, timers).
It does not move with an auto-hiding taskbar, and full-screen apps need separate detection.
Per-pixel transparency of a winit/femtovg OpenGL window over the taskbar's Mica material is not
reliable across GPU drivers.

**B. Native layered child window of `Shell_TrayWnd`.** A small Win32 window, created with
`WS_CHILD` and `WS_EX_LAYERED`, parented to the taskbar. It is drawn into a premultiplied
32-bit bitmap with Direct2D and DirectWrite and pushed with `UpdateLayeredWindow`. Because it is
part of the taskbar's window tree, it moves with an auto-hiding taskbar, hides with the taskbar
under a full-screen app, and never fights for z-order. This is the approach long used by
TrafficMonitor and similar tools. DirectWrite shapes Devanagari, and loads the bundled fonts
from memory, so there is no system-font dependency. Costs: about 600 lines of `unsafe` Win32
and COM code, an application manifest (layered child windows need the Windows 8+
`supportedOS` entry), and a cross-process parent that disappears when Explorer restarts.

**C. Tray icon only.** No new code; does not meet the owner's request.

## Decision

Option **B**, on Windows 11 with a bottom, centre-aligned taskbar. Everywhere else (Windows 10,
a left-aligned Windows 11 taskbar, no free room) the taskbar text stays hidden and the tray icon
alone shows the date; the reason is logged once. macOS keeps the menu-bar title of WR-01 AC2,
which is the native equivalent. Linux has no equivalent.

Design:

1. **Two windows, one thread.** A hidden top-level *controller* window (never shown, class
   `mitibar-taskbar-controller`) receives the broadcasts a child cannot: `TaskbarCreated`
   (Explorer restarted; registered with `RegisterWindowMessageW`), `WM_SETTINGCHANGE`
   (`ImmersiveColorSet` for theme, `TraySettings` for alignment), `WM_DISPLAYCHANGE` and
   `WM_DPICHANGED`. It owns the *text* window: `WS_CHILD | WS_CLIPSIBLINGS` with
   `WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`, parent `Shell_TrayWnd`. Both
   windows live on the main thread; Slint's winit event loop already dispatches every message
   of that thread, so there is no second message loop.
2. **Placement.** The free area runs from the taskbar's left edge (or the right edge of the
   Widgets button, if it is shown) to the left edge of the Start button, both read with UI
   Automation from the taskbar's element tree. The text sits 12 px (logical, scaled by the
   taskbar's DPI) from the left of that area, vertically centred. If the text plus 12 px on the
   right does not fit before the Start button, or the taskbar is not at the bottom
   (`SHAppBarMessage(ABM_GETTASKBARPOS)`), the text window is hidden. The maths is a pure
   function in `app/taskbar.rs`.
3. **UI Automation off the UI thread.** Explorer can hang; a UIA call from the UI thread would
   hang the widget with it. A worker thread (COM multithreaded) owns all UIA objects. It finds
   the Start and Widgets buttons, subscribes to `BoundingRectangle` changes on the Start button
   (it moves when apps open or close in a centred taskbar), and posts the rectangles to the
   controller window with `PostMessageW`. The UI thread never calls UIA.
4. **Drawing.** Direct2D DC render target onto a 32-bit top-down DIB, premultiplied BGRA,
   grayscale text antialiasing (ClearType is wrong on a transparent surface). Fonts: the
   bundled Noto Sans and Noto Sans Devanagari, loaded with DirectWrite's in-memory font loader
   into a private font collection. Line 1 regular, line 2 semibold (a named instance of the
   variable fonts), both 12 px at 100 % scale. Every pixel of the window rectangle gets at
   least alpha 1, so the whole rectangle takes clicks; fully transparent pixels would let
   clicks through to the taskbar.
5. **Colour.** The taskbar follows the *system* theme (`SystemUsesLightTheme`), which can
   differ from the app theme (`AppsUseLightTheme`). The value is read with `RegGetValueW`,
   never by spawning a process, and re-read on `WM_SETTINGCHANGE` "ImmersiveColorSet". Text
   colour is the `ink` token of `docs/design/widget-design.md` for that scheme; a test keeps the Rust
   constants equal to the table.
6. **Repaint only on change.** The once-a-minute timer (or once a second with seconds shown)
   passes new lines in; the text window redraws only when the lines, the colour, the DPI or the
   placement changed.
7. **Input.** Left click toggles the popup, opened above the text and left-aligned with it.
   Right click opens a native menu (`TrackPopupMenuEx`) with the WR-01 AC5 items; the labels
   come from the same `@tr` strings as the tray menu, passed in by `ui_bridge.rs`.
8. **Recovery.** On `TaskbarCreated` the controller recreates the text window and the worker
   re-subscribes. Any Win32 or COM failure hides the text, logs it once, and leaves the tray icon
   working; it never ends the app.

**Unsafe code.** Only `src/platform/windows/` (module root `#![allow(unsafe_code)]`,
`#![deny(clippy::undocumented_unsafe_blocks)]`) may use `unsafe`, and every block has a
`// SAFETY:` comment. The library crates keep `#![forbid(unsafe_code)]`; everything else in
the widget keeps the workspace `deny`.

**Dependencies** added to ADR-0015's list:

- `windows` 0.62 (already in `Cargo.lock` through winit), Windows-only target dependency, only
  the `Win32_*` features the module uses.
- `windows-registry` 0.6 (same windows-rs family, safe API), Windows-only, for reading
  `SystemUsesLightTheme` without `unsafe`; the tray icon uses it for its ink.
- `embed-manifest` (build dependency) for the application manifest: Windows 8 to 11
  `supportedOS`, per-monitor-v2 DPI awareness, UTF-8 active code page.
- `image` (PNG only) moves behind a cargo feature `snapshot`, off by default. ADR-0015 says
  the shipped binary has no image crate; the `--snapshot` flag is a test tool, so CI and
  developers enable the feature and release builds don't carry it.

## Consequences

- The best-liked NepDateWidget feature is in v0.1 on Windows 11, without z-order fights or
  flicker.
- About 600 lines of `unsafe` Win32 and COM code, kept in one module with a safety comment on
  every block.
  Everything decidable without a desktop (line text, placement, theme value, colours) is pure
  and unit-tested in `app/` and `platform/windows/` tests.
- Depends on undocumented parts of the Windows 11 taskbar: the class name `Shell_TrayWnd` and
  the UI Automation IDs of the Start and Widgets buttons. A Windows update can break it; the
  failure mode is "text hidden, tray icon still works, one log line", and the manual checklist
  re-checks it on each release.
- Windows 10 users get the tray icon only. Adding Windows 10 later (room left of the
  notification area) needs a new decision; it would mean resizing Explorer's own task band.
