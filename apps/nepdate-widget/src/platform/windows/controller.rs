//! The hidden top-level controller window and the layered child text window (ADR-0016 points 1,
//! 7, 8). Both live on the main thread; Slint's winit event loop dispatches every message of that
//! thread, so there is no second message loop. The UI thread never calls UIA.
// FFI conventions this module cannot avoid: `&x` where `*const T` is expected, and the text, DPI
// and bitmap dimensions, which are all a few hundred pixels or bytes at most.
#![allow(
    clippy::borrow_as_ptr,
    clippy::ptr_as_ptr,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::struct_field_names,
    clippy::unnecessary_cast,
    clippy::needless_pass_by_value,
    clippy::if_not_else,
    clippy::single_match
)]

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, ClientToScreen,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HBITMAP,
    HBRUSH, HDC, HGDIOBJ, RGBQUAD, ReleaseDC, SelectObject,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{ABE_BOTTOM, ABM_GETTASKBARPOS, APPBARDATA, SHAppBarMessage};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CS_HREDRAW, CS_VREDRAW, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DestroyWindow, FindWindowW, GWLP_USERDATA, GetClientRect, GetCursorPos,
    GetWindowLongPtrW, HCURSOR, HICON, MA_NOACTIVATE, MF_STRING, PostThreadMessageW,
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
const CONTROLLER_CLASS: windows::core::PCWSTR = w!("nepdate-taskbar-controller");
/// The text window class.
const TEXT_CLASS: windows::core::PCWSTR = w!("nepdate-taskbar-text");

/// Controller messages (uia.rs keeps the same numbers).
const WM_APP_RECTS: u32 = WM_APP + 1;
const WM_APP_RESTART: u32 = WM_APP + 3;

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

struct Controller {
    controller_hwnd: HWND,
    text_hwnd: HWND,
    lines: TaskbarLines,
    enabled: bool,
    labels: MenuLabels,
    dark: bool,
    uia: Arc<Mutex<UiaState>>,
    worker_tid: u32,
    taskbar_created_msg: u32,
    on_event: Box<dyn FnMut(TaskbarEvent)>,
    on_scheme: Box<dyn FnMut(bool)>,
    shown: Option<Shown>,
    hidden_reason: Option<Hidden>,
}

/// The Windows taskbar text (ADR-0016). `unsafe` stays inside this module and its siblings.
/// `RefCell` is the interior mutability the `&self` trait needs; every access is on the main
/// thread, and the window procedure holds the only raw `&mut` while a message is dispatched.
pub struct WindowsTaskbarText {
    inner: RefCell<Controller>,
}

impl WindowsTaskbarText {
    pub fn new(
        on_event: impl FnMut(TaskbarEvent) + 'static,
        on_taskbar_scheme: impl FnMut(bool) + 'static,
    ) -> Self {
        let mut inner = RefCell::new(Controller::new(
            Box::new(on_event),
            Box::new(on_taskbar_scheme),
        ));
        let ptr = inner.as_ptr();
        inner.get_mut().bind(ptr);
        inner.get_mut().refresh();
        Self { inner }
    }
}

impl TaskbarText for WindowsTaskbarText {
    fn set_lines(&self, lines: &TaskbarLines) {
        self.inner.borrow_mut().set_lines(lines);
    }

    fn set_enabled(&self, on: bool) {
        self.inner.borrow_mut().set_enabled(on);
    }

    fn set_menu_labels(&self, labels: &MenuLabels) {
        self.inner.borrow_mut().set_menu_labels(labels);
    }
}

impl Controller {
    fn new(on_event: Box<dyn FnMut(TaskbarEvent)>, on_scheme: Box<dyn FnMut(bool)>) -> Controller {
        // Registration failures leave the taskbar text hidden but the tray icon working; the
        // controller window is still created so the failure mode stays "text hidden, one log".
        register_classes();
        let controller_hwnd = create_window(CONTROLLER_CLASS, None);
        let text_hwnd = create_text_window();
        let uia = Arc::new(Mutex::new(UiaState::default()));
        let worker_tid = match uia::spawn(controller_hwnd, uia.clone()) {
            Ok(tid) => tid,
            Err(error) => {
                log::warn!("taskbar text: UIA worker did not start: {error}");
                // SAFETY: GetCurrentThreadId has no preconditions.
                unsafe { GetCurrentThreadId() }
            }
        };
        // SAFETY: RegisterWindowMessageW only registers a string-to-id mapping.
        let taskbar_created_msg = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        Controller {
            controller_hwnd,
            text_hwnd,
            lines: TaskbarLines::default(),
            enabled: true,
            labels: MenuLabels::default(),
            dark: taskbar_is_dark().unwrap_or(false),
            uia,
            worker_tid,
            taskbar_created_msg,
            on_event,
            on_scheme,
            shown: None,
            hidden_reason: None,
        }
    }

    /// Stores `ptr` in both windows' user data so the window procedure can reach this controller.
    fn bind(&mut self, ptr: *mut Controller) {
        // SAFETY: the windows exist and `ptr` stays valid for their lifetime.
        unsafe {
            SetWindowLongPtrW(self.controller_hwnd, GWLP_USERDATA, ptr as isize);
            SetWindowLongPtrW(self.text_hwnd, GWLP_USERDATA, ptr as isize);
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
    fn refresh(&mut self) {
        if !self.enabled {
            self.hide(Hidden::Disabled);
            return;
        }
        let lines = self.lines.clone();
        self.dark = taskbar_is_dark().unwrap_or(false);
        let ink = theme::ink(self.dark);

        // SAFETY: FindWindowW and SHAppBarMessage are read-only queries; a missing taskbar is a
        // normal Windows 10 or Explorer-restart state, handled below.
        let Ok(taskbar) = (unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }) else {
            self.hide(Hidden::NoStartButton);
            return;
        };
        let edge = taskbar_edge(taskbar);
        // SAFETY: GetDpiForWindow is a read-only query.
        let scale = unsafe { GetDpiForWindow(taskbar) } as f32 / 96.0;
        let Some(origin) = client_origin(taskbar) else {
            self.hide(Hidden::NotBottom);
            return;
        };
        let Some(client) = client_rect(taskbar) else {
            self.hide(Hidden::NotBottom);
            return;
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
            if let Err(error) =
                update_window(hwnd, &shown.pixels, shown.width, shown.height, rect_screen)
            {
                log::warn!("taskbar text: UpdateLayeredWindow failed: {error}");
                self.hide(Hidden::NoRoom);
                return;
            }
        }
        // SAFETY: ShowWindow is safe on the window we created.
        unsafe {
            let _ = ShowWindow(self.text_hwnd, SW_SHOWNOACTIVATE);
        }
        if self.hidden_reason.take().is_some() {
            log::info!("taskbar text shown");
        }
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
        self.shown = None;
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
        let ptr: *mut Controller = self;
        // SAFETY: the new window gets the same stable pointer.
        unsafe { SetWindowLongPtrW(self.text_hwnd, GWLP_USERDATA, ptr as isize) };
        self.shown = None;
    }

    /// The window procedure body. `hwnd` tells which of the two windows the message is for.
    unsafe fn handle(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if hwnd == self.text_hwnd {
            match msg {
                WM_LBUTTONUP => {
                    self.on_left_click();
                    return LRESULT(0);
                }
                WM_RBUTTONUP => {
                    self.on_right_click();
                    return LRESULT(0);
                }
                WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
                _ => {}
            }
        } else if hwnd == self.controller_hwnd {
            match msg {
                WM_APP_RECTS | WM_DISPLAYCHANGE | WM_DPICHANGED => {
                    self.refresh();
                    return LRESULT(0);
                }
                WM_SETTINGCHANGE => {
                    if let Some(name) = setting_name(lparam) {
                        if name == "ImmersiveColorSet" {
                            self.dark = taskbar_is_dark().unwrap_or(false);
                            (self.on_scheme)(self.dark);
                            self.refresh();
                        } else if name == "TraySettings" {
                            self.refresh();
                        }
                    }
                    return LRESULT(0);
                }
                _ if msg == self.taskbar_created_msg && msg != 0 => {
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
                    return LRESULT(0);
                }
                _ => {}
            }
        }
        // SAFETY: DefWindowProcW is the documented default for unhandled messages.
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    fn on_left_click(&mut self) {
        let anchor = self.shown.as_ref().map_or(
            Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            |s| s.rect,
        );
        (self.on_event)(TaskbarEvent::Toggle { anchor });
    }

    fn on_right_click(&mut self) {
        // SAFETY: the popup menu, cursor query and TrackPopupMenuEx are all read-only; the labels
        // are our own UTF-16 buffers.
        unsafe {
            let Ok(menu) = CreatePopupMenu() else { return };
            let labels = [
                &self.labels.open,
                &self.labels.copy_bs,
                &self.labels.copy_ad,
                &self.labels.settings,
                &self.labels.quit,
            ];
            for (index, label) in labels.iter().enumerate() {
                let text: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                let _ = AppendMenuW(menu, MF_STRING, index + 1, PCWSTR(text.as_ptr()));
            }
            let mut point = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut point).is_err() {
                let _ = DestroyMenu(menu);
                return;
            }
            // The text window never activates, so claim the foreground so the menu dismisses on
            // a click elsewhere (WR-01 AC5 behaviour).
            let _ = SetForegroundWindow(self.text_hwnd);
            let command = TrackPopupMenuEx(
                menu,
                TPM_RETURNCMD.0 | TPM_RIGHTBUTTON.0,
                point.x,
                point.y,
                self.text_hwnd,
                None,
            );
            let _ = DestroyMenu(menu);
            let item = match command.0 as u32 {
                1 => MenuItem::Open,
                2 => MenuItem::CopyBs,
                3 => MenuItem::CopyAd,
                4 => MenuItem::Settings,
                5 => MenuItem::Quit,
                _ => return,
            };
            (self.on_event)(TaskbarEvent::Menu(item));
        }
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        // SAFETY: the windows are ours; the worker is signalled to quit, never joined, so a
        // hung Explorer UIA call cannot block app shutdown.
        unsafe {
            if !self.text_hwnd.0.is_null() {
                let _ = DestroyWindow(self.text_hwnd);
            }
            if !self.controller_hwnd.0.is_null() {
                let _ = DestroyWindow(self.controller_hwnd);
            }
            let _ = PostThreadMessageW(self.worker_tid, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

/// The window procedure for both windows. The controller pointer lives in `GWLP_USERDATA`.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: the pointer was stored in `bind` and is valid for the window's lifetime.
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut Controller;
    if ptr.is_null() {
        // SAFETY: before `bind` runs (WM_CREATE and friends), fall through to the default.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // SAFETY: `ptr` points at the boxed Controller, never moved.
    unsafe { (&mut *ptr).handle(hwnd, msg, wparam, lparam) }
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
            lpszMenuName: windows::core::PCWSTR(std::ptr::null()),
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
fn create_window(class: PCWSTR, _parent: Option<HWND>) -> HWND {
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
        0 => Edge::Top,
        1 => Edge::Left,
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

/// The `WM_SETTINGCHANGE` `lParam` string (`ImmersiveColorSet`, `TraySettings`), or `None`.
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
unsafe fn dib_with_pixels(
    pixels: &[u8],
    width: u32,
    height: u32,
) -> anyhow::Result<(HDC, HBITMAP, HGDIOBJ)> {
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
    // SAFETY: `bits` is exactly `pixels.len()` bytes.
    unsafe { std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len()) };
    // SAFETY: selecting the DIB makes it the DC's current bitmap.
    let old = unsafe { SelectObject(hdc, HGDIOBJ(bitmap.0)) };
    Ok((hdc, bitmap, old))
}
