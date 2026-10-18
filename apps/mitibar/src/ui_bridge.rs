//! Maps `app` view models to Slint generated types and callbacks (ADR-0015). The only place that
//! converts between the two. W1 adds the popup placement (W1-04); W1B wires `PopupAnchor::Above`
//! for the taskbar text (W1B-03).

use nepdate::{Lang, Month, NepaliDate};
use slint::ComponentHandle;

use crate::app::clock::Header;
use crate::app::holidays::NextHoliday;
use crate::app::month_view::{Cell, DayDetail, MonthView};
use crate::platform::taskbar::MenuLabels;
use crate::{AppInfo, CellData, DayDetailData, MenuText, PopupClockLine, PopupWindow};

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

/// Fills the grid from `app::MonthView` (W2-02, WR-03). The language is passed separately because
/// the view keeps names, not the language.
pub fn set_month(popup: &PopupWindow, view: &MonthView, lang: Lang) {
    let cells: Vec<CellData> = view
        .cells
        .iter()
        .map(|cell| cell_data(cell, lang))
        .collect();
    popup.set_cells(slint::ModelRc::new(slint::VecModel::from(cells)));
    let headers: Vec<slint::SharedString> = view
        .weekday_headers
        .iter()
        .map(|header| (*header).into())
        .collect();
    popup.set_weekday_headers(slint::ModelRc::new(slint::VecModel::from(headers)));
    let months: Vec<slint::SharedString> = Month::ALL
        .iter()
        .map(|month| month.name(lang).into())
        .collect();
    popup.set_month_names(slint::ModelRc::new(slint::VecModel::from(months)));
    popup.set_month_index(i32::from(view.month.number()) - 1);
    popup.set_year(i32::from(view.year));
    popup.set_month_title(view.title.as_str().into());
    popup.set_can_prev(view.can_prev);
    popup.set_can_next(view.can_next);
}

/// One `Cell` to one `CellData` (WR-03 AC3). The model keeps Saturday and public holidays
/// apart; the UI gives the holiday colour priority, so a holiday on a Saturday stays red.
fn cell_data(cell: &Cell, lang: Lang) -> CellData {
    CellData {
        bs_day: cell.bs_day.as_str().into(),
        ad_day: cell.ad_day.as_str().into(),
        ad_month: cell.ad_month_marker.unwrap_or_default().into(),
        is_today: cell.is_today,
        is_holiday: cell.is_public_holiday,
        is_saturday: cell.is_saturday,
        tithi: cell.tithi.unwrap_or_default().into(),
        has_events: cell.has_events,
        is_padding: cell.date.is_none(),
        accessible_label: cell
            .date
            .map_or_else(String::new, |date| cell_label(date, lang))
            .into(),
    }
}

/// The accessible name of a day cell: ISO BS and AD dates, the weekday and the tithi, all data.
/// The holiday suffix is a `@tr` string added in the markup.
fn cell_label(date: NepaliDate, lang: Lang) -> String {
    let mut label = format!(
        "{} {} {}",
        date,
        date.to_gregorian(),
        date.weekday().name(lang)
    );
    if let Some(tithi) = nepdate_patro::info(date).tithi() {
        label.push(' ');
        label.push_str(tithi.name(lang));
    }
    label
}

/// Fills the WR-03 AC8 line. `None` hides it; the sentence itself is built in `.slint`.
pub fn set_next_holiday(popup: &PopupWindow, holiday: Option<&NextHoliday>) {
    let (name, days) = holiday.map_or(("", -1), |holiday| {
        (
            holiday.name.unwrap_or_default(),
            i32::try_from(holiday.days_away).unwrap_or(-1),
        )
    });
    popup.set_holiday_name(name.into());
    popup.set_holiday_days(days);
}

/// Fills the day detail overlay (W2-03, WR-04). `None` hides it and leaves the last text in
/// place; the popup is only shown with a complete detail.
pub fn set_detail(popup: &PopupWindow, detail: Option<&DayDetail>) {
    let Some(detail) = detail else {
        popup.set_detail_visible(false);
        return;
    };
    let events: Vec<slint::SharedString> =
        detail.events.iter().map(|event| (*event).into()).collect();
    popup.set_detail(DayDetailData {
        bs_long: detail.bs_long.as_str().into(),
        ad_long: detail.ad_long.as_str().into(),
        weekday: detail.weekday.as_str().into(),
        tithi: detail.tithi.unwrap_or_default().into(),
        is_holiday: detail.is_holiday,
        events: slint::ModelRc::new(slint::VecModel::from(events)),
        copy_bs: detail.copy_bs.as_str().into(),
        copy_ad: detail.copy_ad.as_str().into(),
    });
    popup.set_detail_visible(true);
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

/// The popup size and the per-OS corner margins (logical px), WR-02 AC1. 520 replaced 440 in the
/// W2 review to fit the 44 px grid cells and a 6-row month.
const POPUP_W: f64 = 360.0;
const POPUP_H: f64 = 520.0;

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
        assert_eq!(above_position(anchor, (360, 520), area), (1000, 380));
        // Anchor near the left and top edges: clamped inside.
        let anchor = rect(-50, 100, 50, 150);
        assert_eq!(above_position(anchor, (360, 520), area), (0, 0));
        // Anchor near the right and top edges: clamped to the right.
        let anchor = rect(1900, 500, 1950, 550);
        assert_eq!(above_position(anchor, (360, 520), area), (1560, 0));
    }

    #[test]
    fn day_cell_label_has_the_dates_weekday_and_tithi() {
        use nepdate::{Lang, NepaliDate};

        let date = NepaliDate::new(2081, 1, 1).expect("date");
        assert_eq!(
            cell_label(date, Lang::En),
            "2081/01/01 2024-04-13 Saturday Panchami"
        );
        assert_eq!(
            cell_label(date, Lang::Ne),
            "2081/01/01 2024-04-13 शनिवार पञ्चमी"
        );
    }

    #[test]
    fn cell_data_keeps_a_holiday_on_a_saturday() {
        use crate::app::month_view::month_view;

        let new_year = NepaliDate::new(2081, 1, 1).expect("date");
        let view = month_view(2081, Month::Baishakh, new_year, Lang::En);
        let cell = view
            .cells
            .iter()
            .find(|cell| cell.date == Some(new_year))
            .expect("New Year");
        let data = cell_data(cell, Lang::En);
        assert!(data.is_saturday);
        assert!(
            data.is_holiday,
            "the public-holiday colour must win over the Saturday colour"
        );
    }

    /// A platform that builds a component without a display, for the tests that read a Slint
    /// model or global back. The software renderer is a default feature (ADR-0010).
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

    /// A process can set one platform, so this single component test is the only test that
    /// creates one. Slint's platform is also tied to the thread that set it, so splitting the
    /// assertions over tests would fail under `cargo test` (one process, many threads).
    static PLATFORM: std::sync::Once = std::sync::Once::new();

    fn init_platform() {
        PLATFORM.call_once(|| {
            let _ = slint::platform::set_platform(Box::new(TestPlatform));
        });
    }

    #[test]
    fn popup_bridge_maps_the_name_month_and_detail() {
        use crate::app::APP_NAME;
        use crate::app::month_view::{day_detail, month_view};
        use nepdate::{Lang, Month, NepaliDate};
        use slint::Model;

        assert_eq!(APP_NAME, "MitiBar");
        init_platform();
        let popup = crate::PopupWindow::new().expect("popup component");

        set_app_name(&popup);
        assert_eq!(
            popup.global::<crate::AppInfo>().get_app_name().as_str(),
            APP_NAME
        );

        let today = NepaliDate::new(2083, 6, 1).expect("date");
        let view = month_view(2083, Month::Ashoj, today, Lang::En);
        set_month(&popup, &view, Lang::En);
        assert_eq!(
            popup.get_month_title().as_str(),
            "Ashoj 2083 · Sep/Oct 2026"
        );
        assert_eq!(popup.get_month_index(), 5);
        assert_eq!(popup.get_year(), 2083);
        assert_eq!(popup.get_cells().row_count(), view.cells.len());
        let first = popup.get_cells().row_data(4).expect("Ashoj 1");
        assert_eq!(first.bs_day.as_str(), "1");
        assert_eq!(first.ad_month.as_str(), "");
        assert_eq!(popup.get_month_names().row_count(), 12);

        let detail = day_detail(NepaliDate::new(2081, 1, 1).expect("date"), Lang::En);
        set_detail(&popup, Some(&detail));
        assert!(popup.get_detail_visible());
        assert_eq!(popup.get_detail().bs_long.as_str(), "Baishakh 1, 2081");
        assert_eq!(popup.get_detail().events.row_count(), 3);
        set_detail(&popup, None);
        assert!(!popup.get_detail_visible());
    }
}
