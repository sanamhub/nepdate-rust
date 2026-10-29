> Moved to [sanamhub/mitibar](https://github.com/sanamhub/mitibar) as ADR-0008. The text is kept here unchanged.

# ADR-0019: A floating mini bar replaces the taskbar text, plus one global shortcut

- **Status:** accepted
- **Date:** 2026-10-22
- **Approved by:** Sanam
- **Supersedes:** ADR-0016 (Windows taskbar text)
- **Amends:** ADR-0008 §B (the floating mini bar and a global shortcut move into v0.1),
  ADR-0015 (dependency list)

## Context

The owner tested the first beta build on Windows 11 on 2026-10-22 and asked for:

- a bar that can be dragged anywhere, like NepDateWidget's pill, instead of text fixed left of
  the Start button;
- the bar on Windows 10 too, which ADR-0016 ruled out;
- the extra time zone on the bar, and control over which fields each line shows;
- a keyboard shortcut that opens the popup from anywhere.

ADR-0016 drew the text inside `Shell_TrayWnd` as a child window. A child of the taskbar can't
leave it, so it can't be dragged to another place, and its placement depended on UI
Automation reading the Windows 11 Start and Widgets buttons. Windows 10 has neither.

ADR-0008 §B deferred the floating mini bar because Wayland can't position windows or keep them
on top. That is still true on Wayland, and nothing here changes it for Linux.

NepDateWidget's pill (read for reference, ADR-0008 §A) is a top-level, top-most tool window
whose owner is `Shell_TrayWnd`. Being owned by the taskbar keeps it visible through Win+D. It
re-asserts top-most when the foreground window changes, hides while a full-screen app is in
front, and saves its position after a drag.

## Options

**A. Keep the taskbar child, add horizontal dragging inside the taskbar.** Still Windows 11
only, still tied to the Start button's UI Automation ID, and the user can't put it anywhere else.

**B. A top-level layered window owned by the taskbar, drawn with the existing Direct2D and
DirectWrite code.** Works on Windows 10 and 11, with any taskbar position. Reuses ADR-0016's
renderer, in-memory fonts and theme code. Removes the UI Automation worker. Costs: top-most
upkeep on foreground changes, full-screen detection, and position storage.

**C. A second Slint window.** Same renderer as the popup, but per-pixel transparency over the
taskbar is not reliable with the software renderer (ADR-0018), and a second Slint window holds
its own frame buffer all day, which works against the idle memory target.

## Decision

Option **B**, named the **mini bar** (WR-08). The ADR-0016 child window and its UI Automation
worker are removed.

1. **Window.** `WS_POPUP` with `WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE |
   WS_EX_TOPMOST`, owner `Shell_TrayWnd`. No taskbar button, no Alt+Tab entry, and it never
   takes focus, so the app under it keeps the keyboard. When Explorer restarts
   (`TaskbarCreated`), the window is recreated with the new owner.
2. **Drawing.** ADR-0016 §4 unchanged: Direct2D onto a premultiplied 32-bit DIB, DirectWrite
   with the bundled fonts, grayscale antialiasing, pushed with `UpdateLayeredWindow`. The pill
   shape, colours and type come from `docs/design/widget-design.md` §5.
3. **Top-most upkeep without polling.** A `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` callback
   (out of context, on the UI thread) re-asserts `HWND_TOPMOST` with `SWP_NOACTIVATE` and runs
   the full-screen check. There is no timer for either.
4. **Full screen.** The bar hides while the foreground window covers its whole monitor and is
   not the desktop or the taskbar, and while `SHQueryUserNotificationState` reports a
   full-screen Direct3D app or presentation mode. It shows again when that ends.
5. **Dragging.** Press, move past the system drag distance (`SM_CXDRAG`, `SM_CYDRAG`), then move
   with mouse capture. Release saves the position. A press and release without that movement is
   a click. The bar stays fully on one monitor and snaps to that monitor's taskbar centre line
   when it's dropped within 8 logical px of it. "Lock position" turns dragging off.
6. **Position.** Saved as the monitor's device name and the bar's top-left offset from that
   monitor's top-left, in logical px. At start, after a display change and after a DPI
   change, the saved monitor is looked up. If it's gone, the bar goes to the default position
   on the primary monitor. If it would be partly off-screen, it's moved fully on-screen.
7. **Default position**, used on first run and by "Reset position": vertically centred on the
   primary monitor's taskbar. With a centred Windows 11 taskbar it sits 12 logical px from the
   taskbar's left edge. Otherwise its right edge sits 12 logical px left of the notification
   area (`TrayNotifyWnd`, a child of `Shell_TrayWnd`). With a vertical or hidden taskbar, it
   sits 12 logical px above the bottom-right corner of the work area. These are pure functions
   over rectangles, unit-tested without a desktop.
8. **Input.** A click toggles the popup. A double-click opens it and never closes it: the second
   click of a double-click is ignored. The popup opens above the bar when there is room, below
   it otherwise, left-aligned with it, fully on-screen. A right-click opens the native menu:
   the WR-01 AC5 items, then Customize bar, Lock position and Hide bar.
9. **Fields.** The two lines are lists of field ids chosen by the user (WR-08 AC2). The line
   text is built in `app/` from the settings and the clock, never in the Win32 module.
10. **Recovery.** Any Win32 or COM failure hides the bar and logs one line. The tray icon keeps
    working and the app never ends because of the bar.

**Other platforms.** macOS keeps the menu-bar title (WR-01 AC2). It is the native place for
always-visible text, and macOS lets the user reorder menu-bar items with Cmd-drag. Its text uses
the bar's top line. Linux keeps the tray icon only. A floating bar on X11 is possible later but
needs its own decision, and Wayland can't support it.

**Global shortcut.** One system-wide shortcut opens and closes the popup (WR-13). Default
Win+Shift+N on Windows, Cmd+Shift+N on macOS, Super+Shift+N on Linux X11. Ctrl+Alt combinations
are refused, because Ctrl+Alt is AltGr on the Nepali and many European keyboard layouts. If the
OS refuses the registration, the app logs it and Settings says the shortcut is in use. Wayland
has no portable way to grab a global key, so the field is disabled there.

**Dependencies** added to ADR-0015's list:

- `global-hotkey` 0.8 (MIT OR Apache-2.0, the Tauri project's crate, no web view): one API over
  `RegisterHotKey`, the macOS hot-key API and X11 grabs. It is the only new direct crate. 0.9.0
  was six days old when this was written; Dependabot proposes it once it has settled.
- `windows` gains the `Win32_UI_Accessibility` feature for `SetWinEventHook` (already enabled
  for ADR-0016's UI Automation) and keeps the rest. The UI Automation code is deleted, so
  `windows-core`'s `#[implement]` use goes with it if nothing else needs it.

**Unsafe code.** ADR-0016's rule stands: only `src/platform/windows/` may use `unsafe`, and
every block has a `// SAFETY:` comment.

## Consequences

- The bar works on Windows 10 and 11, at any taskbar position, and goes where the user puts it.
- The UI Automation worker and the Start-button placement code are gone, which removes the
  dependency on undocumented Windows 11 automation IDs. The bar still depends on the
  `Shell_TrayWnd` and `TrayNotifyWnd` class names, which have been stable since Windows XP.
- A top-most window can cover part of another app. The user can drag it, lock it or hide it,
  and it hides for full-screen apps.
- Idle cost stays at a redraw once a minute (once a second with seconds shown) and one
  callback per foreground change. The release checklist measures it (ADR-0008 §C).
- WR-12 is withdrawn. Its ID is kept so later IDs don't change.
