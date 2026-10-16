//! The hidden top-level controller window and the layered child text window (ADR-0016 points 1,
//! 7, 8). Both live on the main thread; Slint's winit event loop dispatches every message of that
//! thread, so there is no second message loop. The UI thread never calls UIA.
//!
//! Soundness rule (ADR-0016): never hold a `&mut Controller` across a Win32 call that can send a
//! message or run a callback. The window procedure borrows the controller with `try_borrow_mut`;
//! re-entrant messages fall through to `DefWindowProcW` (or are re-posted when they carry state),
//! and the app callbacks (`on_event`, `on_scheme`) run only in `deliver`, after the borrow is
//! dropped. Every `borrow_mut` site below says why it cannot re-enter.
// FFI conventions this module cannot avoid: `&x` where `*const T` is expected.
#![allow(clippy::borrow_as_ptr, clippy::ptr_as_ptr)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, ClientToScreen,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HBITMAP,
    HBRUSH, HDC, HGDIOBJ, RGBQUAD, ReleaseDC, SelectObject,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{
    ABE_BOTTOM, ABE_LEFT, ABE_TOP, ABM_GETTASKBARPOS, APPBARDATA, SHAppBarMessage,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CS_HREDRAW, CS_VREDRAW, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DestroyWindow, FindWindowW, GWLP_USERDATA, GetClientRect, GetCursorPos,
    GetWindowLongPtrW, HCURSOR, HICON, MA_NOACTIVATE, MF_STRING, PostMessageW, PostThreadMessageW,
    RegisterClassW, RegisterWindowMessageW, SW_HIDE, SW_SHOWNOACTIVATE, SetForegroundWindow,
    SetWindowLongPtrW, ShowWindow, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx, ULW_ALPHA,
    UpdateLayeredWindow, WINDOW_STYLE, WM_APP, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_LBUTTONUP,
    WM_MOUSEACTIVATE, WM_QUIT, WM_RBUTTONUP, WM_SETTINGCHANGE, WNDCLASSW, WS_CHILD,
    WS_CLIPSIBLINGS, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};
use windows::core::{PCWSTR, w};

use crate::app::taskbar::{Edge, Hidden, Rect, TaskbarLayout, TaskbarLines, place};
use crate::platform::taskbar::{MenuItem, MenuLabels, TaskbarEvent, TaskbarText};
use crate::platform::taskbar_scheme::taskbar_is_dark;

use super::render;
use super::theme;
use super::uia::{self, UiaState};

/// The controller window class (ADR-0016 point 1).
const CONTROLLER_CLASS: PCWSTR = w!("nepdate-taskbar-controller");
/// The text window class.
const TEXT_CLASS: PCWSTR = w!("nepdate-taskbar-text");

/// Controller messages (uia.rs keeps the same numbers).
const WM_APP_RECTS: u32 = WM_APP + 1;
const WM_APP_RESTART: u32 = WM_APP + 3;
/// A `WM_SETTINGCHANGE` that arrived during a borrow, reposted with the setting in `wParam`
/// (`SETTING_COLOR` or `SETTING_TRAY`) instead of the sender's string pointer.
const WM_APP_SETTING: u32 = WM_APP + 4;
const SETTING_COLOR: usize = 1;
const SETTING_TRAY: usize = 2;

/// The `TaskbarCreated` registered message id, read by the window procedure without a borrow.
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

/// What is currently shown, so a tick can skip a redraw when nothing changed (ADR-0016 point 6).
struct Shown {
    lines: TaskbarLines,
    ink: [u8; 4],
    scale: f32,
    /// The text window rectangle in screen coordinates.
    rect: Rect,
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

/// The outcome for the window procedure: either `result` is returned, or the default handler runs.
enum Outcome {
    Return(LRESULT),
    Default,
}

/// Work the window procedure must do after the controller borrow ends.
enum Action {
    None,
    /// Deliver events and, optionally, the new taskbar scheme to the app callbacks.
    Deliver {
        events: Vec<TaskbarEvent>,
        scheme: Option<bool>,
    },
    /// Run the right-click menu (modal) and then deliver the chosen item.
    Menu {
        labels: MenuLabels,
        text_hwnd: HWND,
        point: POINT,
    },
}

struct Controller {
    hwnd: HWND,
    text_hwnd: HWND,
    /// The `RefCell<Controller>` this controller lives in, stored in `GWLP_USERDATA`.
    cell: *const RefCell<Controller>,
    lines: TaskbarLines,
    enabled: bool,
    labels: MenuLabels,
    dark: bool,
    uia: Arc<Mutex<UiaState>>,
    worker_tid: u32,
    /// `Option` so `deliver` can take the callback out, drop the borrow, call it, and put it back.
    on_event: Option<Box<dyn FnMut(TaskbarEvent)>>,
    on_scheme: Option<Box<dyn FnMut(bool)>>,
    shown: Option<Shown>,
    hidden_reason: Option<Hidden>,
}

/// The Windows taskbar text (ADR-0016). `unsafe` stays inside this module and its siblings. The
/// controller lives in an `Rc<RefCell<Controller>>`; `GWLP_USERDATA` points at the `RefCell`, so
/// the window procedure borrows with `try_borrow_mut` instead of fabricating a second `&mut`.
pub struct WindowsTaskbarText {
    inner: Rc<RefCell<Controller>>,
}

impl WindowsTaskbarText {
    pub fn new(
        on_event: impl FnMut(TaskbarEvent) + 'static,
        on_taskbar_scheme: impl FnMut(bool) + 'static,
    ) -> Self {
        let inner = Rc::new(RefCell::new(Controller::new(
            Some(Box::new(on_event)),
            Some(Box::new(on_taskbar_scheme)),
        )));
        let cell = Rc::as_ptr(&inner);
        // These two borrows cannot re-enter: they run before the event loop starts, so no window
        // message and no callback can fire yet.
        inner.borrow_mut().bind(cell);
        inner.borrow_mut().refresh();
        Self { inner }
    }
}

impl TaskbarText for WindowsTaskbarText {
    fn set_lines(&self, lines: &TaskbarLines) {
        // Cannot re-enter: the W1 timer runs between window messages, never while the window
        // procedure holds a borrow, and the refresh under this borrow only sends messages that the
        // window procedure catches with `try_borrow_mut`.
        self.inner.borrow_mut().set_lines(lines);
    }

    fn set_enabled(&self, on: bool) {
        // Cannot re-enter: called once from `main` before the event loop starts.
        self.inner.borrow_mut().set_enabled(on);
    }

    fn set_menu_labels(&self, labels: &MenuLabels) {
        // Cannot re-enter: called once from `main` before the event loop starts.
        self.inner.borrow_mut().set_menu_labels(labels);
    }
}

impl Controller {
    fn new(
        on_event: Option<Box<dyn FnMut(TaskbarEvent)>>,
        on_scheme: Option<Box<dyn FnMut(bool)>>,
    ) -> Controller {
        // Registration failures leave the taskbar text hidden but the tray icon working; the
        // controller window is still created so the failure mode stays "text hidden, one log".
        register_classes();
        let hwnd = create_window(CONTROLLER_CLASS);
        let text_hwnd = create_text_window();
        let uia = Arc::new(Mutex::new(UiaState::default()));
        let worker_tid = match uia::spawn(hwnd, uia.clone()) {
            Ok(tid) => tid,
            Err(error) => {
                log::warn!("taskbar text: UIA worker did not start: {error}");
                // SAFETY: GetCurrentThreadId has no preconditions.
                unsafe { GetCurrentThreadId() }
            }
        };
        // SAFETY: RegisterWindowMessageW only registers a string-to-id mapping.
        let created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        TASKBAR_CREATED.store(created, Ordering::Relaxed);
        Controller {
            hwnd,
            text_hwnd,
            cell: std::ptr::null(),
            lines: TaskbarLines::default(),
            enabled: true,
            labels: MenuLabels::default(),
            dark: taskbar_is_dark().unwrap_or(false),
            uia,
            worker_tid,
            on_event,
            on_scheme,
            shown: None,
            hidden_reason: None,
        }
    }

    /// Stores the `RefCell` pointer in both windows' user data so the window procedure can reach
    /// this controller with `try_borrow_mut`.
    fn bind(&mut self, cell: *const RefCell<Controller>) {
        self.cell = cell;
        // SAFETY: the windows exist and `cell` stays valid for their lifetime; it is cleared in
        // `Controller::drop` before the controller is freed.
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, cell as isize);
            SetWindowLongPtrW(self.text_hwnd, GWLP_USERDATA, cell as isize);
        }
    }

    fn set_lines(&mut self, lines: &TaskbarLines) {
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

    fn set_menu_labels(&mut self, labels: &MenuLabels) {
        self.labels.clone_from(labels);
    }

    /// The single refresh path: re-reads the scheme, DPI and UIA rectangles, re-renders and
    /// re-places only when something changed (ADR-0016 point 6), and hides with one log line
    /// otherwise (point 8).
    // The scale comparison is exact (both values came from the same DPI / 96) and the text size is
    // a few hundred pixels, so the i32 casts never wrap.
    #[allow(clippy::float_cmp, clippy::cast_possible_wrap)]
    fn refresh(&mut self) {
        if !self.enabled {
            self.hide(Hidden::Disabled);
            return;
        }
        let lines = self.lines.clone();
        self.dark = taskbar_is_dark().unwrap_or(false);
        let ink = theme::ink(self.dark);

        let Some((layout, origin)) = self.read_layout() else {
            return;
        };
        let scale = layout.scale;

        // Re-render only when the lines, ink or scale changed.
        let needs_render = self
            .shown
            .as_ref()
            .is_none_or(|s| s.lines != lines || s.ink != ink || s.scale != scale);
        if needs_render {
            match render::render(&lines, scale, ink) {
                Ok(rendered) => {
                    self.shown = Some(Shown {
                        lines: lines.clone(),
                        ink,
                        scale,
                        rect: Rect {
                            left: 0,
                            top: 0,
                            right: 0,
                            bottom: 0,
                        },
                        pixels: rendered.pixels,
                        width: rendered.width,
                        height: rendered.height,
                    });
                }
                Err(error) => {
                    log::warn!("taskbar text: render failed: {error}");
                    self.hide(Hidden::NoRoom);
                    return;
                }
            }
        }

        let Some(shown) = self.shown.as_mut() else {
            return;
        };
        let rect_client = match place(&layout, (shown.width as i32, shown.height as i32)) {
            Ok(rect) => rect,
            Err(reason) => {
                self.hide(reason);
                return;
            }
        };
        let rect_screen = Rect {
            left: rect_client.left + origin.x,
            top: rect_client.top + origin.y,
            right: rect_client.right + origin.x,
            bottom: rect_client.bottom + origin.y,
        };
        if shown.rect != rect_screen {
            shown.rect = rect_screen;
            let hwnd = self.text_hwnd;
            // `UpdateLayeredWindow` positions a child window in its parent's client coordinates,
            // so pass `rect_client`, not the screen rectangle stored for the popup anchor.
            if let Err(error) =
                update_window(hwnd, &shown.pixels, shown.width, shown.height, rect_client)
            {
                log::warn!("taskbar text: UpdateLayeredWindow failed: {error}");
                self.hide(Hidden::NoRoom);
                return;
            }
        }
        // SAFETY: ShowWindow is safe on the window we created. It may send messages synchronously;
        // the window procedure catches them with `try_borrow_mut` and falls through.
        unsafe {
            let _ = ShowWindow(self.text_hwnd, SW_SHOWNOACTIVATE);
        }
        if self.hidden_reason.take().is_some() {
            log::info!("taskbar text shown");
        }
    }

    /// Reads the taskbar geometry, DPI and UIA rectangles into a layout, hiding the text on
    /// failure. Returns the layout and the taskbar's client origin in screen coordinates.
    // The DPI is a small u32 (96, 144, 192, ...), so the f32 cast only loses integer precision far
    // below any screen coordinate range.
    #[allow(clippy::cast_precision_loss)]
    fn read_layout(&mut self) -> Option<(TaskbarLayout, POINT)> {
        // SAFETY: FindWindowW and SHAppBarMessage are read-only queries; a missing taskbar is a
        // normal Windows 10 or Explorer-restart state, handled below.
        let Ok(taskbar) = (unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }) else {
            self.hide(Hidden::NoStartButton);
            return None;
        };
        let edge = taskbar_edge(taskbar);
        // SAFETY: GetDpiForWindow is a read-only query.
        let scale = unsafe { GetDpiForWindow(taskbar) } as f32 / 96.0;
        let Some(origin) = client_origin(taskbar) else {
            self.hide(Hidden::NotBottom);
            return None;
        };
        let Some(client) = client_rect(taskbar) else {
            self.hide(Hidden::NotBottom);
            return None;
        };
        let uia = self
            .uia
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let to_client = |r: Rect| Rect {
            left: r.left - origin.x,
            top: r.top - origin.y,
            right: r.right - origin.x,
            bottom: r.bottom - origin.y,
        };
        let layout = TaskbarLayout {
            taskbar: Rect {
                left: 0,
                top: 0,
                right: client.right,
                bottom: client.bottom,
            },
            edge,
            start: uia.start.map(to_client),
            widgets: uia.widgets.map(to_client),
            scale,
        };
        drop(uia);
        Some((layout, origin))
    }

    fn hide(&mut self, reason: Hidden) {
        if self.hidden_reason != Some(reason) {
            log::info!("taskbar text hidden: {reason:?}");
            self.hidden_reason = Some(reason);
        }
        // SAFETY: ShowWindow is safe on the window we created.
        unsafe {
            let _ = ShowWindow(self.text_hwnd, SW_HIDE);
        }
        // Keep the rendered bitmap and `shown` so a hidden refresh never re-renders; the cache is
        // only cleared when lines, ink or scale change (W1B-05 item 3).
    }

    /// Recreates the text window after Explorer restarted (`TaskbarCreated`).
    fn recreate_text_window(&mut self) {
        // SAFETY: destroying and recreating our own windows is safe.
        unsafe {
            if !self.text_hwnd.0.is_null() {
                let _ = DestroyWindow(self.text_hwnd);
            }
        }
        self.text_hwnd = create_text_window();
        // SAFETY: the new window gets the same `RefCell` pointer.
        unsafe { SetWindowLongPtrW(self.text_hwnd, GWLP_USERDATA, self.cell as isize) };
        self.shown = None;
    }

    /// The window procedure body, run under a `try_borrow_mut`. Returns the LRESULT decision and
    /// the work to do after the borrow ends. It never calls the app callbacks.
    // MA_NOACTIVATE is a small u32; the isize cast never wraps on either 32- or 64-bit targets.
    #[allow(clippy::cast_possible_wrap)]
    fn handle(
        &mut self,
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> (Outcome, Action) {
        if hwnd == self.text_hwnd {
            return match msg {
                WM_LBUTTONUP => {
                    let anchor = self.shown.as_ref().map_or(
                        Rect {
                            left: 0,
                            top: 0,
                            right: 0,
                            bottom: 0,
                        },
                        |s| s.rect,
                    );
                    (
                        Outcome::Return(LRESULT(0)),
                        Action::Deliver {
                            events: vec![TaskbarEvent::Toggle { anchor }],
                            scheme: None,
                        },
                    )
                }
                WM_RBUTTONUP => {
                    let mut point = POINT { x: 0, y: 0 };
                    // SAFETY: `point` is a valid out value; GetCursorPos sends nothing.
                    unsafe {
                        let _ = GetCursorPos(&mut point);
                    }
                    (
                        Outcome::Return(LRESULT(0)),
                        Action::Menu {
                            labels: self.labels.clone(),
                            text_hwnd: self.text_hwnd,
                            point,
                        },
                    )
                }
                WM_MOUSEACTIVATE => (
                    Outcome::Return(LRESULT(MA_NOACTIVATE as isize)),
                    Action::None,
                ),
                _ => (Outcome::Default, Action::None),
            };
        }
        if hwnd == self.hwnd {
            match msg {
                WM_APP_RECTS | WM_DISPLAYCHANGE | WM_DPICHANGED => {
                    self.refresh();
                    return (Outcome::Return(LRESULT(0)), Action::None);
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
                        Some(SETTING_TRAY) => {
                            self.refresh();
                            Action::None
                        }
                        _ => Action::None,
                    };
                    return (Outcome::Return(LRESULT(0)), action);
                }
                _ if msg == TASKBAR_CREATED.load(Ordering::Relaxed) && msg != 0 => {
                    log::info!("taskbar text: Explorer restarted, recreating the text window");
                    self.recreate_text_window();
                    // SAFETY: posting to our worker thread is always safe.
                    unsafe {
                        let _ = PostThreadMessageW(
                            self.worker_tid,
                            WM_APP_RESTART,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                    self.refresh();
                    return (Outcome::Return(LRESULT(0)), Action::None);
                }
                _ => {}
            }
        }
        (Outcome::Default, Action::None)
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        // SAFETY: clear the user data first so a late message never derefs this freed controller,
        // then destroy the windows, then signal the worker (never joined, so a hung Explorer UIA
        // call cannot block shutdown).
        unsafe {
            if !self.hwnd.0.is_null() {
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            }
            if !self.text_hwnd.0.is_null() {
                SetWindowLongPtrW(self.text_hwnd, GWLP_USERDATA, 0);
            }
            if !self.text_hwnd.0.is_null() {
                let _ = DestroyWindow(self.text_hwnd);
            }
            if !self.hwnd.0.is_null() {
                let _ = DestroyWindow(self.hwnd);
            }
            let _ = PostThreadMessageW(self.worker_tid, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

/// Runs the app callbacks with no controller borrow held. `Option::take` moves the callbacks out
/// of the controller first, so a callback that calls back into the taskbar text (for example
/// `set_lines`) borrows cleanly instead of aliasing.
fn deliver(cell: &RefCell<Controller>, events: Vec<TaskbarEvent>, scheme: Option<bool>) {
    if events.is_empty() && scheme.is_none() {
        return;
    }
    let (mut on_event, mut on_scheme) = match cell.try_borrow_mut() {
        Ok(mut controller) => (controller.on_event.take(), controller.on_scheme.take()),
        Err(_) => return,
    };
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

/// Builds and runs the right-click menu (WR-01 AC5 items), with no controller borrow held so the
/// modal loop can dispatch the timer and other windows. Returns the chosen item.
// The command id is 0..=5, so the BOOL-to-u32 cast never sees a negative value.
#[allow(clippy::cast_sign_loss)]
unsafe fn run_menu(labels: &MenuLabels, text_hwnd: HWND, point: POINT) -> Option<MenuItem> {
    // SAFETY: the popup menu is ours and the labels are UTF-16 buffers we own.
    let menu = unsafe { CreatePopupMenu() }.ok()?;
    let items = [
        (MenuItem::Open, &labels.open),
        (MenuItem::CopyBs, &labels.copy_bs),
        (MenuItem::CopyAd, &labels.copy_ad),
        (MenuItem::Settings, &labels.settings),
        (MenuItem::Quit, &labels.quit),
    ];
    for (index, (_, label)) in items.iter().enumerate() {
        let text: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: the menu and the text buffer are valid for this call.
        let _ = unsafe { AppendMenuW(menu, MF_STRING, index + 1, PCWSTR(text.as_ptr())) };
    }
    // The text window never activates, so claim the foreground so the menu dismisses on a click
    // elsewhere (WR-01 AC5 behaviour).
    // SAFETY: SetForegroundWindow is safe on our window.
    let _ = unsafe { SetForegroundWindow(text_hwnd) };
    // SAFETY: TrackPopupMenuEx blocks and dispatches messages; nothing borrows the controller
    // here, so re-entrant messages (the timer, other windows) are handled normally.
    let command = unsafe {
        TrackPopupMenuEx(
            menu,
            TPM_RETURNCMD.0 | TPM_RIGHTBUTTON.0,
            point.x,
            point.y,
            text_hwnd,
            None,
        )
    };
    // SAFETY: the menu is ours.
    let _ = unsafe { DestroyMenu(menu) };
    match command.0 as u32 {
        1 => Some(MenuItem::Open),
        2 => Some(MenuItem::CopyBs),
        3 => Some(MenuItem::CopyAd),
        4 => Some(MenuItem::Settings),
        5 => Some(MenuItem::Quit),
        _ => None,
    }
}

/// The window procedure for both windows. The `RefCell<Controller>` pointer lives in
/// `GWLP_USERDATA`.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: the pointer was stored in `bind` and cleared in `Controller::drop`; a null pointer
    // means the message arrived before `bind` or after drop.
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
                text_hwnd,
                point,
            } => {
                // SAFETY: the menu runs with no borrow held; see `run_menu`.
                if let Some(item) = unsafe { run_menu(&labels, text_hwnd, point) } {
                    deliver(cell, vec![TaskbarEvent::Menu(item)], None);
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
        // `ShowWindow`/`UpdateLayeredWindow`/`DestroyWindow`, or a callback). Repost the
        // messages that carry state so nothing is lost; drop the rest. `WM_SETTINGCHANGE` is
        // never reposted as is: its `lParam` string belongs to the sender and is freed when the
        // send returns, so read it now and post only which setting changed.
        let repost = match msg {
            WM_APP_RECTS | WM_DISPLAYCHANGE | WM_DPICHANGED => Some((WM_APP_RECTS, WPARAM(0))),
            WM_SETTINGCHANGE => setting_code(lparam).map(|code| (WM_APP_SETTING, WPARAM(code))),
            _ if msg != 0 && msg == TASKBAR_CREATED.load(Ordering::Relaxed) => {
                Some((msg, WPARAM(0)))
            }
            _ => None,
        };
        if let Some((msg, wparam)) = repost {
            // SAFETY: posting to our own window is always safe; no pointer travels with it.
            unsafe {
                let _ = PostMessageW(Some(hwnd), msg, wparam, LPARAM(0));
            }
        }
        // SAFETY: the default handler is safe for any message.
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }
}

/// Registers the two window classes. Failure is logged once and left to the caller's fallback.
fn register_classes() {
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
            log::warn!("taskbar text: RegisterClassW (controller) failed");
        }
        let text = WNDCLASSW {
            lpszClassName: TEXT_CLASS,
            ..controller
        };
        if RegisterClassW(&text) == 0 {
            log::warn!("taskbar text: RegisterClassW (text) failed");
        }
    }
}

/// Creates the hidden controller window.
fn create_window(class: PCWSTR) -> HWND {
    // SAFETY: the class name is static; the window is hidden and never shown.
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class,
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

/// Creates the layered child text window, parented to `Shell_TrayWnd` (ADR-0016 point 1).
fn create_text_window() -> HWND {
    // SAFETY: FindWindowW is a read-only query.
    let parent =
        unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.unwrap_or(HWND(std::ptr::null_mut()));
    // SAFETY: the class name is static; the parent is the taskbar or null (hidden until it works).
    unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            TEXT_CLASS,
            w!(""),
            WS_CHILD | WS_CLIPSIBLINGS,
            0,
            0,
            0,
            0,
            Some(parent),
            None,
            Some(HINSTANCE(std::ptr::null_mut())),
            None,
        )
        .unwrap_or(HWND(std::ptr::null_mut()))
    }
}

/// The taskbar's docked edge via `SHAppBarMessage` (ADR-0016 point 2).
// The struct size is a few hundred bytes, so the usize-to-u32 cast never truncates.
#[allow(clippy::cast_possible_truncation)]
fn taskbar_edge(taskbar: HWND) -> Edge {
    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: taskbar,
        ..Default::default()
    };
    // SAFETY: `data` is a valid APPBARDATA with cbSize set.
    let result = unsafe { SHAppBarMessage(ABM_GETTASKBARPOS, &mut data) };
    if result == 0 {
        return Edge::Bottom;
    }
    match data.uEdge {
        ABE_BOTTOM => Edge::Bottom,
        ABE_TOP => Edge::Top,
        ABE_LEFT => Edge::Left,
        _ => Edge::Right,
    }
}

/// The taskbar's client origin in screen coordinates, or `None` on failure.
fn client_origin(hwnd: HWND) -> Option<POINT> {
    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: `point` is a valid out value.
    if !unsafe { ClientToScreen(hwnd, &mut point) }.as_bool() {
        return None;
    }
    Some(point)
}

/// The taskbar's client rectangle, or `None` on failure.
fn client_rect(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    // SAFETY: `rect` is a valid out value.
    unsafe { GetClientRect(hwnd, &mut rect) }.ok()?;
    Some(rect)
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

/// Pushes the rendered pixels to the layered window with `UpdateLayeredWindow` (ADR-0016 point 4).
// The text size is a few hundred pixels and the blend flags are the two small AC_SRC_* constants,
// so the i32 and u8 casts never wrap or truncate.
#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
fn update_window(
    hwnd: HWND,
    pixels: &[u8],
    width: u32,
    height: u32,
    rect: Rect,
) -> anyhow::Result<()> {
    // SAFETY: the DIB and DC are created and released within this function.
    let (hdc, bitmap, old) = unsafe { dib_with_pixels(pixels, width, height) }?;
    let dst = POINT {
        x: rect.left,
        y: rect.top,
    };
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
