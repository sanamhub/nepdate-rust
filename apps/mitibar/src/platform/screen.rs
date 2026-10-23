//! What the popup needs to know about the screen before its own window exists (WR-02 AC1).

use crate::app::bar_place::Monitor;

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
