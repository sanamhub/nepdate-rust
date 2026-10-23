//! Maps `app` view models to Slint generated types and callbacks (ADR-0015). The only place that
//! converts between the two, including popup placement and the bar anchor.

use nepdate::{DateOrder, Lang, LongDate, Month, NepaliDate, Separator};
use slint::{ComponentHandle, Model};

use crate::app::bar_place::{self, Monitor, Point, Rect, Size, to_physical};
use crate::app::clock::Header;
use crate::app::converter::ConvertOutcome;
use crate::app::format;
use crate::app::holidays::NextHoliday;
use crate::app::line_editor;
use crate::app::month_view::{Cell, DayDetail, DayTooltip, MonthView};
use crate::app::settings::Settings;
use crate::app::settings_place;
use crate::app::shortcut::{Key, Pressed};
use crate::app::zoom::{Zoom, ZoomState};
use crate::platform::bar::Anchor;
use crate::platform::frame::{self, Frame, Surface};
use crate::{
    AppInfo, CellData, ConverterData, DayDetailData, FrameStyle, PopupClockLine, PopupWindow,
    SettingsData, SettingsWindow, TipData, UpcomingData, ZoomCellData,
};

/// Where to open the popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupAnchor {
    /// The fixed corner of the primary monitor per OS (WR-02 AC1).
    TrayCorner,
    /// Above or below the bar, left-aligned with it, fully on-screen (WR-02 AC1, ADR-0019 point 8).
    Bar(Anchor),
}

/// Positions and shows the popup at `anchor`, then gives the inner `FocusScope` focus so Escape
/// and focus-loss hiding work (WR-02 AC2).
pub fn show_popup_at(popup: &PopupWindow, anchor: PopupAnchor) -> anyhow::Result<()> {
    match anchor {
        PopupAnchor::TrayCorner => position_tray_corner(popup),
        PopupAnchor::Bar(anchor) => position_at_bar(popup, anchor),
    }
    popup.show()?;
    // The window exists from the second open on. On the first, the filter below frames it with
    // the first event it sees, before the first paint.
    let mut framed = apply_frame(popup);
    // Window deactivation reaches only the focused item. A FocusScope handler would miss a
    // child control, so the winit filter hides the popup whatever holds focus.
    {
        use slint::winit_030::winit::event::WindowEvent as WinitEvent;
        use slint::winit_030::{EventResult, WinitWindowAccessor};
        let weak = popup.as_weak();
        popup.window().on_winit_window_event(move |_window, event| {
            if !framed && let Some(popup) = weak.upgrade() {
                framed = apply_frame(&popup);
            }
            if let WinitEvent::Focused(active) = event
                && let Some(popup) = weak.upgrade()
            {
                if *active {
                    popup.invoke_window_activated();
                } else {
                    popup.invoke_window_deactivated();
                }
            }
            EventResult::Propagate
        });
    }
    popup.invoke_focus();
    Ok(())
}

/// Tells the markup who draws the corners (WR-02 AC5). Reads the winit window's handle, asks
/// `platform::frame` for the style (on Windows 11 that asks DWM), and sets it. A window with no
/// handle yet returns `false` and keeps the square hairline default: Slint creates the winit
/// window after `show` returns, on the first open.
fn apply_frame(popup: &PopupWindow) -> bool {
    use slint::winit_030::WinitWindowAccessor;
    use slint::winit_030::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let surface = popup
        .window()
        .with_winit_window(|window| {
            window
                .window_handle()
                .ok()
                .map(|handle| match handle.as_raw() {
                    RawWindowHandle::Win32(handle) => Surface::Win32(handle.hwnd.get()),
                    RawWindowHandle::AppKit(_) => Surface::AppKit,
                    RawWindowHandle::Wayland(_) => Surface::Wayland,
                    _ => Surface::Other,
                })
        })
        .flatten();
    let Some(surface) = surface else {
        return false;
    };
    popup.set_frame_style(match frame::choose(surface) {
        Frame::System => FrameStyle::System,
        Frame::Rounded => FrameStyle::Rounded,
        Frame::Square => FrameStyle::Square,
    });
    true
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
/// window before it is shown, so the popup title and the Settings title read one value.
pub fn set_app_name<'a, T>(ui: &'a T)
where
    T: ComponentHandle,
    AppInfo<'a>: slint::Global<'a, T>,
{
    ui.global::<AppInfo<'a>>()
        .set_app_name(crate::app::APP_NAME.into());
}

/// Fills the calendar for the current zoom level (WR-03 AC6): the title, the Previous, Next and
/// Today controls, the roving cursor and the cells. `view` is the month's cells and is only
/// needed, and only built, in the month view. The language is passed separately because the
/// models keep names, not the language.
pub fn set_calendar(
    popup: &PopupWindow,
    zoom: ZoomState,
    view: Option<&MonthView>,
    today: NepaliDate,
    lang: Lang,
) {
    popup.set_zoom_mode(match zoom.zoom() {
        Zoom::Month => 0,
        Zoom::Year => 1,
        Zoom::Decade => 2,
    });
    popup.set_can_prev(zoom.can_step(-1));
    popup.set_can_next(zoom.can_step(1));
    popup.set_today_shown(zoom.today_pill_shown(today));
    // The AD day is body size (13 px); Devanagari needs 12 px minimum and is narrower at 12 px
    // (widget-design §3/§4). The UI only needs the size, not the language.
    popup.set_ad_day_size(if lang == Lang::Ne { 12.0 } else { 13.0 });
    // The tithi is a Latin caption at 11 px in English; Devanagari needs 12 px or more
    // (widget-design §3/§4).
    popup.set_tithi_size(if lang == Lang::Ne { 12.0 } else { 11.0 });
    let digits = |year: u16| lang.digits(u32::from(year), 1).to_string();
    match zoom.zoom() {
        Zoom::Month => {
            let Some(view) = view else {
                return;
            };
            set_month_cells(popup, view, lang);
            popup.set_month_title(view.title.as_str().into());
            popup.set_nav_title(view.title.as_str().into());
            let cursor = zoom
                .cursor_date()
                .and_then(|date| view.cells.iter().position(|cell| cell.date == Some(date)))
                .and_then(|index| i32::try_from(index).ok());
            popup.set_cursor_index(cursor.unwrap_or(-1));
        }
        Zoom::Year => {
            let year = digits(zoom.year());
            let cells: Vec<ZoomCellData> = Month::ALL
                .iter()
                .map(|month| ZoomCellData {
                    label: month.name(lang).into(),
                    is_today: zoom.year() == today.year() && *month == today.month(),
                    is_viewed: *month == zoom.month(),
                    enabled: true,
                    accessible_label: format!("{} {year}", month.name(lang)).into(),
                })
                .collect();
            popup.set_zoom_cells(slint::ModelRc::new(slint::VecModel::from(cells)));
            popup.set_nav_title(year.into());
            popup.set_cursor_index(i32::from(zoom.cursor()));
        }
        Zoom::Decade => {
            let cells: Vec<ZoomCellData> = zoom
                .decade_years()
                .iter()
                .map(|year| match year {
                    Some(year) => ZoomCellData {
                        label: digits(*year).into(),
                        is_today: *year == today.year(),
                        is_viewed: *year == zoom.year(),
                        enabled: true,
                        accessible_label: digits(*year).into(),
                    },
                    None => ZoomCellData::default(),
                })
                .collect();
            popup.set_zoom_cells(slint::ModelRc::new(slint::VecModel::from(cells)));
            popup.set_nav_title(
                format!("{}–{}", digits(zoom.decade()), digits(zoom.decade() + 9)).into(),
            );
            popup.set_cursor_index(i32::from(zoom.cursor()));
        }
    }
}

/// The month view's day cells and weekday headers.
fn set_month_cells(popup: &PopupWindow, view: &MonthView, lang: Lang) {
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
/// Both dates use the language's digits, like the visible grid. The holiday suffix is a `@tr`
/// string added in the markup.
fn cell_label(date: NepaliDate, lang: Lang) -> String {
    let mut label = format!(
        "{} {} {}",
        date.short(DateOrder::Ymd, Separator::Slash, true, lang),
        format::ad_short(date.to_gregorian(), lang),
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

/// Fills the upcoming-holidays list (WR-14 AC4): the name, the BS date and how many days away.
pub fn set_upcoming(popup: &PopupWindow, holidays: &[NextHoliday], lang: Lang) {
    let rows: Vec<UpcomingData> = holidays
        .iter()
        .map(|holiday| UpcomingData {
            name: holiday.name.unwrap_or_default().into(),
            date: holiday
                .date
                .long(
                    LongDate {
                        weekday: false,
                        year: true,
                        pad: false,
                    },
                    lang,
                )
                .to_string()
                .into(),
            days: i32::try_from(holiday.days_away).unwrap_or(0),
        })
        .collect();
    popup.set_upcoming(slint::ModelRc::new(slint::VecModel::from(rows)));
}

/// Fills the day tooltip card (WR-14 AC2). A field with no data stays empty, so its line is not
/// drawn.
pub fn set_tip(popup: &PopupWindow, tip: &DayTooltip) {
    popup.set_tip(TipData {
        bs_long: tip.bs_long.as_str().into(),
        ad_long: tip.ad_long.as_str().into(),
        weekday: tip.weekday.as_str().into(),
        tithi: tip.tithi.unwrap_or_default().into(),
        holiday: tip.holiday.unwrap_or_default().into(),
        events: i32::try_from(tip.events).unwrap_or(0),
    });
}

/// Fills the day detail overlay (WR-04). `None` hides it and leaves the last text in
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

/// Fills the converter result area. The UI shows the message, or the short and long
/// forms; an empty outcome clears all three.
pub fn set_converter(popup: &PopupWindow, outcome: &ConvertOutcome) {
    let (short, long, error) = match outcome {
        ConvertOutcome::Empty => (String::new(), String::new(), String::new()),
        ConvertOutcome::Ok { short, long } => (short.clone(), long.clone(), String::new()),
        ConvertOutcome::Error(message) => (String::new(), String::new(), message.clone()),
    };
    popup.set_converter_data(ConverterData {
        short: short.as_str().into(),
        long: long.as_str().into(),
        error: error.as_str().into(),
    });
}

/// Sets the Days tab defaults to today's BS date. A user-typed value is kept.
pub fn reset_converter(popup: &PopupWindow, today: NepaliDate, lang: Lang) {
    let today = today
        .short(
            nepdate::DateOrder::Ymd,
            nepdate::Separator::Slash,
            true,
            lang,
        )
        .to_string();
    if popup.get_converter_from().is_empty() {
        popup.set_converter_from(today.as_str().into());
    }
    if popup.get_converter_to().is_empty() {
        popup.set_converter_to(today.as_str().into());
    }
    set_converter(popup, &ConvertOutcome::Empty);
}

/// Every IANA zone that `TimeZone::get` can load, sorted (WR-06 AC4). That is jiff's bundled
/// database on Windows and the system zoneinfo on macOS and Linux, where jiff bundles nothing.
/// The zoneinfo helper files are not zones a person would pick, so they are left out.
#[must_use]
pub fn all_time_zones() -> Vec<String> {
    let mut zones: Vec<String> = jiff::tz::db()
        .available()
        .map(|name| name.as_str().to_owned())
        .filter(|name| !matches!(name.as_str(), "Factory" | "localtime" | "posixrules"))
        .collect();
    zones.sort();
    zones.dedup();
    zones
}

/// The zones whose id contains `filter`, case-insensitively, by city or region. An underscore in
/// an id reads as a space, so `new york` finds `America/New_York`. An empty filter returns all.
#[must_use]
pub fn filter_time_zones(zones: &[String], filter: &str) -> Vec<String> {
    if filter.is_empty() {
        return zones.to_vec();
    }
    let needle = filter.trim().to_lowercase();
    zones
        .iter()
        .filter(|zone| zone.to_lowercase().replace('_', " ").contains(&needle))
        .cloned()
        .collect()
}

/// The extra clock's drop-down rows for what is typed (WR-06 AC4): the matching zones, with an
/// empty id, which the window shows as "None", first when nothing is typed or the text could
/// start "None". Also the row to highlight: the chosen zone when nothing is typed, else the first.
#[must_use]
pub fn zone_rows(zones: &[String], query: &str, current: &str) -> (Vec<String>, usize) {
    let typed = query.trim().to_lowercase();
    let mut rows = Vec::new();
    if typed.is_empty() || "none".contains(&typed) {
        rows.push(String::new());
    }
    rows.extend(filter_time_zones(zones, query));
    let highlight = if typed.is_empty() {
        rows.iter().position(|row| row == current).unwrap_or(0)
    } else {
        0
    };
    (rows, highlight)
}

/// Fills the Settings window from `Settings` (WR-06 AC6). `bar_available` is true where the app has
/// a bar (Windows); `shortcut_available` is false where the OS has no way to grab a global key.
pub fn set_settings(
    window: &SettingsWindow,
    settings: &Settings,
    bar_available: bool,
    shortcut_available: bool,
) {
    let language_index = i32::from(u8::from(settings.lang() == Lang::Ne));
    let theme_index = match settings.theme.as_str() {
        "light" => 1,
        "dark" => 2,
        _ => 0,
    };
    // The chip rows are repeaters over these models. Replacing a model rebuilds every chip and
    // drops the keyboard focus from the one that has it, so the models are updated in place; the
    // window refreshes on every change and every `Saved` note.
    let previous = window.get_data();
    let top = synced(&previous.top, &settings.bar_top);
    let bottom = synced(&previous.bottom, &settings.bar_bottom);
    let unused_ids: Vec<String> = line_editor::unused(settings)
        .iter()
        .map(|field| field.id().to_owned())
        .collect();
    let unused = synced(&previous.unused, &unused_ids);
    window.set_data(SettingsData {
        language_index,
        theme_index,
        clock_24h: settings.clock_24h,
        show_seconds: settings.show_seconds,
        zone: settings.extra_time_zone.as_str().into(),
        launch_at_login: settings.launch_at_login,
        bar: settings.bar,
        bar_locked: settings.bar_locked,
        bar_available,
        top,
        bottom,
        unused,
        only_one_field: line_editor::is_last_chip(settings),
        shortcut: settings
            .shortcut_combo()
            .map(|combo| combo.to_string())
            .unwrap_or_default()
            .into(),
        shortcut_available,
        version: env!("CARGO_PKG_VERSION").into(),
    });
}

/// `items` as a model of ids, reusing `model` when it is already a `VecModel` so that a repeater
/// over it keeps its rows: only the rows that differ change, and rows are added or removed at the
/// end.
fn synced(
    model: &slint::ModelRc<slint::SharedString>,
    items: &[String],
) -> slint::ModelRc<slint::SharedString> {
    let Some(rows) = model
        .as_any()
        .downcast_ref::<slint::VecModel<slint::SharedString>>()
    else {
        let fresh: Vec<slint::SharedString> = items
            .iter()
            .map(|item| slint::SharedString::from(item.as_str()))
            .collect();
        return slint::ModelRc::new(slint::VecModel::from(fresh));
    };
    while rows.row_count() > items.len() {
        rows.remove(rows.row_count() - 1);
    }
    for (index, item) in items.iter().enumerate() {
        if index < rows.row_count() {
            if rows.row_data(index).as_deref() != Some(item.as_str()) {
                rows.set_row_data(index, item.as_str().into());
            }
        } else {
            rows.push(item.as_str().into());
        }
    }
    model.clone()
}

/// Fills the extra clock's drop-down (WR-06 AC4).
pub fn set_zone_rows(window: &SettingsWindow, rows: &[String], highlight: usize) {
    let model: Vec<slint::SharedString> = rows
        .iter()
        .map(|row| slint::SharedString::from(row.as_str()))
        .collect();
    window.set_zone_rows(slint::ModelRc::new(slint::VecModel::from(model)));
    window.set_zone_highlight(i32::try_from(highlight).unwrap_or(0));
}

/// Reads a Slint key event's text for the shortcut field (WR-13 AC2): a main key, a modifier on
/// its own, or a key the field leaves to the window, such as Tab and Escape.
#[must_use]
pub fn pressed(text: &str) -> Pressed {
    use slint::platform::Key as Special;

    let mut chars = text.chars();
    let (Some(only), None) = (chars.next(), chars.next()) else {
        return Pressed::Navigation;
    };
    let is = |key: Special| only == char::from(key);
    if [
        Special::Shift,
        Special::ShiftR,
        Special::Control,
        Special::ControlR,
        Special::Alt,
        Special::AltGr,
        Special::Meta,
        Special::MetaR,
        Special::CapsLock,
    ]
    .into_iter()
    .any(is)
    {
        return Pressed::Modifier;
    }
    if [
        Special::Tab,
        Special::Backtab,
        Special::Escape,
        Special::Return,
        Special::Backspace,
    ]
    .into_iter()
    .any(is)
    {
        return Pressed::Navigation;
    }
    let named = [
        (Special::Home, Key::Home),
        (Special::End, Key::End),
        (Special::PageUp, Key::PageUp),
        (Special::PageDown, Key::PageDown),
        (Special::Insert, Key::Insert),
        (Special::Delete, Key::Delete),
        (Special::LeftArrow, Key::Left),
        (Special::RightArrow, Key::Right),
        (Special::UpArrow, Key::Up),
        (Special::DownArrow, Key::Down),
        (Special::F1, Key::Function(1)),
        (Special::F2, Key::Function(2)),
        (Special::F3, Key::Function(3)),
        (Special::F4, Key::Function(4)),
        (Special::F5, Key::Function(5)),
        (Special::F6, Key::Function(6)),
        (Special::F7, Key::Function(7)),
        (Special::F8, Key::Function(8)),
        (Special::F9, Key::Function(9)),
        (Special::F10, Key::Function(10)),
        (Special::F11, Key::Function(11)),
        (Special::F12, Key::Function(12)),
    ];
    if let Some((_, key)) = named.into_iter().find(|(special, _)| is(*special)) {
        return Pressed::Key(key);
    }
    if only == ' ' {
        return Pressed::Key(Key::Space);
    }
    Key::from_char(only).map_or(Pressed::Navigation, Pressed::Key)
}

/// Where a Settings window opens, with the monitor it is placed on (WR-06 AC1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SettingsHint {
    /// The work area and DPI of the bar's monitor, when Settings was asked for from the bar and
    /// the popup is closed.
    pub bar_monitor: Option<(Rect, u32)>,
}

/// The Settings window's logical size (WR-06, widget-design §8).
const SETTINGS_W: i32 = 420;
const SETTINGS_H: i32 = 560;

/// Positions the Settings window beside the popup, or centred when the popup is closed or neither
/// side has room (WR-06 AC1). Leaves it to the OS when no monitor can be read.
pub fn place_settings(window: &SettingsWindow, popup: Option<&PopupWindow>, hint: SettingsHint) {
    let monitors = desktop_monitors(popup);
    let popup_rect = popup
        .filter(|popup| popup.window().is_visible())
        .map(|popup| {
            let at = popup.window().position();
            let size = popup.window().size();
            Rect::at(
                Point { x: at.x, y: at.y },
                Size {
                    width: i32::try_from(size.width).unwrap_or(0),
                    height: i32::try_from(size.height).unwrap_or(0),
                },
            )
        });
    let (work, dpi) = if let Some(rect) = popup_rect {
        let centre = Point {
            x: i32::midpoint(rect.left, rect.right),
            y: i32::midpoint(rect.top, rect.bottom),
        };
        bar_place::monitor_at(&monitors, centre).map(|monitor| (monitor.work, monitor.dpi))
    } else if let Some(bar) = hint.bar_monitor {
        Some(bar)
    } else {
        monitors
            .iter()
            .find(|monitor| monitor.primary)
            .or_else(|| monitors.first())
            .map(|monitor| (monitor.work, monitor.dpi))
    }
    .map_or((None, 96), |(work, dpi)| (Some(work), dpi));
    let Some(work) = work else {
        return;
    };
    let client = Size {
        width: to_physical(SETTINGS_W, dpi),
        height: to_physical(SETTINGS_H, dpi),
    };
    // The window has a frame of its own, with an invisible resize border on Windows; place what is
    // drawn.
    let (visible, insets) = crate::platform::screen::frame_metrics(dpi).map_or(
        (client, settings_place::Insets::default()),
        |(caption, frame)| settings_place::framed(client, caption, frame),
    );
    let at = settings_place::position(popup_rect, visible, insets, work, dpi);
    window
        .window()
        .set_position(slint::WindowPosition::Physical(
            slint::PhysicalPosition::new(at.x, at.y),
        ));
}

/// Every monitor the placement can use. Windows asks the shell; elsewhere the popup's winit window
/// lists them, so with no popup yet there is nothing to go on and the OS places the window.
fn desktop_monitors(popup: Option<&PopupWindow>) -> Vec<Monitor> {
    #[cfg(windows)]
    {
        let _ = popup;
        crate::platform::screen::monitors()
    }
    #[cfg(not(windows))]
    {
        use slint::winit_030::WinitWindowAccessor;

        let Some(popup) = popup else {
            return Vec::new();
        };
        popup
            .window()
            .with_winit_window(|window| {
                let primary = window.primary_monitor();
                window
                    .available_monitors()
                    .map(|monitor| {
                        let at = monitor.position();
                        let size = monitor.size();
                        let bounds = Rect::new(
                            at.x,
                            at.y,
                            at.x + i32::try_from(size.width).unwrap_or(0),
                            at.y + i32::try_from(size.height).unwrap_or(0),
                        );
                        Monitor {
                            name: monitor.name().unwrap_or_default(),
                            bounds,
                            work: bounds,
                            // Winit gives a scale; 96 dpi is 1.0.
                            dpi: (monitor.scale_factor() * 96.0).round() as u32,
                            primary: primary.as_ref() == Some(&monitor),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Maps the `theme` setting of WR-06 onto the `ThemeChoice` of `ui/theme.slint`.
pub fn theme_choice(theme: &str) -> crate::ThemeChoice {
    match theme {
        "light" => crate::ThemeChoice::Light,
        "dark" => crate::ThemeChoice::Dark,
        _ => crate::ThemeChoice::System,
    }
}

/// Positions the popup for a click on the bar: above it when there is room, below it otherwise,
/// left-aligned and inside the work area of the bar's monitor (WR-02 AC1). The anchor is in
/// physical pixels, and so is the position set here; the popup's size is its logical size at the
/// scale of the monitor the bar is on.
fn position_at_bar(popup: &PopupWindow, anchor: Anchor) {
    let size = popup_size(anchor.dpi);
    let at = bar_place::popup_position(anchor.bar, size, anchor.work);
    popup.window().set_position(slint::WindowPosition::Physical(
        slint::PhysicalPosition::new(at.x, at.y),
    ));
}

/// Positions the popup at the tray corner of the primary monitor (WR-02 AC1): bottom-right of the
/// work area, which is the corner above the taskbar wherever the taskbar is. The monitor, its
/// work area and its scale come from the shell, not from the popup's window, which does not exist
/// yet on the first open.
#[cfg(windows)]
fn position_tray_corner(popup: &PopupWindow) {
    let Some(primary) = crate::platform::screen::primary_monitor() else {
        return; // No monitor to read; keep the OS default placement.
    };
    let size = popup_size(primary.dpi);
    let at = bar_place::tray_corner(&primary, size);
    popup.window().set_position(slint::WindowPosition::Physical(
        slint::PhysicalPosition::new(at.x, at.y),
    ));
}

/// Positions the popup at the OS-specific corner of the primary monitor (WR-02 AC1).
/// Screen coordinates are far below f32's exact integer range (2^24), so the f64-to-f32 cast
/// at the end is lossless.
#[cfg(not(windows))]
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

/// The popup size in physical px at `dpi`.
// The popup size is a few hundred px, so the f64-to-i32 cast is lossless.
#[allow(clippy::cast_possible_truncation)]
fn popup_size(dpi: u32) -> Size {
    Size {
        width: to_physical(POPUP_W as i32, dpi),
        height: to_physical(POPUP_H as i32, dpi),
    }
}

/// The popup size and the per-OS corner margins (logical px), WR-02 AC1. 520 replaced 440 in the
/// 520 fits the 44 px grid cells and a 6-row month.
const POPUP_W: f64 = 360.0;
const POPUP_H: f64 = 520.0;

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

#[cfg(test)]
mod tests {
    use super::*;

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
            "२०८१/०१/०१ २०२४-०४-१३ शनिवार पञ्चमी"
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

    #[test]
    fn filter_time_zones_matches_case_insensitively() {
        let zones = vec![
            "Asia/Kathmandu".to_owned(),
            "Europe/Berlin".to_owned(),
            "America/New_York".to_owned(),
        ];
        assert_eq!(
            filter_time_zones(&zones, "kath"),
            vec!["Asia/Kathmandu".to_owned()]
        );
        assert_eq!(
            filter_time_zones(&zones, "BER"),
            vec!["Europe/Berlin".to_owned()]
        );
        assert_eq!(filter_time_zones(&zones, "").len(), 3);
        assert!(filter_time_zones(&zones, "Mars").is_empty());
    }

    #[test]
    fn wr06_settings_window_maps_the_indices_and_lines() {
        use crate::app::settings::Settings;
        use slint::Model;

        crate::test_support::init_platform();
        let window = crate::SettingsWindow::new().expect("settings component");
        let settings = Settings {
            language: "ne".to_owned(),
            theme: "dark".to_owned(),
            clock_24h: true,
            show_seconds: true,
            extra_time_zone: "Asia/Kathmandu".to_owned(),
            launch_at_login: false,
            bar: false,
            bar_locked: true,
            shortcut: "Ctrl+Shift+K".to_owned(),
            ..Settings::default()
        };
        set_settings(&window, &settings, true, true);
        let data = window.get_data();
        assert_eq!(data.language_index, 1);
        assert_eq!(data.theme_index, 2);
        assert!(data.clock_24h);
        assert!(data.show_seconds);
        assert!(!data.launch_at_login);
        assert!(!data.bar);
        assert!(data.bar_locked);
        assert!(data.bar_available);
        assert_eq!(data.zone.as_str(), "Asia/Kathmandu");
        assert_eq!(data.top.row_count(), 3);
        assert_eq!(data.bottom.row_count(), 2);
        assert_eq!(data.unused.row_count(), 5);
        assert!(!data.only_one_field);
        assert_eq!(data.shortcut.as_str(), "Ctrl+Shift+K");
        assert!(data.shortcut_available);
    }

    #[test]
    fn the_zone_rows_start_with_none_when_nothing_is_typed() {
        let zones = vec![
            "Asia/Kathmandu".to_owned(),
            "Asia/Singapore".to_owned(),
            "America/New_York".to_owned(),
        ];
        let (rows, highlight) = zone_rows(&zones, "", "Asia/Singapore");
        assert_eq!(
            rows,
            ["", "Asia/Kathmandu", "Asia/Singapore", "America/New_York"]
        );
        assert_eq!(highlight, 2, "the chosen zone is highlighted");
        let (_, none_chosen) = zone_rows(&zones, "", "");
        assert_eq!(none_chosen, 0);

        let (typed, first) = zone_rows(&zones, "new york", "Asia/Singapore");
        assert_eq!(
            typed,
            ["America/New_York"],
            "an underscore reads as a space"
        );
        assert_eq!(first, 0);
        let (region, _) = zone_rows(&zones, "asia", "");
        assert_eq!(region, ["Asia/Kathmandu", "Asia/Singapore"]);
        let (none, _) = zone_rows(&zones, "no", "");
        assert_eq!(none, [""], "typing the start of None offers None");
    }

    #[test]
    fn a_key_event_reads_as_a_main_key_a_modifier_or_navigation() {
        use slint::platform::Key as Special;

        let text = |key: Special| String::from(char::from(key));
        assert_eq!(pressed("n"), Pressed::Key(Key::Letter('N')));
        assert_eq!(pressed("N"), Pressed::Key(Key::Letter('N')));
        assert_eq!(pressed("7"), Pressed::Key(Key::Digit(7)));
        assert_eq!(pressed(" "), Pressed::Key(Key::Space));
        assert_eq!(pressed(&text(Special::F5)), Pressed::Key(Key::Function(5)));
        assert_eq!(
            pressed(&text(Special::PageDown)),
            Pressed::Key(Key::PageDown)
        );
        assert_eq!(pressed(&text(Special::UpArrow)), Pressed::Key(Key::Up));
        for modifier in [
            Special::Shift,
            Special::Control,
            Special::Alt,
            Special::Meta,
        ] {
            assert_eq!(pressed(&text(modifier)), Pressed::Modifier);
        }
        for navigation in [
            Special::Tab,
            Special::Backtab,
            Special::Escape,
            Special::Return,
        ] {
            assert_eq!(pressed(&text(navigation)), Pressed::Navigation);
        }
        assert_eq!(
            pressed(";"),
            Pressed::Navigation,
            "punctuation is not a main key"
        );
        assert_eq!(pressed(""), Pressed::Navigation);
    }

    #[test]
    fn popup_bridge_maps_the_name_month_and_detail() {
        use crate::app::APP_NAME;
        use crate::app::month_view::{day_detail, month_view};
        use nepdate::{Lang, Month, NepaliDate};
        use slint::Model;

        assert_eq!(APP_NAME, "MitiBar");
        crate::test_support::init_platform();
        let popup = crate::PopupWindow::new().expect("popup component");

        set_app_name(&popup);
        assert_eq!(
            popup.global::<crate::AppInfo>().get_app_name().as_str(),
            APP_NAME
        );

        let today = NepaliDate::new(2083, 6, 1).expect("date");
        let view = month_view(2083, Month::Ashoj, today, Lang::En);
        let zoom = ZoomState::new(today);
        set_calendar(&popup, zoom, Some(&view), today, Lang::En);
        assert_eq!(
            popup.get_month_title().as_str(),
            "Ashoj 2083 · Sep/Oct 2026"
        );
        assert_eq!(popup.get_nav_title().as_str(), "Ashoj 2083 · Sep/Oct 2026");
        assert_eq!(popup.get_zoom_mode(), 0);
        assert_eq!(popup.get_cells().row_count(), view.cells.len());
        let first = popup.get_cells().row_data(4).expect("Ashoj 1");
        assert_eq!(first.bs_day.as_str(), "1");
        assert_eq!(first.ad_month.as_str(), "");
        assert_eq!(
            popup.get_cursor_index(),
            4,
            "the cursor is on today, Ashoj 1"
        );
        assert!(!popup.get_today_shown());
        assert!(popup.get_can_prev() && popup.get_can_next());

        let detail = day_detail(NepaliDate::new(2081, 1, 1).expect("date"), Lang::En);
        set_detail(&popup, Some(&detail));
        assert!(popup.get_detail_visible());
        assert_eq!(popup.get_detail().bs_long.as_str(), "Baishakh 1, 2081");
        assert_eq!(popup.get_detail().events.row_count(), 3);
        set_detail(&popup, None);
        assert!(!popup.get_detail_visible());
    }

    /// The Settings window's zone list is a few hundred short strings, not a memory cause.
    #[test]
    fn the_time_zone_list_is_small() {
        let zones = all_time_zones();
        let bytes: usize = zones.iter().map(String::capacity).sum();
        eprintln!("time zones: {} names, {} bytes", zones.len(), bytes);
        assert!(zones.len() > 100);
        assert!(bytes < 1_000_000, "{bytes} bytes");
    }
}
