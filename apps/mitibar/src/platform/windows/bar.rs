//! The mini bar: a hidden top-level controller window and the top-most, layered bar window it owns
//! (ADR-0019 points 1, 3 to 5, 8 and 10). Both live on the main thread; Slint's winit event loop
//! dispatches every message of that thread, so there is no second message loop.
//!
//! The controller window exists for the whole run. It receives what the bar window can't: the
//! `TaskbarCreated` broadcast after Explorer restarts (the bar window is destroyed with its owner,
//! the taskbar), display, DPI and theme changes, and the messages the foreground hook posts. The
//! bar window is created when it is first needed and recreated after an Explorer restart.
//!
//! Soundness rule (ADR-0016, kept): never hold a `&mut Controller` across a Win32 call that can
//! send a message or run a callback. The window procedure borrows the controller with
//! `try_borrow_mut`; re-entrant messages fall through to `DefWindowProcW` (or are re-posted when
//! they carry state), and the app callbacks (`on_event`, `on_scheme`) run only in `deliver`, after
//! the borrow is dropped. Every `borrow_mut` site below says why it cannot re-enter.
// FFI conventions this module cannot avoid: `&x` where `*const T` is expected.
#![allow(clippy::borrow_as_ptr, clippy::ptr_as_ptr)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};

use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HBITMAP,
    HBRUSH, HDC, HGDIOBJ, MONITOR_DEFAULTTONEAREST, MonitorFromWindow, RGBQUAD, ReleaseDC,
    SelectObject,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DestroyWindow, FindWindowW, GWLP_USERDATA, GetCursorPos,
    GetForegroundWindow, GetSystemMetrics, GetWindowLongPtrW, HCURSOR, HICON, HWND_TOPMOST,
    IDC_ARROW, IsWindow, LoadCursorW, MA_NOACTIVATE, MF_CHECKED, MF_STRING, MF_UNCHECKED,
    PostMessageW, RegisterClassW, RegisterWindowMessageW, SM_CXDRAG, SM_CYDRAG, SW_HIDE,
    SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetForegroundWindow,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx,
    ULW_ALPHA, UpdateLayeredWindow, WINDOW_STYLE, WM_APP, WM_CAPTURECHANGED, WM_DISPLAYCHANGE,
    WM_DPICHANGED, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE,
    WM_RBUTTONUP, WM_SETTINGCHANGE, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::app::bar::BarLines;
use crate::app::bar_place::{
    self, Monitor, Point, Rect, Saved, Size, drag_to, monitor_at, snap, to_saved,
};
use crate::app::bar_pointer::{Action as PointerAction, Pointer};
use crate::platform::bar::{Anchor, Bar, BarEvent, MenuItem, MenuLabels};
use crate::platform::taskbar_scheme::taskbar_is_dark;

use super::foreground::{self, Hooks};
use super::monitors;
use super::render;
use super::theme;

/// The controller window class (ADR-0019 point 1).
const CONTROLLER_CLASS: PCWSTR = w!("mitibar-bar-controller");
/// The bar window class. `CS_DBLCLKS` makes Windows report the second press of a double-click as
/// `WM_LBUTTONDBLCLK`, which is how the second click is told apart from a click (ADR-0019 point 8).
const BAR_CLASS: PCWSTR = w!("mitibar-bar");

/// Controller messages.
const WM_APP_REFRESH: u32 = WM_APP + 1;
/// Posted by the foreground hook (`foreground.rs`).
const WM_APP_FOREGROUND: u32 = WM_APP + 2;
/// A `WM_SETTINGCHANGE` that arrived during a borrow, reposted with the setting in `wParam`
/// (`SETTING_COLOR` or `SETTING_TRAY`) instead of the sender's string pointer.
const WM_APP_SETTING: u32 = WM_APP + 3;
const SETTING_COLOR: usize = 1;
const SETTING_TRAY: usize = 2;

/// `WM_MOUSELEAVE`, which `windows` keeps in a feature this crate does not otherwise use.
const WM_MOUSELEAVE: u32 = 0x02A3;

/// The `TaskbarCreated` registered message id, read by the window procedure without a borrow.
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
/// The controller window, which re-posted messages go to whichever window received them.
static CONTROLLER: AtomicIsize = AtomicIsize::new(0);

/// What is on screen, so a tick can skip a redraw when nothing changed (WR-08 AC9).
struct Drawn {
    key: Key,
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    /// The bar's top-left in screen coordinates once it has been placed.
    at: Option<Point>,
}

/// Everything the pixels depend on; the bar redraws only when one of these changes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Key {
    lines: BarLines,
    dark: bool,
    hover: bool,
    dpi: u32,
}

/// Why the bar is hidden, so each reason is logged once (ADR-0019 point 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hidden {
    Disabled,
    NothingToShow,
    FullScreen,
    NoTaskbar,
    NoMonitor,
    RenderFailed,
    PlaceFailed,
}

/// The outcome for the window procedure: either `result` is returned, or the default handler runs.
enum Outcome {
    Return(LRESULT),
    Default,
}

/// Work the window procedure must do after the controller borrow ends. One value lives for one
/// message, so the size of the largest variant costs nothing.
#[allow(clippy::large_enum_variant)]
enum Action {
    None,
    /// Deliver events and, optionally, the new taskbar scheme to the app callbacks.
    Deliver {
        events: Vec<BarEvent>,
        scheme: Option<bool>,
    },
    /// Run the right-click menu (modal) and then deliver the chosen item.
    Menu {
        labels: MenuLabels,
        locked: bool,
        bar: HWND,
        point: POINT,
        anchor: Anchor,
    },
}

// The flags are independent facts about the window (enabled, locked, hover, dragging, shown,
// full-screen); a state machine would not make them fewer.
#[allow(clippy::struct_excessive_bools)]
struct Controller {
    hwnd: HWND,
    bar: HWND,
    /// The `RefCell<Controller>` this controller lives in, stored in `GWLP_USERDATA`.
    cell: *const RefCell<Controller>,
    lines: BarLines,
    /// The `bar` setting.
    enabled: bool,
    locked: bool,
    labels: MenuLabels,
    saved: Saved,
    dark: bool,
    hover: bool,
    pointer: Pointer,
    /// Set from the first drag move to the release; a refresh never moves a bar under the pointer.
    dragging: bool,
    /// The monitors as they were when the drag began, so a move doesn't enumerate them.
    drag_monitors: Vec<Monitor>,
    drawn: Option<Drawn>,
    /// Whether the bar window is currently shown.
    visible: bool,
    hidden_reason: Option<Hidden>,
    /// A full-screen app is in front (ADR-0019 point 4).
    fullscreen: bool,
    hooks: Option<Hooks>,
    /// `Option` so `deliver` can take the callback out, drop the borrow, call it, and put it back.
    on_event: Option<Box<dyn FnMut(BarEvent)>>,
    on_scheme: Option<Box<dyn FnMut(bool)>>,
}

/// The Windows mini bar (ADR-0019). `unsafe` stays inside this module and its siblings. The
/// controller lives in an `Rc<RefCell<Controller>>`; `GWLP_USERDATA` points at the `RefCell`, so
/// the window procedure borrows with `try_borrow_mut` instead of fabricating a second `&mut`.
pub struct WindowsBar {
    inner: Rc<RefCell<Controller>>,
}

impl WindowsBar {
    pub fn new(
        on_event: impl FnMut(BarEvent) + 'static,
        on_taskbar_scheme: impl FnMut(bool) + 'static,
    ) -> Self {
        let inner = Rc::new(RefCell::new(Controller::new(
            Some(Box::new(on_event)),
            Some(Box::new(on_taskbar_scheme)),
        )));
        let cell = Rc::as_ptr(&inner);
        // Cannot re-enter: this runs before the event loop starts, so no window message and no
        // callback can fire yet.
        inner.borrow_mut().bind(cell);
        Self { inner }
    }
}

impl Bar for WindowsBar {
    fn set_lines(&self, lines: &BarLines) {
        // Cannot re-enter: the timer runs between window messages, never while the window
        // procedure holds a borrow, and the refresh under this borrow only sends messages that the
        // window procedure catches with `try_borrow_mut`.
        self.inner.borrow_mut().set_lines(lines);
    }

    fn set_shown(&self, shown: bool) {
        // Cannot re-enter: called from the app's handlers, which `deliver` runs after the borrow
        // is dropped, and from the settings callbacks, which run between window messages.
        self.inner.borrow_mut().set_enabled(shown);
    }

    fn set_locked(&self, locked: bool) {
        // Cannot re-enter, as `set_shown`; and it makes no Win32 call.
        let mut controller = self.inner.borrow_mut();
        controller.locked = locked;
        controller.pointer.set_locked(locked);
    }

    fn set_position(&self, saved: &Saved) {
        // Cannot re-enter, as `set_shown`.
        self.inner.borrow_mut().set_position(saved);
    }

    fn set_menu_labels(&self, labels: &MenuLabels) {
        // Cannot re-enter, as `set_locked`.
        self.inner.borrow_mut().labels.clone_from(labels);
    }
}

impl Controller {
    fn new(
        on_event: Option<Box<dyn FnMut(BarEvent)>>,
        on_scheme: Option<Box<dyn FnMut(bool)>>,
    ) -> Controller {
        // A registration failure leaves the bar hidden but the tray icon working; the controller
        // window is still created so the failure mode stays "bar hidden, one log line".
        register_classes();
        let hwnd = create_controller();
        CONTROLLER.store(hwnd.0 as isize, Ordering::Relaxed);
        // SAFETY: RegisterWindowMessageW only registers a string-to-id mapping.
        let created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        TASKBAR_CREATED.store(created, Ordering::Relaxed);
        // SAFETY: GetSystemMetrics is a read-only query.
        let drag = unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) };
        let hooks = if hwnd.0.is_null() {
            None
        } else {
            Hooks::install(hwnd, WM_APP_FOREGROUND)
        };
        Controller {
            hwnd,
            bar: HWND(std::ptr::null_mut()),
            cell: std::ptr::null(),
            lines: BarLines::default(),
            enabled: false,
            locked: false,
            labels: MenuLabels::default(),
            saved: Saved::default(),
            dark: taskbar_is_dark().unwrap_or(false),
            hover: false,
            pointer: Pointer::new(drag),
            dragging: false,
            drag_monitors: Vec::new(),
            drawn: None,
            visible: false,
            hidden_reason: None,
            fullscreen: false,
            hooks,
            on_event,
            on_scheme,
        }
    }

    /// Stores the `RefCell` pointer in the controller window's user data (and the bar window's, when
    /// it exists) so the window procedure can reach this controller with `try_borrow_mut`.
    fn bind(&mut self, cell: *const RefCell<Controller>) {
        self.cell = cell;
        // SAFETY: the window exists and `cell` stays valid for its lifetime; it is cleared in
        // `Controller::drop` before the controller is freed.
        unsafe { SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, cell as isize) };
    }

    fn set_lines(&mut self, lines: &BarLines) {
        if self.lines == *lines {
            return;
        }
        self.lines.clone_from(lines);
        self.refresh();
    }

    fn set_enabled(&mut self, on: bool) {
        if self.enabled == on {
            return;
        }
        self.enabled = on;
        self.refresh();
    }

    fn set_position(&mut self, saved: &Saved) {
        if self.saved == *saved {
            return;
        }
        self.saved.clone_from(saved);
        self.refresh();
    }

    /// The single refresh path: re-reads the scheme, monitors and DPI, re-renders and re-places
    /// only when something changed (WR-08 AC9), and hides with one log line otherwise (ADR-0019
    /// point 10).
    // The scale is a DPI of a few hundred at most, so the f32 cast loses nothing a pixel could show.
    #[allow(clippy::cast_precision_loss)]
    fn refresh(&mut self) {
        if self.dragging {
            return; // the drag ends with its own refresh
        }
        if !self.enabled {
            self.hide(Hidden::Disabled);
            return;
        }
        if self.lines.is_empty() {
            self.hide(Hidden::NothingToShow);
            return;
        }
        if self.fullscreen {
            self.hide(Hidden::FullScreen);
            return;
        }
        if !self.ensure_bar_window() {
            self.hide(Hidden::NoTaskbar);
            return;
        }
        let monitors = monitors::list();
        let Some(target) = bar_place::target_monitor(&self.saved, &monitors) else {
            self.hide(Hidden::NoMonitor);
            return;
        };
        self.dark = taskbar_is_dark().unwrap_or(false);
        let key = Key {
            lines: self.lines.clone(),
            dark: self.dark,
            hover: self.hover,
            dpi: target.dpi,
        };
        let redraw = self.drawn.as_ref().is_none_or(|drawn| drawn.key != key);
        if redraw {
            let scale = key.dpi as f32 / 96.0;
            match render::render(&key.lines, scale, theme::palette(key.dark), key.hover) {
                Ok(rendered) => {
                    // New pixels always go to the window, so the placement below starts afresh.
                    self.drawn = Some(Drawn {
                        key,
                        pixels: rendered.pixels,
                        width: rendered.width,
                        height: rendered.height,
                        at: None,
                    });
                }
                Err(error) => {
                    log::warn!("bar: render failed: {error}");
                    self.hide(Hidden::RenderFailed);
                    return;
                }
            }
        }
        let Some(drawn) = self.drawn.as_mut() else {
            return;
        };
        let size = Size {
            width: i32::try_from(drawn.width).unwrap_or(i32::MAX),
            height: i32::try_from(drawn.height).unwrap_or(i32::MAX),
        };
        // The shell is only asked about for the default position.
        let shell = if self.saved.monitor == target.name {
            bar_place::Shell {
                centred: false,
                tray_notify: None,
            }
        } else {
            monitors::shell()
        };
        let at = bar_place::position(&self.saved, target, &shell, size);
        let moved = drawn.at != Some(at);
        if moved {
            drawn.at = Some(at);
            if let Err(error) =
                update_window(self.bar, &drawn.pixels, drawn.width, drawn.height, at)
            {
                log::warn!("bar: UpdateLayeredWindow failed: {error}");
                drawn.at = None;
                self.hide(Hidden::PlaceFailed);
                return;
            }
        }
        if !self.visible {
            // SAFETY: ShowWindow is safe on the window we created. It may send messages
            // synchronously; the window procedure catches them with `try_borrow_mut` and falls
            // through.
            unsafe {
                let _ = ShowWindow(self.bar, SW_SHOWNOACTIVATE);
            }
            self.visible = true;
            self.raise();
        }
        if self.hidden_reason.take().is_some() {
            log::info!("bar shown");
        }
    }

    /// Hides the bar and logs why, once.
    fn hide(&mut self, reason: Hidden) {
        if self.hidden_reason != Some(reason) {
            log::info!("bar hidden: {reason:?}");
            self.hidden_reason = Some(reason);
        }
        if self.visible && !self.bar.0.is_null() {
            // SAFETY: ShowWindow is safe on the window we created.
            unsafe {
                let _ = ShowWindow(self.bar, SW_HIDE);
            }
        }
        self.visible = false;
        // The pixels stay, so a hidden refresh never re-renders; the cache is cleared only when
        // what it depends on changes.
    }

    /// Re-asserts `HWND_TOPMOST` without activating (ADR-0019 point 3). Called when the bar is
    /// shown and on foreground changes, never on a timer.
    fn raise(&self) {
        if self.bar.0.is_null() {
            return;
        }
        // SAFETY: SetWindowPos on our own window, which it neither moves nor resizes nor
        // activates.
        unsafe {
            let _ = SetWindowPos(
                self.bar,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    /// Makes sure the bar window exists, owned by the taskbar. `false` when there is no taskbar to
    /// own it (Explorer isn't running yet).
    fn ensure_bar_window(&mut self) -> bool {
        // SAFETY: IsWindow is a read-only query and accepts any handle.
        if !self.bar.0.is_null() && unsafe { IsWindow(Some(self.bar)) }.as_bool() {
            return true;
        }
        self.visible = false;
        self.drawn = None;
        self.bar = create_bar_window();
        if self.bar.0.is_null() {
            return false;
        }
        // SAFETY: the window exists and `cell` stays valid for its lifetime.
        unsafe { SetWindowLongPtrW(self.bar, GWLP_USERDATA, self.cell as isize) };
        true
    }

    /// Recreates the bar window after Explorer restarted (`TaskbarCreated`): the old one was
    /// destroyed with its owner, and the new taskbar is the owner of the new one (ADR-0019 point 1).
    fn recreate_bar_window(&mut self) {
        if !self.bar.0.is_null() {
            // SAFETY: destroying our own window; it fails harmlessly if the system already did.
            unsafe {
                let _ = DestroyWindow(self.bar);
            }
        }
        self.bar = HWND(std::ptr::null_mut());
        self.visible = false;
        self.drawn = None;
        self.pointer = Pointer::new(Self::pointer_drag());
        self.pointer.set_locked(self.locked);
        self.refresh();
    }

    /// The drag rectangle, read again from the system.
    fn pointer_drag() -> (i32, i32) {
        // SAFETY: GetSystemMetrics is a read-only query.
        unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) }
    }

    /// The foreground window changed, or the full-screen window moved or resized: re-run the
    /// full-screen check and re-assert top-most (ADR-0019 points 3 and 4).
    fn on_foreground(&mut self) {
        let bar_monitor = (!self.bar.0.is_null()).then(|| {
            // SAFETY: MonitorFromWindow is a read-only query and always returns a monitor with
            // this flag.
            unsafe { MonitorFromWindow(self.bar, MONITOR_DEFAULTTONEAREST) }
        });
        let front = foreground::fullscreen_window(bar_monitor);
        if let Some(hooks) = self.hooks.as_mut() {
            match front {
                Some(window) => hooks.watch(window),
                None => hooks.unwatch(),
            }
        }
        let fullscreen = front.is_some();
        if fullscreen != self.fullscreen {
            self.fullscreen = fullscreen;
            self.refresh();
        } else if !fullscreen && self.visible {
            self.raise();
        }
    }

    /// Where the popup opens for something done on the bar, or `None` when the bar has no place.
    fn anchor(&self) -> Option<Anchor> {
        let drawn = self.drawn.as_ref()?;
        let at = drawn.at?;
        let bar = Rect::at(
            at,
            Size {
                width: i32::try_from(drawn.width).ok()?,
                height: i32::try_from(drawn.height).ok()?,
            },
        );
        let monitors = monitors::list();
        let centre = Point {
            x: i32::midpoint(bar.left, bar.right),
            y: i32::midpoint(bar.top, bar.bottom),
        };
        let monitor = monitor_at(&monitors, centre)?;
        Some(Anchor {
            bar,
            work: monitor.work,
            dpi: monitor.dpi,
        })
    }

    /// Moves the bar under a drag, fully onto the monitor under the pointer.
    fn drag_to(&mut self, wanted: Point) {
        if !self.dragging {
            self.dragging = true;
            self.drag_monitors = monitors::list();
        }
        let Some(drawn) = self.drawn.as_mut() else {
            return;
        };
        let size = Size {
            width: i32::try_from(drawn.width).unwrap_or(i32::MAX),
            height: i32::try_from(drawn.height).unwrap_or(i32::MAX),
        };
        let Some((at, _)) = drag_to(wanted, cursor(), size, &self.drag_monitors) else {
            return;
        };
        if drawn.at == Some(at) {
            return;
        }
        drawn.at = Some(at);
        // SAFETY: SetWindowPos on our own window; it only moves it. It may send messages
        // synchronously, which the window procedure catches with `try_borrow_mut`.
        unsafe {
            let _ = SetWindowPos(
                self.bar,
                None,
                at.x,
                at.y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    /// Ends a drag: snaps the drop to the taskbar's centre line, remembers the place and redraws at
    /// the DPI of the monitor it landed on. Returns the position to save.
    fn finish_drag(&mut self) -> Option<Saved> {
        let monitors = std::mem::take(&mut self.drag_monitors);
        self.dragging = false;
        let drawn = self.drawn.as_ref()?;
        let at = drawn.at?;
        let size = Size {
            width: i32::try_from(drawn.width).ok()?,
            height: i32::try_from(drawn.height).ok()?,
        };
        let (at, monitor) = drag_to(at, cursor(), size, &monitors)?;
        let at = snap(at, size, monitor);
        let saved = to_saved(at, monitor);
        self.saved.clone_from(&saved);
        // The refresh re-renders if the monitor's DPI differs and puts the bar at the saved place.
        if let Some(drawn) = self.drawn.as_mut() {
            drawn.at = None;
        }
        self.refresh();
        Some(saved)
    }

    /// Follows the pointer into or out of the bar: the `surface-raised` hover look (widget-design §5).
    fn set_hover(&mut self, hover: bool) {
        if self.hover == hover {
            return;
        }
        self.hover = hover;
        self.refresh();
    }

    /// The window procedure body, run under a `try_borrow_mut`. Returns the LRESULT decision and
    /// the work to do after the borrow ends. It never calls the app callbacks.
    // MA_NOACTIVATE is a small u32; the isize cast never wraps on either 32- or 64-bit targets.
    // One arm per message is the clearest form of a window procedure.
    #[allow(clippy::cast_possible_wrap, clippy::too_many_lines)]
    fn handle(
        &mut self,
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> (Outcome, Action) {
        let handled = (Outcome::Return(LRESULT(0)), Action::None);
        if hwnd == self.bar {
            match msg {
                WM_MOUSEACTIVATE => {
                    return (
                        Outcome::Return(LRESULT(MA_NOACTIVATE as isize)),
                        Action::None,
                    );
                }
                WM_LBUTTONDOWN => {
                    if let Some(at) = self.drawn.as_ref().and_then(|drawn| drawn.at) {
                        self.pointer.down(cursor(), at);
                        // SAFETY: SetCapture on our own window; the capture ends at the release.
                        unsafe { SetCapture(self.bar) };
                    }
                    return handled;
                }
                WM_LBUTTONDBLCLK => {
                    let action = self.pointer.double_down();
                    // SAFETY: as for the first press.
                    unsafe { SetCapture(self.bar) };
                    return (
                        Outcome::Return(LRESULT(0)),
                        self.deliver_pointer(action, None),
                    );
                }
                WM_MOUSEMOVE => {
                    if !self.hover && !self.pointer.is_pressed() {
                        track_leave(self.bar);
                        self.set_hover(true);
                    }
                    if let PointerAction::DragTo(wanted) = self.pointer.moved(cursor()) {
                        self.drag_to(wanted);
                    }
                    return handled;
                }
                WM_MOUSELEAVE => {
                    self.set_hover(false);
                    return handled;
                }
                WM_LBUTTONUP => {
                    let action = self.pointer.up();
                    // SAFETY: ReleaseCapture is safe; it sends WM_CAPTURECHANGED to this window,
                    // which the window procedure catches with `try_borrow_mut`.
                    unsafe {
                        let _ = ReleaseCapture();
                    }
                    let saved = (action == PointerAction::DragEnd)
                        .then(|| self.finish_drag())
                        .flatten();
                    return (
                        Outcome::Return(LRESULT(0)),
                        self.deliver_pointer(action, saved),
                    );
                }
                WM_CAPTURECHANGED => {
                    // Another window took the capture mid-press: a drag ends where it is.
                    let action = self.pointer.capture_lost();
                    let saved = (action == PointerAction::DragEnd)
                        .then(|| self.finish_drag())
                        .flatten();
                    return (
                        Outcome::Return(LRESULT(0)),
                        self.deliver_pointer(action, saved),
                    );
                }
                WM_RBUTTONUP => {
                    let Some(anchor) = self.anchor() else {
                        return handled;
                    };
                    let point = cursor();
                    return (
                        Outcome::Return(LRESULT(0)),
                        Action::Menu {
                            labels: self.labels.clone(),
                            locked: self.locked,
                            bar: self.bar,
                            point: POINT {
                                x: point.x,
                                y: point.y,
                            },
                            anchor,
                        },
                    );
                }
                _ => {}
            }
        }
        match msg {
            WM_APP_REFRESH | WM_DISPLAYCHANGE | WM_DPICHANGED => {
                self.refresh();
                handled
            }
            WM_APP_FOREGROUND => {
                self.on_foreground();
                handled
            }
            WM_SETTINGCHANGE | WM_APP_SETTING => {
                let code = if msg == WM_APP_SETTING {
                    Some(wparam.0)
                } else {
                    setting_code(lparam)
                };
                let action = match code {
                    Some(SETTING_COLOR) => {
                        self.dark = taskbar_is_dark().unwrap_or(false);
                        self.refresh();
                        Action::Deliver {
                            events: Vec::new(),
                            scheme: Some(self.dark),
                        }
                    }
                    // The taskbar's alignment or size changed: the default position moves with it.
                    Some(SETTING_TRAY) => {
                        self.refresh();
                        Action::None
                    }
                    _ => Action::None,
                };
                (Outcome::Return(LRESULT(0)), action)
            }
            _ if hwnd == self.hwnd
                && msg == TASKBAR_CREATED.load(Ordering::Relaxed)
                && msg != 0 =>
            {
                log::info!("bar: Explorer restarted, recreating the bar window");
                self.recreate_bar_window();
                handled
            }
            _ => (Outcome::Default, Action::None),
        }
    }

    /// The app events for what a pointer action means.
    fn deliver_pointer(&self, action: PointerAction, saved: Option<Saved>) -> Action {
        let event = match action {
            PointerAction::Click => self.anchor().map(BarEvent::Toggle),
            PointerAction::DoubleClick => self.anchor().map(BarEvent::Open),
            PointerAction::DragEnd => saved.map(BarEvent::Moved),
            PointerAction::None | PointerAction::DragTo(_) => None,
        };
        match event {
            Some(event) => Action::Deliver {
                events: vec![event],
                scheme: None,
            },
            None => Action::None,
        }
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        // Unhook first, so no callback posts to a window that is going away.
        self.hooks = None;
        CONTROLLER.store(0, Ordering::Relaxed);
        // SAFETY: clear the user data first so a late message never derefs this freed controller,
        // then destroy the windows (the system may already have destroyed the bar window).
        unsafe {
            if !self.hwnd.0.is_null() {
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            }
            if !self.bar.0.is_null() {
                SetWindowLongPtrW(self.bar, GWLP_USERDATA, 0);
                let _ = DestroyWindow(self.bar);
            }
            if !self.hwnd.0.is_null() {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}

/// The cursor in screen coordinates (physical pixels: the process is per-monitor DPI aware).
fn cursor() -> Point {
    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: `point` is a valid out value; GetCursorPos sends nothing.
    unsafe {
        let _ = GetCursorPos(&mut point);
    }
    Point {
        x: point.x,
        y: point.y,
    }
}

/// Asks for one `WM_MOUSELEAVE` when the cursor leaves the bar.
// The struct is a few bytes, so the usize-to-u32 cast never truncates.
#[allow(clippy::cast_possible_truncation)]
fn track_leave(hwnd: HWND) {
    let mut track = TRACKMOUSEEVENT {
        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dwFlags: TME_LEAVE,
        hwndTrack: hwnd,
        dwHoverTime: 0,
    };
    // SAFETY: `track` is a valid TRACKMOUSEEVENT with `cbSize` set.
    unsafe {
        let _ = TrackMouseEvent(&mut track);
    }
}

/// Runs the app callbacks with no controller borrow held. `Option::take` moves the callbacks out
/// of the controller first, so a callback that calls back into the bar (for example `set_lines`)
/// borrows cleanly instead of aliasing.
fn deliver(cell: &RefCell<Controller>, events: Vec<BarEvent>, scheme: Option<bool>) {
    if events.is_empty() && scheme.is_none() {
        return;
    }
    let (mut on_event, mut on_scheme, bar) = match cell.try_borrow_mut() {
        Ok(mut controller) => (
            controller.on_event.take(),
            controller.on_scheme.take(),
            controller.bar,
        ),
        Err(_) => return,
    };
    // The popup opens from a click on a window that never activates, so the system does not hand
    // the popup the foreground, and Escape and focus-loss hiding need it. Claiming the foreground
    // for the bar first lets the popup take it from the same process.
    if events
        .iter()
        .any(|event| matches!(event, BarEvent::Toggle(_) | BarEvent::Open(_)))
    {
        // SAFETY: SetForegroundWindow on our own window; it fails harmlessly if the window is gone.
        let _ = unsafe { SetForegroundWindow(bar) };
    }
    if let (Some(dark), Some(callback)) = (scheme, on_scheme.as_mut()) {
        callback(dark);
    }
    for event in events {
        if let Some(callback) = on_event.as_mut() {
            callback(event);
        }
    }
    // The callbacks never drop the `Rc` (Quit only queues the event-loop exit), so `cell` is still
    // valid here.
    if let Ok(mut controller) = cell.try_borrow_mut() {
        controller.on_event = on_event;
        controller.on_scheme = on_scheme;
    }
}

/// Builds and runs the right-click menu (WR-08 AC6 items), with no controller borrow held so the
/// modal loop can dispatch the timer and other windows. Returns the chosen item.
// The command id is 0..=8, so the BOOL-to-u32 cast never sees a negative value.
#[allow(clippy::cast_sign_loss)]
unsafe fn run_menu(labels: &MenuLabels, locked: bool, bar: HWND, point: POINT) -> Option<MenuItem> {
    // SAFETY: the popup menu is ours and the labels are UTF-16 buffers we own.
    let menu = unsafe { CreatePopupMenu() }.ok()?;
    let items = [
        (MenuItem::Open, &labels.open),
        (MenuItem::CopyBs, &labels.copy_bs),
        (MenuItem::CopyAd, &labels.copy_ad),
        (MenuItem::Customize, &labels.customize),
        (MenuItem::Lock, &labels.lock),
        (MenuItem::Hide, &labels.hide),
        (MenuItem::Settings, &labels.settings),
        (MenuItem::Quit, &labels.quit),
    ];
    for (index, (item, label)) in items.iter().enumerate() {
        let text: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
        let flags = if *item == MenuItem::Lock {
            MF_STRING | if locked { MF_CHECKED } else { MF_UNCHECKED }
        } else {
            MF_STRING
        };
        // SAFETY: the menu and the text buffer are valid for this call.
        let _ = unsafe { AppendMenuW(menu, flags, index + 1, PCWSTR(text.as_ptr())) };
    }
    // The bar never activates, so claim the foreground for the menu to dismiss on a click
    // elsewhere. The window that had it is given back afterwards unless the choice opens the popup.
    // SAFETY: both are safe calls on our own window and a read-only query.
    let previous = unsafe { GetForegroundWindow() };
    // SAFETY: SetForegroundWindow is safe on our window.
    let _ = unsafe { SetForegroundWindow(bar) };
    // SAFETY: TrackPopupMenuEx blocks and dispatches messages; nothing borrows the controller
    // here, so re-entrant messages (the timer, other windows) are handled normally.
    let command = unsafe {
        TrackPopupMenuEx(
            menu,
            TPM_RETURNCMD.0 | TPM_RIGHTBUTTON.0,
            point.x,
            point.y,
            bar,
            None,
        )
    };
    // SAFETY: the menu is ours.
    let _ = unsafe { DestroyMenu(menu) };
    let chosen = usize::try_from(command.0)
        .ok()
        .and_then(|command| command.checked_sub(1))
        .and_then(|index| items.get(index))
        .map(|(item, _)| *item);
    let opens_popup = matches!(
        chosen,
        Some(MenuItem::Open | MenuItem::Customize | MenuItem::Settings)
    );
    if !opens_popup && !previous.0.is_null() && previous != bar {
        // SAFETY: SetForegroundWindow on a handle that may be gone; the call then fails.
        let _ = unsafe { SetForegroundWindow(previous) };
    }
    chosen
}

/// The window procedure for both windows. The `RefCell<Controller>` pointer lives in
/// `GWLP_USERDATA`.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: the pointer was stored in `bind` or `ensure_bar_window` and cleared in
    // `Controller::drop`; a null pointer means the message arrived before that or after drop.
    let cell = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const RefCell<Controller>;
    if cell.is_null() {
        // SAFETY: before `bind` or after drop, fall through to the default handler.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // SAFETY: `cell` points at the `RefCell<Controller>` inside the `Rc`, heap-allocated and never
    // moved while the windows live.
    let cell = unsafe { &*cell };
    if let Ok(mut controller) = cell.try_borrow_mut() {
        // The borrow ends before any callback runs, so a callback that calls `set_lines`
        // borrows cleanly instead of aliasing.
        let (outcome, action) = controller.handle(hwnd, msg, wparam, lparam);
        drop(controller);
        match action {
            Action::None => {}
            Action::Deliver { events, scheme } => deliver(cell, events, scheme),
            Action::Menu {
                labels,
                locked,
                bar,
                point,
                anchor,
            } => {
                // SAFETY: the menu runs with no borrow held; see `run_menu`.
                if let Some(item) = unsafe { run_menu(&labels, locked, bar, point) } {
                    deliver(cell, vec![BarEvent::Menu { item, anchor }], None);
                }
            }
        }
        match outcome {
            Outcome::Return(result) => result,
            Outcome::Default => {
                // SAFETY: the default handler is safe for any message.
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
        }
    } else {
        // Re-entry: the controller is already borrowed (a synchronous message sent from
        // `ShowWindow`/`UpdateLayeredWindow`/`DestroyWindow`, or a callback). Repost the messages
        // that carry state to the controller window so nothing is lost; drop the rest.
        // `WM_SETTINGCHANGE` is never reposted as is: its `lParam` string belongs to the sender and
        // is freed when the send returns, so read it now and post only which setting changed.
        let repost = match msg {
            WM_APP_REFRESH | WM_DISPLAYCHANGE | WM_DPICHANGED => Some((WM_APP_REFRESH, WPARAM(0))),
            WM_APP_FOREGROUND => Some((WM_APP_FOREGROUND, WPARAM(0))),
            WM_SETTINGCHANGE => setting_code(lparam).map(|code| (WM_APP_SETTING, WPARAM(code))),
            WM_APP_SETTING => Some((WM_APP_SETTING, wparam)),
            _ if msg != 0 && msg == TASKBAR_CREATED.load(Ordering::Relaxed) => {
                Some((msg, WPARAM(0)))
            }
            _ => None,
        };
        if let Some((msg, wparam)) = repost {
            let controller = CONTROLLER.load(Ordering::Relaxed);
            let target = if controller == 0 {
                hwnd
            } else {
                HWND(controller as *mut std::ffi::c_void)
            };
            // SAFETY: posting to our own window is always safe; no pointer travels with it.
            unsafe {
                let _ = PostMessageW(Some(target), msg, wparam, LPARAM(0));
            }
        }
        // SAFETY: the default handler is safe for any message.
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }
}

/// Registers the two window classes. Failure is logged once and left to the caller's fallback.
fn register_classes() {
    // SAFETY: LoadCursorW with a system cursor id is a read-only query.
    let arrow = unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default();
    // SAFETY: RegisterClassW reads the class struct for the duration of the call only.
    unsafe {
        let controller = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HINSTANCE(std::ptr::null_mut()),
            hIcon: HICON::default(),
            hCursor: HCURSOR::default(),
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR(std::ptr::null()),
            lpszClassName: CONTROLLER_CLASS,
        };
        if RegisterClassW(&controller) == 0 {
            log::warn!("bar: RegisterClassW (controller) failed");
        }
        let bar = WNDCLASSW {
            style: CS_DBLCLKS,
            hCursor: arrow,
            lpszClassName: BAR_CLASS,
            ..controller
        };
        if RegisterClassW(&bar) == 0 {
            log::warn!("bar: RegisterClassW (bar) failed");
        }
    }
}

/// Creates the hidden controller window.
fn create_controller() -> HWND {
    // SAFETY: the class name is static; the window is hidden and never shown.
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            CONTROLLER_CLASS,
            w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(HINSTANCE(std::ptr::null_mut())),
            None,
        )
        .unwrap_or(HWND(std::ptr::null_mut()))
    }
}

/// Creates the bar window: a top-level popup owned by `Shell_TrayWnd`, top-most, never activated,
/// with no taskbar button and no Alt+Tab entry (ADR-0019 point 1). Null when there is no taskbar.
fn create_bar_window() -> HWND {
    // SAFETY: FindWindowW is a read-only query.
    let Ok(owner) = (unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }) else {
        return HWND(std::ptr::null_mut());
    };
    // SAFETY: the class name is static and the owner is the taskbar window just found. The
    // window is created hidden; `refresh` places and shows it.
    unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            BAR_CLASS,
            w!(""),
            WS_POPUP,
            0,
            0,
            0,
            0,
            Some(owner),
            None,
            Some(HINSTANCE(std::ptr::null_mut())),
            None,
        )
        .unwrap_or(HWND(std::ptr::null_mut()))
    }
}

/// Which setting a `WM_SETTINGCHANGE` names: `SETTING_COLOR` for `ImmersiveColorSet`,
/// `SETTING_TRAY` for `TraySettings`, `None` for anything else. Call it only while the message is
/// being sent, since the string belongs to the sender.
fn setting_code(lparam: LPARAM) -> Option<usize> {
    match setting_name(lparam)?.as_str() {
        "ImmersiveColorSet" => Some(SETTING_COLOR),
        "TraySettings" => Some(SETTING_TRAY),
        _ => None,
    }
}

/// The `WM_SETTINGCHANGE` `lParam` string, or `None`.
fn setting_name(lparam: LPARAM) -> Option<String> {
    if lparam.0 == 0 {
        return None;
    }
    // SAFETY: lParam points at a null-terminated UTF-16 string from the system; read at most 64
    // units, well inside any such string.
    unsafe {
        let mut units = [0u16; 64];
        for (i, slot) in units.iter_mut().enumerate() {
            *slot = *(lparam.0 as *const u16).add(i);
            if *slot == 0 {
                return Some(String::from_utf16_lossy(&units[..i]));
            }
        }
        Some(String::from_utf16_lossy(&units))
    }
}

/// Pushes the rendered pixels to the layered window with `UpdateLayeredWindow`, putting the
/// window's top-left at `at` in screen coordinates (ADR-0019 point 2).
// The bar size is a few hundred pixels and the blend flags are the two small AC_SRC_* constants,
// so the i32 and u8 casts never wrap or truncate.
#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
fn update_window(
    hwnd: HWND,
    pixels: &[u8],
    width: u32,
    height: u32,
    at: Point,
) -> anyhow::Result<()> {
    // SAFETY: the DIB and DC are created and released within this function.
    let (hdc, bitmap, old) = unsafe { dib_with_pixels(pixels, width, height) }?;
    let dst = POINT { x: at.x, y: at.y };
    let size = windows::Win32::Foundation::SIZE {
        cx: width as i32,
        cy: height as i32,
    };
    let src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    // SAFETY: the DC holds the DIB; the points and size describe it exactly.
    let result = unsafe {
        UpdateLayeredWindow(
            hwnd,
            None,
            Some(&dst),
            Some(&size),
            Some(hdc),
            Some(&src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        )
    };
    // SAFETY: restoring the previous bitmap and deleting the objects created above.
    unsafe {
        SelectObject(hdc, old);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(hdc);
    }
    result.map_err(Into::into)
}

/// A 32-bit top-down DIB with `pixels` copied into it, plus its DC and the previously selected
/// bitmap.
// SAFETY: the caller restores and deletes the objects (see `update_window`).
// The DIB size is a few hundred pixels and the header size a few dozen bytes, so the i32 and u32
// casts never wrap or truncate.
#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
unsafe fn dib_with_pixels(
    pixels: &[u8],
    width: u32,
    height: u32,
) -> anyhow::Result<(HDC, HBITMAP, HGDIOBJ)> {
    let expected = width as usize * height as usize * 4;
    anyhow::ensure!(
        pixels.len() == expected,
        "pixel buffer is {} bytes but the DIB is {width}x{height} ({expected} bytes)",
        pixels.len()
    );
    // SAFETY: the screen DC is released right after the compatible DC is created.
    let screen = unsafe { GetDC(None) };
    // SAFETY: `screen` is a valid DC and the compatible DC is released below.
    let hdc = unsafe { CreateCompatibleDC(Some(screen)) };
    // SAFETY: the screen DC is no longer needed.
    unsafe { ReleaseDC(None, screen) };
    if hdc.is_invalid() {
        anyhow::bail!("CreateCompatibleDC returned a null DC");
    }
    let header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: -(height as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let info = BITMAPINFO {
        bmiHeader: header,
        bmiColors: [RGBQUAD::default(); 1],
    };
    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
    // SAFETY: `info` describes the DIB; `bits` receives the pixel memory.
    let bitmap = unsafe { CreateDIBSection(Some(hdc), &info, DIB_RGB_COLORS, &mut bits, None, 0)? };
    // SAFETY: `bits` is exactly `pixels.len()` bytes (checked above).
    unsafe { std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len()) };
    // SAFETY: selecting the DIB makes it the DC's current bitmap.
    let old = unsafe { SelectObject(hdc, HGDIOBJ(bitmap.0)) };
    Ok((hdc, bitmap, old))
}
