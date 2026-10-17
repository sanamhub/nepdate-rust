//! Maps `app` view models to Slint generated types and callbacks (ADR-0015). The only place that
//! converts between the two. W1 adds the popup placement (W1-04); W1B wires `PopupAnchor::Above`
//! for the taskbar text (W1B-03).

use slint::ComponentHandle;

use crate::app::clock::Header;
use crate::platform::taskbar::MenuLabels;
use crate::{AppInfo, MenuText, PopupClockLine, PopupWindow};

/// Where to open the popup (W1-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupAnchor {
    /// The fixed corner of the primary monitor per OS (WR-02 AC1).
    TrayCorner,
    /// Above a rectangle, left-aligned, clamped to the monitor work area (used by W1B).
    Above(Rect),
}

/// A rectangle in physical pixels (W1B's `app::taskbar::Rect` matches this shape).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Positions and shows the popup at `anchor`, then gives the inner `FocusScope` focus so Escape
/// and focus-loss hiding work (WR-02 AC2).
pub fn show_popup_at(popup: &PopupWindow, anchor: PopupAnchor) -> anyhow::Result<()> {
    match anchor {
        PopupAnchor::TrayCorner => position_tray_corner(popup),
        PopupAnchor::Above(anchor) => position_above(popup, anchor),
    }
    popup.show()?;
    popup.invoke_focus();
    Ok(())
}

/// Sets the popup's header and clock lines from the app clock model (WR-02 AC3).
pub fn set_header(popup: &PopupWindow, header: &Header) {
    popup.set_bs(header.bs.as_str().into());
    popup.set_weekday(header.weekday.as_str().into());
    popup.set_ad(header.ad.as_str().into());
}

/// Sets the popup's clock lines. The local line's label is `@tr("Local")` in the markup; the
/// extra zone's is `{city} {offset}`.
pub fn set_clocks(popup: &PopupWindow, lines: &[crate::app::clock::ClockLine]) {
    let model: std::vec::Vec<PopupClockLine> = lines
        .iter()
        .map(|line| PopupClockLine {
            is_local: line.zone_city.is_none(),
            zone_city: line.zone_city.clone().unwrap_or_default().into(),
            offset: line.offset.clone().into(),
            time: line.time.clone().into(),
        })
        .collect();
    popup.set_clocks(slint::ModelRc::new(slint::VecModel::from(model)));
}

/// Sets the one Slint copy of the display name from `app::APP_NAME` (ADR-0017). Called once per
/// window before it is shown, so the popup title and any later window title read one value.
pub fn set_app_name(popup: &PopupWindow) {
    popup
        .global::<AppInfo>()
        .set_app_name(crate::app::APP_NAME.into());
}

/// Maps the `theme` setting of WR-06 onto the `ThemeChoice` of `ui/theme.slint`.
pub fn theme_choice(theme: &str) -> crate::ThemeChoice {
    match theme {
        "light" => crate::ThemeChoice::Light,
        "dark" => crate::ThemeChoice::Dark,
        _ => crate::ThemeChoice::System,
    }
}

/// The WR-01 AC5 menu labels, read from the translated `MenuText` global (W1B-03). The taskbar
/// text's right-click menu and the tray menu read the same `@tr` strings.
pub fn menu_labels(popup: &PopupWindow) -> MenuLabels {
    let menu = popup.global::<MenuText>();
    MenuLabels {
        open: menu.get_open().to_string(),
        copy_bs: menu.get_copy_bs().to_string(),
        copy_ad: menu.get_copy_ad().to_string(),
        settings: menu.get_settings().to_string(),
        quit: menu.get_quit().to_string(),
    }
}

impl From<crate::app::taskbar::Rect> for Rect {
    fn from(rect: crate::app::taskbar::Rect) -> Self {
        Rect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        }
    }
}

/// Positions the popup above `anchor`, left-aligned and clamped to the primary monitor (WR-12
/// AC3). The anchor is physical pixels; the popup position is logical, so both are scaled by the
/// window's scale factor before the pure `above_position` maths runs. winit 0.30 exposes the full
/// monitor rect but not the work area; WR-12 only shows on a bottom taskbar, where the popup opens
/// above the text and the full monitor rect clamps it correctly.
// Screen coordinates are far below f32's exact integer range (2^24), so the f64-to-i32 and
// i32-to-f32 casts are lossless.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn position_above(popup: &PopupWindow, anchor: Rect) {
    use slint::winit_030::WinitWindowAccessor;

    let Some(monitor) = popup
        .window()
        .with_winit_window(slint::winit_030::winit::window::Window::primary_monitor)
        .flatten()
    else {
        return; // No monitor (headless); keep the OS default placement.
    };
    let scale = f64::from(popup.window().scale_factor());
    let position = monitor.position();
    let size = monitor.size();
    let work = Rect {
        left: (f64::from(position.x) / scale) as i32,
        top: (f64::from(position.y) / scale) as i32,
        right: (f64::from(position.x) / scale + f64::from(size.width) / scale) as i32,
        bottom: (f64::from(position.y) / scale + f64::from(size.height) / scale) as i32,
    };
    let anchor = Rect {
        left: (f64::from(anchor.left) / scale) as i32,
        top: (f64::from(anchor.top) / scale) as i32,
        right: (f64::from(anchor.right) / scale) as i32,
        bottom: (f64::from(anchor.bottom) / scale) as i32,
    };
    let (x, y) = above_position(anchor, (POPUP_W as i32, POPUP_H as i32), work);
    popup
        .window()
        .set_position(slint::WindowPosition::Logical(slint::LogicalPosition::new(
            x as f32, y as f32,
        )));
}

/// Positions the popup at the OS-specific corner of the primary monitor (WR-02 AC1).
/// Screen coordinates are far below f32's exact integer range (2^24), so the f64-to-f32 cast
/// at the end is lossless.
#[allow(clippy::cast_possible_truncation)]
fn position_tray_corner(popup: &PopupWindow) {
    use slint::winit_030::WinitWindowAccessor;

    let Some(size) = popup
        .window()
        .with_winit_window(|w| w.primary_monitor().map(|m| m.size()))
        .flatten()
    else {
        return; // No monitor (headless); keep the OS default placement.
    };
    let scale = f64::from(popup.window().scale_factor());
    let monitor_w = f64::from(size.width) / scale;
    let monitor_h = f64::from(size.height) / scale;
    let (x, y) = tray_corner_position(monitor_w, monitor_h);
    popup
        .window()
        .set_position(slint::WindowPosition::Logical(slint::LogicalPosition::new(
            x as f32, y as f32,
        )));
}

/// The popup size and the per-OS corner margins (logical px), WR-02 AC1.
const POPUP_W: f64 = 360.0;
const POPUP_H: f64 = 440.0;

/// Top-left position of the popup for the primary monitor, per OS (WR-02 AC1). Wayland ignores
/// `set_position`, so the Linux value is a hint only.
#[cfg(target_os = "windows")]
fn tray_corner_position(monitor_w: f64, monitor_h: f64) -> (f64, f64) {
    // Bottom-right, 12 px from the right, 56 px above the bottom (standard taskbar).
    (monitor_w - POPUP_W - 12.0, monitor_h - POPUP_H - 56.0)
}

#[cfg(target_os = "macos")]
fn tray_corner_position(monitor_w: f64, monitor_h: f64) -> (f64, f64) {
    // Top-right, 32 px from the top (below the menu bar).
    let _ = monitor_h;
    (monitor_w - POPUP_W - 12.0, 32.0)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn tray_corner_position(monitor_w: f64, monitor_h: f64) -> (f64, f64) {
    // Top-right (X11); Wayland lets the compositor decide.
    let _ = monitor_h;
    (monitor_w - POPUP_W - 12.0, 0.0)
}

/// The popup's top-left above `anchor`, left-aligned and clamped to `work_area` (W1B). Pure so
/// the clamping is unit-tested now (W1-04). `max`/`min` never panic even when the popup is wider
/// than the work area.
fn above_position(anchor: Rect, popup_size: (i32, i32), work_area: Rect) -> (i32, i32) {
    let (w, h) = popup_size;
    let x = anchor.left.max(work_area.left).min(work_area.right - w);
    let y = (anchor.top - h)
        .max(work_area.top)
        .min(work_area.bottom - h);
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn above_clamps_to_work_area() {
        // Anchor fully inside the work area: popup sits directly above it, left-aligned.
        let area = rect(0, 0, 1920, 1080);
        let anchor = rect(1000, 900, 1100, 950);
        assert_eq!(above_position(anchor, (360, 440), area), (1000, 460));
        // Anchor near the left and top edges: clamped inside.
        let anchor = rect(-50, 100, 50, 150);
        assert_eq!(above_position(anchor, (360, 440), area), (0, 0));
        // Anchor near the right and top edges: clamped to the right.
        let anchor = rect(1900, 500, 1950, 550);
        assert_eq!(above_position(anchor, (360, 440), area), (1560, 60));
    }

    /// A platform that builds a component without a display, for the one test that reads a Slint
    /// global back. The software renderer is a default feature (ADR-0010).
    struct TestPlatform;

    impl slint::platform::Platform for TestPlatform {
        fn create_window_adapter(
            &self,
        ) -> Result<std::rc::Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
            Ok(
                slint::platform::software_renderer::MinimalSoftwareWindow::new(
                    slint::platform::software_renderer::RepaintBufferType::ReusedBuffer,
                ),
            )
        }
    }

    #[test]
    fn app_name_is_one_value_in_rust_and_slint() {
        use crate::app::APP_NAME;

        assert_eq!(APP_NAME, "MitiBar");
        // A process can set one platform, and no other test creates a component.
        let _ = slint::platform::set_platform(Box::new(TestPlatform));
        let popup = crate::PopupWindow::new().expect("popup component");
        set_app_name(&popup);
        assert_eq!(
            popup.global::<crate::AppInfo>().get_app_name().as_str(),
            APP_NAME
        );
    }
}
