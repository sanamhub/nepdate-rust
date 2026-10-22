//! Top-most upkeep and the full-screen check, driven by events and never by a timer (ADR-0019
//! points 3 and 4, WR-08 AC9). A `SetWinEventHook` callback runs on the UI thread, in the message
//! loop, and does one thing: post a message to the bar's controller window. The controller then
//! re-asserts `HWND_TOPMOST` and re-runs `fullscreen_window` from its own window procedure, under
//! its own re-entrancy rules.

use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::Shell::{
    QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_FOREGROUND, GetClassNameW, GetForegroundWindow,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, OBJID_WINDOW, PostMessageW,
    WINEVENT_OUTOFCONTEXT,
};

use super::monitors::window_rect;
use crate::app::bar_place::{Rect, covers_monitor};

/// The controller window the callback posts to, and the message it posts. Statics because a
/// `WINEVENTPROC` carries no user data; there is one bar per process.
static NOTIFY: AtomicIsize = AtomicIsize::new(0);
static NOTIFY_MESSAGE: AtomicU32 = AtomicU32::new(0);
/// The full-screen window whose location changes are being watched, so the callback can drop every
/// other window's.
static WATCHED: AtomicIsize = AtomicIsize::new(0);

/// Windows that cover their monitor without being a full-screen app: the desktop and the taskbars.
const SHELL_CLASSES: [&str; 4] = [
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
];

/// The installed event hooks. Dropping it unhooks them.
pub struct Hooks {
    foreground: HWINEVENTHOOK,
    /// Watches the full-screen window while the bar is hidden for it, so leaving full screen in
    /// place (F11 in a browser changes no foreground window) shows the bar again.
    location: Option<HWINEVENTHOOK>,
}

impl Hooks {
    /// Hooks foreground changes; each one posts `message` to `notify`.
    pub fn install(notify: HWND, message: u32) -> Option<Hooks> {
        NOTIFY.store(notify.0 as isize, Ordering::Relaxed);
        NOTIFY_MESSAGE.store(message, Ordering::Relaxed);
        // SAFETY: the callback is a plain function that only posts a message; out-of-context
        // delivery means it runs on this thread, inside our own message loop.
        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(on_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        };
        if hook.is_invalid() {
            log::warn!(
                "bar: SetWinEventHook failed; the bar will not follow the foreground window"
            );
            return None;
        }
        Some(Hooks {
            foreground: hook,
            location: None,
        })
    }

    /// Starts watching `window`'s location, replacing any earlier watch.
    pub fn watch(&mut self, window: HWND) {
        if WATCHED.load(Ordering::Relaxed) == window.0 as isize && self.location.is_some() {
            return;
        }
        self.unwatch();
        let mut process = 0u32;
        // SAFETY: `process` is a valid out value; the thread id is the return value.
        let thread = unsafe { GetWindowThreadProcessId(window, Some(&raw mut process)) };
        if thread == 0 {
            return;
        }
        WATCHED.store(window.0 as isize, Ordering::Relaxed);
        // SAFETY: as in `install`; the hook is limited to the one window's process and thread.
        let hook = unsafe {
            SetWinEventHook(
                EVENT_OBJECT_LOCATIONCHANGE,
                EVENT_OBJECT_LOCATIONCHANGE,
                None,
                Some(on_event),
                process,
                thread,
                WINEVENT_OUTOFCONTEXT,
            )
        };
        if hook.is_invalid() {
            log::debug!("bar: could not watch the full-screen window");
            WATCHED.store(0, Ordering::Relaxed);
        } else {
            self.location = Some(hook);
        }
    }

    /// Stops watching the full-screen window.
    pub fn unwatch(&mut self) {
        WATCHED.store(0, Ordering::Relaxed);
        if let Some(hook) = self.location.take() {
            // SAFETY: the hook came from `SetWinEventHook` and is unhooked once.
            let _ = unsafe { UnhookWinEvent(hook) };
        }
    }
}

impl Drop for Hooks {
    fn drop(&mut self) {
        self.unwatch();
        // SAFETY: the hook came from `SetWinEventHook` and is unhooked once.
        let _ = unsafe { UnhookWinEvent(self.foreground) };
        NOTIFY.store(0, Ordering::Relaxed);
    }
}

/// The hook callback. It filters, posts one message and returns.
unsafe extern "system" fn on_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if event == EVENT_OBJECT_LOCATIONCHANGE
        && (id_object != OBJID_WINDOW.0 || hwnd.0 as isize != WATCHED.load(Ordering::Relaxed))
    {
        return;
    }
    let notify = NOTIFY.load(Ordering::Relaxed);
    if notify == 0 {
        return;
    }
    // SAFETY: posting to a window handle is safe even if the window is gone; the call then fails.
    let _ = unsafe {
        PostMessageW(
            Some(HWND(notify as *mut std::ffi::c_void)),
            NOTIFY_MESSAGE.load(Ordering::Relaxed),
            WPARAM(0),
            LPARAM(0),
        )
    };
}

/// The foreground window if it is a full-screen app that should hide the bar (ADR-0019 point 4):
/// it covers its whole monitor, that monitor is the bar's (`bar_monitor`), and it is not the
/// desktop, a taskbar or one of our own windows; or `SHQueryUserNotificationState` reports a
/// full-screen Direct3D app or presentation mode.
pub fn fullscreen_window(bar_monitor: Option<HMONITOR>) -> Option<HWND> {
    // SAFETY: GetForegroundWindow is a read-only query.
    let front = unsafe { GetForegroundWindow() };
    if front.0.is_null() {
        return None;
    }
    let mut process = 0u32;
    // SAFETY: `process` is a valid out value.
    unsafe { GetWindowThreadProcessId(front, Some(&raw mut process)) };
    // SAFETY: GetCurrentProcessId has no preconditions.
    if process == unsafe { GetCurrentProcessId() } || is_shell_window(front) {
        return None;
    }
    // SAFETY: both are read-only queries on the foreground window.
    if !unsafe { IsWindowVisible(front) }.as_bool() || unsafe { IsIconic(front) }.as_bool() {
        return None;
    }
    // SAFETY: SHQueryUserNotificationState is a read-only query.
    let exclusive = matches!(
        unsafe { SHQueryUserNotificationState() },
        Ok(QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
    );
    if exclusive || covers_its_monitor(front, bar_monitor) {
        Some(front)
    } else {
        None
    }
}

/// Whether `window` covers the whole of its monitor, and that monitor is the bar's (or the bar's
/// is unknown).
fn covers_its_monitor(window: HWND, bar_monitor: Option<HMONITOR>) -> bool {
    // SAFETY: MonitorFromWindow is a read-only query and always returns a monitor with this flag.
    let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
    if bar_monitor.is_some_and(|bar| bar != monitor) {
        return false;
    }
    let mut info = MONITORINFO {
        cbSize: u32::try_from(std::mem::size_of::<MONITORINFO>()).unwrap_or(0),
        ..Default::default()
    };
    // SAFETY: `info` has `cbSize` set.
    if !unsafe { GetMonitorInfoW(monitor, &raw mut info) }.as_bool() {
        return false;
    }
    let bounds = Rect::new(
        info.rcMonitor.left,
        info.rcMonitor.top,
        info.rcMonitor.right,
        info.rcMonitor.bottom,
    );
    window_rect(window).is_some_and(|rect| covers_monitor(rect, bounds))
}

/// Whether `window` is the desktop or a taskbar, which cover a monitor without being a full-screen
/// app.
fn is_shell_window(window: HWND) -> bool {
    let mut class = [0u16; 64];
    // SAFETY: `class` is a valid buffer; the return value is the length written.
    let length = unsafe { GetClassNameW(window, &mut class) };
    let length = usize::try_from(length).unwrap_or(0);
    let name = String::from_utf16_lossy(&class[..length]);
    SHELL_CLASSES.contains(&name.as_str())
}
