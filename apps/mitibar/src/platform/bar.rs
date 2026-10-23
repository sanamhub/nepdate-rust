//! The floating mini bar surface (WR-08, ADR-0019). Windows only; a no-op elsewhere. The rest of
//! the app sees only this trait and these types, never the `unsafe` Win32 in `windows/`.

use crate::app::bar::BarLines;
use crate::app::bar_place::{Rect, Saved};

/// The right-click menu labels (WR-08 AC6), built from the `@tr` strings by the tray component.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuLabels {
    pub open: String,
    pub copy_bs: String,
    pub copy_ad: String,
    pub customize: String,
    pub lock: String,
    pub hide: String,
    pub settings: String,
    pub quit: String,
}

/// An item of the bar's right-click menu (WR-08 AC6). Only the Windows bar sends one.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Open,
    CopyBs,
    CopyAd,
    Customize,
    Lock,
    Hide,
    Settings,
    Quit,
}

/// Where the popup opens for something done on the bar (ADR-0019 point 8): the bar's rectangle,
/// the work area of its monitor and that monitor's DPI, all in physical pixels.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    pub bar: Rect,
    pub work: Rect,
    pub dpi: u32,
}

/// What the bar tells the app (ADR-0019 points 5 and 8).
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BarEvent {
    /// A click: open the popup, or close it if it is open.
    Toggle(Anchor),
    /// A double-click: open the popup, and leave it open if it already is.
    Open(Anchor),
    /// A drag ended: the position to save.
    Moved(Saved),
    /// A right-click menu pick.
    Menu { item: MenuItem, anchor: Anchor },
}

/// The bar the app updates (WR-08). `set_lines` is called once per tick; the window redraws only
/// when something it shows changed (WR-08 AC9).
pub trait Bar {
    fn set_lines(&self, lines: &BarLines);
    /// The `bar` setting: whether the bar is shown at all.
    fn set_shown(&self, shown: bool);
    /// The `bar_locked` setting: a locked bar does not drag.
    fn set_locked(&self, locked: bool);
    /// Puts the bar at a saved position; `Saved::default()` is the default position (WR-08 AC5).
    fn set_position(&self, saved: &Saved);
    fn set_menu_labels(&self, labels: &MenuLabels);
}

/// Creates the platform bar (Windows) or a no-op (elsewhere). `on_event` receives `BarEvent`s on
/// the main thread; `on_taskbar_scheme` receives `true` for a dark taskbar so the tray icon can
/// follow the taskbar scheme at once.
pub fn create(
    on_event: impl FnMut(BarEvent) + 'static,
    on_taskbar_scheme: impl FnMut(bool) + 'static,
) -> Box<dyn Bar> {
    #[cfg(windows)]
    {
        Box::new(super::windows::WindowsBar::new(on_event, on_taskbar_scheme))
    }
    #[cfg(not(windows))]
    {
        let _ = (on_event, on_taskbar_scheme);
        Box::new(Noop)
    }
}

/// The no-op on macOS and Linux: macOS has the menu-bar title and Linux the tray icon (ADR-0019).
#[cfg(not(windows))]
struct Noop;

#[cfg(not(windows))]
impl Bar for Noop {
    fn set_lines(&self, _lines: &BarLines) {}
    fn set_shown(&self, _shown: bool) {}
    fn set_locked(&self, _locked: bool) {}
    fn set_position(&self, _saved: &Saved) {}
    fn set_menu_labels(&self, _labels: &MenuLabels) {}
}

// The `--bar-snapshot` tool renders the bar exactly as the window sends it to
// `UpdateLayeredWindow`, so it needs the renderer and the palette behind the snapshot feature.
#[cfg(all(windows, feature = "snapshot"))]
pub use super::windows::render::render as render_bar;
#[cfg(all(windows, feature = "snapshot"))]
pub use super::windows::theme::palette as bar_palette;
