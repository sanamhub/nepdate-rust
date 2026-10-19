//! The taskbar text surface (WR-12, ADR-0016). Windows 11 only; a no-op elsewhere. The rest of
//! the app sees only this trait and these types, never the `unsafe` Win32 in `windows/`.

use crate::app::taskbar::{Rect, TaskbarLines};

/// The WR-01 AC5 menu labels, built from the `@tr` strings by `ui_bridge.rs`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuLabels {
    pub open: String,
    pub copy_bs: String,
    pub copy_ad: String,
    pub settings: String,
    pub quit: String,
}

/// A WR-01 AC5 menu item, shared by the tray menu and the taskbar text's right-click menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Open,
    CopyBs,
    CopyAd,
    Settings,
    Quit,
}

/// Events the taskbar text delivers to the app (ADR-0016 point 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarEvent {
    /// Left click: toggle the popup above `anchor` (the text window, in screen coordinates).
    Toggle { anchor: Rect },
    /// Right-click menu pick.
    Menu(MenuItem),
}

/// The taskbar text surface the app updates (ADR-0016). `set_lines` is called once per tick; the
/// window redraws only when something changed (point 6).
pub trait TaskbarText {
    fn set_lines(&self, lines: &TaskbarLines);
    fn set_enabled(&self, on: bool);
    fn set_menu_labels(&self, labels: &MenuLabels);
}

/// Creates the platform taskbar text (Windows 11) or a no-op (elsewhere). `on_event` receives
/// `TaskbarEvent`s on the main thread; `on_taskbar_scheme` receives `true` for a dark taskbar so
/// the tray icon can follow the taskbar scheme at once (ADR-0016 point 5).
pub fn create(
    on_event: impl FnMut(TaskbarEvent) + 'static,
    on_taskbar_scheme: impl FnMut(bool) + 'static,
) -> Box<dyn TaskbarText> {
    #[cfg(windows)]
    {
        Box::new(super::windows::WindowsTaskbarText::new(
            on_event,
            on_taskbar_scheme,
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = (on_event, on_taskbar_scheme);
        Box::new(Noop)
    }
}

/// The no-op on macOS and Linux: the tray icon alone shows the date there (ADR-0016).
#[cfg(not(windows))]
struct Noop;

#[cfg(not(windows))]
impl TaskbarText for Noop {
    fn set_lines(&self, _lines: &TaskbarLines) {}
    fn set_enabled(&self, _on: bool) {}
    fn set_menu_labels(&self, _labels: &MenuLabels) {}
}

// The `--taskbar-snapshot` tool renders the text exactly as the controller sends it to
// `UpdateLayeredWindow`, so it needs the renderer and the ink colour behind the snapshot feature.
#[cfg(all(windows, feature = "snapshot"))]
pub use super::windows::render::render as render_taskbar_text;
#[cfg(all(windows, feature = "snapshot"))]
pub use super::windows::theme::ink as taskbar_ink;
