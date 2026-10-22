//! Monitor and shell queries for the bar (ADR-0019 points 6 and 7): the monitors with their device
//! names, work areas and DPI, and what the default position needs from the taskbar. Read-only
//! Win32 calls; the placement itself is pure and lives in `app::bar_place`.
// FFI conventions this module cannot avoid: `&x` where `*const T` is expected, and the monitor
// callback's `LPARAM` carrying a pointer.
#![allow(clippy::borrow_as_ptr, clippy::ptr_as_ptr)]

use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, GetWindowRect};
use windows::core::{BOOL, w};

use crate::app::bar_place::{Monitor, Rect, Shell};

/// `MONITORINFOF_PRIMARY`.
const MONITORINFOF_PRIMARY: u32 = 1;
/// The first Windows 11 build.
const WINDOWS_11_BUILD: u32 = 22_000;

/// Every attached monitor. Empty when the query fails, which the caller treats as "nothing to
/// place the bar on".
pub fn list() -> Vec<Monitor> {
    let mut monitors: Vec<Monitor> = Vec::new();
    // SAFETY: the callback only runs during this call and `monitors` outlives it; the LPARAM is
    // a pointer to it, read back as the same type in the callback.
    let ok = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(std::ptr::from_mut::<Vec<Monitor>>(&mut monitors) as isize),
        )
    };
    if !ok.as_bool() {
        log::warn!("bar: EnumDisplayMonitors failed");
        monitors.clear();
    }
    monitors
}

/// The `EnumDisplayMonitors` callback: describes one monitor and pushes it.
unsafe extern "system" fn collect(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    // SAFETY: `data` is the pointer `list` passed, valid for the whole enumeration.
    let monitors = unsafe { &mut *(data.0 as *mut Vec<Monitor>) };
    if let Some(described) = describe(monitor) {
        monitors.push(described);
    }
    BOOL::from(true)
}

/// One monitor's name, rectangles and DPI, or `None` when Windows won't say.
// `cbSize` is the struct's size, a few hundred bytes, so the usize-to-u32 cast never truncates.
#[allow(clippy::cast_possible_truncation)]
fn describe(monitor: HMONITOR) -> Option<Monitor> {
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    // SAFETY: `info` is a MONITORINFOEXW with `cbSize` set, passed through its MONITORINFO prefix
    // as the API documents.
    let ok = unsafe {
        GetMonitorInfoW(
            monitor,
            std::ptr::from_mut::<MONITORINFOEXW>(&mut info).cast(),
        )
    };
    if !ok.as_bool() {
        return None;
    }
    let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
    // SAFETY: both out values are valid. A failure leaves 96, the unscaled default.
    let dpi =
        unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &raw mut dpi_x, &raw mut dpi_y) };
    if let Err(error) = dpi {
        log::debug!("bar: GetDpiForMonitor failed ({error}); using 96");
    }
    let name_len = info
        .szDevice
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(info.szDevice.len());
    Some(Monitor {
        name: String::from_utf16_lossy(&info.szDevice[..name_len]),
        bounds: rect(info.monitorInfo.rcMonitor),
        work: rect(info.monitorInfo.rcWork),
        dpi: dpi_x,
        primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
    })
}

fn rect(r: RECT) -> Rect {
    Rect::new(r.left, r.top, r.right, r.bottom)
}

/// The window rectangle of `hwnd`, or `None`.
pub fn window_rect(hwnd: HWND) -> Option<Rect> {
    let mut r = RECT::default();
    // SAFETY: `r` is a valid out value.
    unsafe { GetWindowRect(hwnd, &mut r) }.ok()?;
    Some(rect(r))
}

/// What the default position needs from the shell: the notification area's rectangle and whether
/// the taskbar's icons are centred (ADR-0019 point 7).
pub fn shell() -> Shell {
    // SAFETY: FindWindowW is a read-only query; a missing taskbar is a normal Explorer-restart
    // state.
    let tray = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok();
    let tray_notify = tray
        // SAFETY: FindWindowExW is a read-only query on the taskbar window found above.
        .and_then(|tray| unsafe { FindWindowExW(Some(tray), None, w!("TrayNotifyWnd"), None) }.ok())
        .and_then(window_rect);
    Shell {
        centred: is_centred(windows_build(), taskbar_alignment()),
        tray_notify,
    }
}

/// Whether the taskbar's icons are centred: Windows 11 does that unless `TaskbarAl` says left
/// (0). Windows 10 has no such value and always starts at the left.
pub fn is_centred(build: Option<u32>, taskbar_al: Option<u32>) -> bool {
    build.is_some_and(|build| build >= WINDOWS_11_BUILD) && taskbar_al != Some(0)
}

/// `CurrentBuildNumber`, such as 26300 for Windows 11 25H2.
fn windows_build() -> Option<u32> {
    windows_registry::LOCAL_MACHINE
        .open(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .and_then(|key| key.get_string("CurrentBuildNumber"))
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// `TaskbarAl`: 0 for left-aligned icons, 1 for centred. Absent until the user changes it.
fn taskbar_alignment() -> Option<u32> {
    windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced")
        .and_then(|key| key.get_u32("TaskbarAl"))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wr08_ac5_windows_11_centres_unless_the_alignment_says_left() {
        assert!(
            is_centred(Some(26_300), None),
            "the default of a Windows 11 install"
        );
        assert!(is_centred(Some(22_000), Some(1)));
        assert!(!is_centred(Some(26_300), Some(0)), "left-aligned");
        assert!(!is_centred(Some(19_045), None), "Windows 10");
        assert!(
            !is_centred(Some(19_045), Some(1)),
            "a stray value on Windows 10"
        );
        assert!(
            !is_centred(None, None),
            "an unreadable build is treated as Windows 10"
        );
    }

    /// The only machine-dependent check in this file: the machine running the tests has a
    /// monitor with a name, a positive size and a DPI, and exactly one primary.
    #[test]
    fn the_machines_monitors_are_described() {
        let monitors = list();
        if monitors.is_empty() {
            // A headless CI session has no display to enumerate.
            return;
        }
        assert_eq!(monitors.iter().filter(|m| m.primary).count(), 1);
        for monitor in &monitors {
            assert!(!monitor.name.is_empty());
            assert!(monitor.bounds.right > monitor.bounds.left && monitor.bounds.height() > 0);
            assert!(monitor.dpi >= 96, "{} dpi", monitor.dpi);
            assert!(monitor.work.left >= monitor.bounds.left);
            assert!(monitor.work.bottom <= monitor.bounds.bottom);
        }
    }
}
