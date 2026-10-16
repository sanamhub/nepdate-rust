//! UI Automation on a worker thread (ADR-0016 point 3). Explorer can hang, so no UIA call runs on
//! the UI thread. This thread owns every UIA object, finds the Start and Widgets buttons under
//! `Shell_TrayWnd`, subscribes to the Start button's `BoundingRectangle` changes and posts the
//! rectangles to the controller window with `PostMessageW`. The UI thread never calls UIA.
// FFI conventions this module cannot avoid: `&x` where `*const T` is expected, the `#[implement]`
// macro's generated code, and the worker-thread plumbing below.
#![allow(
    clippy::borrow_as_ptr,
    clippy::ptr_as_ptr,
    clippy::ref_as_ptr,
    clippy::needless_pass_by_value,
    clippy::single_match,
    clippy::inline_always
)]

use std::sync::{Arc, Mutex, mpsc};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationPropertyChangedEventHandler,
    IUIAutomationPropertyChangedEventHandler_Impl, TreeScope_Descendants, TreeScope_Element,
    UIA_AutomationIdPropertyId, UIA_BoundingRectanglePropertyId, UIA_PROPERTY_ID,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, FindWindowW, GetMessageW, MSG, PostMessageW, TranslateMessage, WM_APP,
};
use windows::core::Interface as _;
use windows::core::{implement, w};

use crate::app::taskbar::Rect;

/// Controller messages (controller.rs keeps the same numbers).
pub const WM_APP_RECTS: u32 = WM_APP + 1;
pub const WM_APP_RESTART: u32 = WM_APP + 3;

/// The Start button's `AutomationId`, found by a UIA tree dump on this machine (Windows 11 build
/// 26300): `AutomationId = "StartButton"`, class `ToggleButton`, name `Start`.
pub const START_ID: &str = "StartButton";
/// The Widgets button's `AutomationId`. Widgets is off on this machine (`TaskbarDa` = 0), so this
/// id is unconfirmed; an absent Widgets button means "left bound = `taskbar.left`".
pub const WIDGETS_ID: &str = "WidgetsButton";

/// What the worker last posted, in physical screen coordinates. The controller reads it and the
/// handler uses it as the "last posted" value so it posts only on a change.
#[derive(Debug, Clone, Copy, Default)]
pub struct UiaState {
    pub start: Option<Rect>,
    pub widgets: Option<Rect>,
}

/// Spawns the worker thread and returns its OS thread id, so the controller can post restart and
/// quit messages to it.
pub fn spawn(controller: HWND, state: Arc<Mutex<UiaState>>) -> anyhow::Result<u32> {
    let (tx, rx) = mpsc::channel();
    // The raw HWND pointer is not `Send`; carry its bits and rebuild it on the worker thread.
    let controller_bits = controller.0 as usize;
    std::thread::Builder::new()
        .name("nepdate-taskbar-uia".into())
        .spawn(move || {
            // SAFETY: GetCurrentThreadId has no preconditions.
            let tid = unsafe { GetCurrentThreadId() };
            let _ = tx.send(tid);
            run(HWND(controller_bits as *mut std::ffi::c_void), state);
        })?;
    Ok(rx.recv()?)
}

fn run(controller: HWND, state: Arc<Mutex<UiaState>>) {
    // SAFETY: COM is initialized for this thread only, and uninitialized on every exit path.
    unsafe {
        if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
            return;
        }
    }
    // SAFETY: CUIAutomation is a documented COM class; None means no outer unknown.
    let Ok(automation) = (unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) })
    else {
        // SAFETY: balancing the CoInitializeEx above.
        unsafe { CoUninitialize() };
        return;
    };

    loop {
        let subscription = subscribe(&automation, controller, &state);
        // Pump until the controller posts a restart or quit message. Dropping the subscription
        // removes the event handler.
        let restart = pump();
        drop(subscription);
        if !restart {
            break;
        }
    }
    // SAFETY: balancing the CoInitializeEx above.
    unsafe { CoUninitialize() };
}

/// Finds the taskbar, Start and Widgets buttons, reports their rectangles and subscribes to the
/// Start button's `BoundingRectangle` change. Returns the subscription, which removes the handler
/// when dropped. Re-finding happens here, on the initial run and on every `WM_APP_RESTART`.
fn subscribe(
    automation: &IUIAutomation,
    controller: HWND,
    state: &Arc<Mutex<UiaState>>,
) -> Option<Subscription> {
    // SAFETY: FindWindowW is a read-only query; a null HWND is handled below.
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok()?;
    // SAFETY: ElementFromHandle is a read-only query on the taskbar window.
    let root = unsafe { automation.ElementFromHandle(taskbar) }.ok()?;
    let start = find(automation, &root, START_ID).ok().flatten()?;
    let widgets = find(automation, &root, WIDGETS_ID).ok().flatten();
    // Initial report with the kept elements; posts only if the state changed.
    report(&start, widgets.as_ref(), controller, state);
    let handler: IUIAutomationPropertyChangedEventHandler = Handler {
        controller,
        start: start.clone(),
        widgets: widgets.clone(),
        state: state.clone(),
    }
    .into();
    // SAFETY: the handler outlives the subscription; the element is the Start button.
    unsafe {
        automation
            .AddPropertyChangedEventHandlerNativeArray(
                &start,
                TreeScope_Element,
                None,
                &handler,
                &[UIA_BoundingRectanglePropertyId],
            )
            .ok()?;
    }
    Some(Subscription {
        automation: automation.clone(),
        start,
        handler,
    })
}

/// Keeps the automation object and removes the handler when dropped, so a restart does not stack
/// handlers (W1B-05 item 3).
struct Subscription {
    automation: IUIAutomation,
    start: IUIAutomationElement,
    handler: IUIAutomationPropertyChangedEventHandler,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        // SAFETY: removing our own handler is safe; it is best-effort because UIA also releases
        // the handler when the Start button is destroyed (Explorer restart).
        unsafe {
            let _ = self
                .automation
                .RemovePropertyChangedEventHandler(&self.start, &self.handler);
        }
    }
}

/// Reads the kept elements' bounding rectangles and posts to the controller only when they
/// changed from the last posted values. Runs only on the worker thread.
fn report(
    start: &IUIAutomationElement,
    widgets: Option<&IUIAutomationElement>,
    controller: HWND,
    state: &Mutex<UiaState>,
) {
    let start_rect = bounding_rect(start);
    let widgets_rect = widgets.and_then(bounding_rect);
    let mut shared = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if shared.start == start_rect && shared.widgets == widgets_rect {
        return; // unchanged: do not wake the controller
    }
    shared.start = start_rect;
    shared.widgets = widgets_rect;
    drop(shared);
    // SAFETY: the controller window is valid; a posted message is always safe.
    let _ = unsafe { PostMessageW(Some(controller), WM_APP_RECTS, WPARAM(0), LPARAM(0)) };
}

/// The element's bounding rectangle in physical screen coordinates.
fn bounding_rect(element: &IUIAutomationElement) -> Option<Rect> {
    // SAFETY: CurrentBoundingRectangle is a read-only query on the element.
    let r = unsafe { element.CurrentBoundingRectangle() }.ok()?;
    Some(Rect {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    })
}

/// Finds one descendant of `root` by `AutomationId`, or `None`.
fn find(
    automation: &IUIAutomation,
    root: &IUIAutomationElement,
    id: &str,
) -> windows::core::Result<Option<IUIAutomationElement>> {
    // SAFETY: the property id and the BSTR-backed VARIANT are valid for the call.
    let condition = unsafe {
        automation.CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from(id))?
    };
    // SAFETY: the condition and root outlive the call.
    let element = unsafe { root.FindFirst(TreeScope_Descendants, &condition)? };
    Ok(if element.as_raw().is_null() {
        None
    } else {
        Some(element)
    })
}

/// The property-changed handler: reads the kept elements' rectangles and posts only on a change.
/// It holds no `IUIAutomation`, so there is no reference cycle with the subscription.
#[implement(IUIAutomationPropertyChangedEventHandler)]
struct Handler {
    controller: HWND,
    start: IUIAutomationElement,
    widgets: Option<IUIAutomationElement>,
    state: Arc<Mutex<UiaState>>,
}

impl IUIAutomationPropertyChangedEventHandler_Impl for Handler_Impl {
    fn HandlePropertyChangedEvent(
        &self,
        _sender: windows::core::Ref<IUIAutomationElement>,
        _propertyid: UIA_PROPERTY_ID,
        _newvalue: &VARIANT,
    ) -> windows::core::Result<()> {
        report(
            &self.start,
            self.widgets.as_ref(),
            self.controller,
            &self.state,
        );
        Ok(())
    }
}

/// Pumps the worker thread's message queue until the controller posts a restart or quit message.
/// Returns `true` for a restart (re-find and re-subscribe), `false` for quit.
fn pump() -> bool {
    let mut msg = MSG::default();
    loop {
        // SAFETY: `msg` is a valid out struct; GetMessageW blocks until a message arrives.
        let got = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if got.0 == 0 || got.0 == -1 {
            return false; // WM_QUIT or an error
        }
        if msg.message == WM_APP_RESTART {
            return true;
        }
        // SAFETY: the message is a valid thread message.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
