//! Rounded corners for the popup on Windows 11 (WR-02 AC5). DWM rounds the window and draws the
//! system border and shadow; Windows 10 has no such attribute, so the call fails there.

use std::ffi::c_void;

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows::Win32::UI::HiDpi::GetSystemMetricsForDpi;
use windows::Win32::UI::WindowsAndMessaging::{SM_CXPADDEDBORDER, SM_CXSIZEFRAME, SM_CYCAPTION};

/// Sets `DWMWA_WINDOW_CORNER_PREFERENCE` to round on `hwnd`. `false` when DWM refuses, which is
/// what Windows 10 does for an attribute it doesn't know.
// The attribute is a 4-byte enum, so its size always fits the `u32` the API takes.
#[allow(clippy::cast_possible_truncation)]
pub fn round_corners(hwnd: isize) -> bool {
    let preference = DWMWCP_ROUND;
    // SAFETY: `hwnd` is the popup's window handle, read from winit just before this call, and
    // `preference` is a live `DWM_WINDOW_CORNER_PREFERENCE` whose size is passed with it. A
    // window that is gone makes the call fail, which this function reports as `false`.
    let result = unsafe {
        DwmSetWindowAttribute(
            HWND(hwnd as *mut c_void),
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::from_ref(&preference).cast::<c_void>(),
            std::mem::size_of_val(&preference) as u32,
        )
    };
    match result {
        Ok(()) => true,
        Err(err) => {
            log::info!("popup: DWM would not round the corners: {err}");
            false
        }
    }
}

/// The caption height and the width of the sizing frame with its padded border, in physical px at
/// `dpi`, for placing a framed window by what is drawn (WR-06 AC1).
pub fn frame_metrics(dpi: u32) -> (i32, i32) {
    // SAFETY: `GetSystemMetricsForDpi` reads three system metrics for a DPI value and touches no
    // pointer or handle.
    unsafe {
        (
            GetSystemMetricsForDpi(SM_CYCAPTION, dpi),
            GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi)
                + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi),
        )
    }
}
