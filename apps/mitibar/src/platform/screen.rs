//! What the popup needs to know about the screen before its own window exists (WR-02 AC1).

use crate::app::bar_place::Monitor;

/// Every monitor with its work area and scale, for placing the Settings window (WR-06 AC1). Empty
/// where the OS won't say; macOS and Linux ask winit through the popup's window instead.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn monitors() -> Vec<Monitor> {
    #[cfg(windows)]
    {
        super::windows::monitors::list()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// The caption height and the sizing frame's width with its padded border, in physical px at
/// `dpi`, for a framed window. `None` where the window is placed by the OS.
#[cfg_attr(not(windows), allow(dead_code))]
// Windows always has the metrics and the other platforms never do: one signature for both.
#[cfg_attr(windows, allow(clippy::unnecessary_wraps))]
pub fn frame_metrics(dpi: u32) -> Option<(i32, i32)> {
    #[cfg(windows)]
    {
        Some(super::windows::frame::frame_metrics(dpi))
    }
    #[cfg(not(windows))]
    {
        let _ = dpi;
        None
    }
}

/// The primary monitor with its work area and scale, or `None` when the OS won't say or the
/// platform places the popup another way (macOS and Linux use winit's monitor instead).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn primary_monitor() -> Option<Monitor> {
    #[cfg(windows)]
    {
        super::windows::monitors::list()
            .into_iter()
            .find(|monitor| monitor.primary)
    }
    #[cfg(not(windows))]
    {
        None
    }
}
