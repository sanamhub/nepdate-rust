//! The popup's window frame (WR-02 AC5): who draws the rounded corners, if anyone. This module
//! decides; `ui_bridge.rs` reads the window handle and tells the markup.

/// How the popup's corners are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// The OS rounds the window and draws its border and shadow (Windows 11).
    System,
    /// The markup draws a 10 px radius on a transparent window (macOS, Wayland).
    Rounded,
    /// Square corners with a 1 px `hairline` border (Windows 10, X11, and when the OS refuses).
    Square,
}

/// What kind of window the popup got. The raw handle's variant, without the handle types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// A Win32 window, with its `HWND` value.
    Win32(isize),
    /// A Cocoa view.
    AppKit,
    /// A Wayland surface, which composites with alpha.
    Wayland,
    /// X11, where transparency needs a compositor that this app cannot check for, or anything
    /// else.
    Other,
}

/// Picks the frame for `surface`. On Windows the answer is the OS's: `DwmSetWindowAttribute`
/// fails on Windows 10, which has no rounded window corners, and that failure is the signal to
/// keep the square hairline frame.
#[must_use]
pub fn choose(surface: Surface) -> Frame {
    match surface {
        Surface::Win32(hwnd) => {
            if round_with_os(hwnd) {
                Frame::System
            } else {
                Frame::Square
            }
        }
        Surface::AppKit | Surface::Wayland => Frame::Rounded,
        Surface::Other => Frame::Square,
    }
}

/// Asks DWM for rounded corners. `false` when the OS has none to give.
#[cfg(windows)]
fn round_with_os(hwnd: isize) -> bool {
    super::windows::frame::round_corners(hwnd)
}

/// There is no DWM off Windows.
#[cfg(not(windows))]
fn round_with_os(_hwnd: isize) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wr02_ac5_macos_and_wayland_round_in_the_markup() {
        assert_eq!(choose(Surface::AppKit), Frame::Rounded);
        assert_eq!(choose(Surface::Wayland), Frame::Rounded);
    }

    #[test]
    fn wr02_ac5_x11_keeps_square_corners_and_the_hairline() {
        assert_eq!(choose(Surface::Other), Frame::Square);
    }

    #[cfg(not(windows))]
    #[test]
    fn wr02_ac5_a_win32_handle_without_dwm_stays_square() {
        assert_eq!(choose(Surface::Win32(1)), Frame::Square);
    }
}
