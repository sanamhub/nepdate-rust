//! Layout tests for the popup (W8-4a): the month grid's day cells and the icon-and-text controls.
//! They read the boxes the layout gives the numeral, the AD day, the event dot and the tithi, so a
//! change that makes two numbers touch, moves the dot, clips a tithi name or leaves a band under
//! the grid fails here; and the boxes of icons and captions, so an icon that drifts off the middle
//! of its pill fails too. The testing backend measures text with the bundled fonts, like the app.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use i_slint_backend_testing::ElementHandle;
use nepdate::{Month, NepaliDate};
use nepdate_patro as patro;
use slint::Model;

use crate::interaction_tests::{app, date, show_month};
use crate::{AppState, PopupWindow, update_popup};

/// A laid-out box in window coordinates.
#[derive(Debug, Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn of(element: &ElementHandle) -> Self {
        let position = element.absolute_position();
        let size = element.size();
        Rect {
            x: position.x,
            y: position.y,
            w: size.width,
            h: size.height,
        }
    }

    fn right(self) -> f32 {
        self.x + self.w
    }

    fn bottom(self) -> f32 {
        self.y + self.h
    }

    /// Whether the top-left corner of `part` is in this box.
    fn holds(self, part: Rect) -> bool {
        part.x >= self.x && part.x < self.right() && part.y >= self.y && part.y < self.bottom()
    }
}

/// One real day of the shown month and the boxes of its parts.
struct Day {
    /// The BS day and the AD slot's text, for messages.
    name: String,
    cell: Rect,
    numeral: Rect,
    ad: Rect,
    dot: Option<Rect>,
    tithi: Option<(String, Rect)>,
    is_today: bool,
    has_marker: bool,
}

fn boxes(popup: &PopupWindow, id: &str) -> Vec<(String, Rect)> {
    ElementHandle::find_by_element_id(popup, id)
        .map(|element| {
            (
                element.accessible_label().unwrap_or_default().to_string(),
                Rect::of(&element),
            )
        })
        .collect()
}

/// Every real day of the shown month, in order, with its parts found inside its cell.
fn days(popup: &PopupWindow) -> Vec<Day> {
    let cells: Vec<Rect> = ElementHandle::find_by_element_type_name(popup, "DayCell")
        .map(|element| Rect::of(&element))
        .collect();
    let numerals = boxes(popup, "DayCell::numeral");
    let ads = boxes(popup, "DayCell::ad-text");
    let dots = boxes(popup, "DayCell::event-dot");
    let tithis = boxes(popup, "DayCell::tithi-text");
    let model = popup.get_cells();
    assert_eq!(cells.len(), model.row_count(), "one DayCell per model row");

    let mut days = Vec::new();
    for (index, cell) in cells.iter().enumerate() {
        let data = model.row_data(index).expect("a cell row");
        if data.is_padding {
            continue;
        }
        let inside = |parts: &[(String, Rect)]| {
            parts
                .iter()
                .find(|(_, rect)| cell.holds(*rect))
                .cloned()
                .expect("the part is inside its cell")
        };
        days.push(Day {
            name: format!(
                "{} / {}",
                data.bs_day,
                if data.ad_month.is_empty() {
                    &data.ad_day
                } else {
                    &data.ad_month
                }
            ),
            cell: *cell,
            numeral: inside(&numerals).1,
            ad: inside(&ads).1,
            dot: data.has_events.then(|| inside(&dots).1),
            tithi: (!data.tithi.is_empty()).then(|| (data.tithi.to_string(), inside(&tithis).1)),
            is_today: data.is_today,
            has_marker: !data.ad_month.is_empty() && !data.is_today,
        });
    }
    days
}

fn set_language(state: &Rc<RefCell<AppState>>, lang: &str) {
    state.borrow_mut().settings.language = lang.to_owned();
    let _ = slint::select_bundled_translation(lang);
    update_popup(state);
}

/// Every month of 2083 in `lang`, with the days of each.
fn every_day_of_2083(lang: &str) -> Vec<(Month, Day)> {
    let (popup, state, _) = app(date(2083, 7, 5));
    set_language(&state, lang);
    let mut all = Vec::new();
    for month in Month::ALL {
        show_month(&state, 2083, month);
        all.extend(days(&popup).into_iter().map(|day| (month, day)));
    }
    set_language(&state, "en");
    all
}

// WR-03 AC2: the BS day and the AD day.

#[test]
fn w8_4a_the_bs_day_and_the_ad_day_keep_a_clear_gap() {
    for lang in ["en", "ne"] {
        for (month, day) in every_day_of_2083(lang) {
            let gap = day.ad.x - day.numeral.right();
            // The month marker is the widest thing in the AD slot and follows a two-digit day, so
            // it gets the smaller floor; two digits get more room.
            let floor = if day.has_marker { 2.0 } else { 4.0 };
            assert!(
                gap >= floor,
                "{lang} {month:?} {}: the numbers are {gap:.2} px apart, at least {floor} wanted",
                day.name
            );
        }
    }
}

#[test]
fn w8_4a_the_numeral_is_centred_and_the_ad_day_ends_at_the_same_inset_in_every_cell() {
    for (month, day) in every_day_of_2083("en") {
        let centre = day.numeral.x + day.numeral.w / 2.0 - day.cell.x;
        assert!(
            (centre - 14.0).abs() < 0.01,
            "{month:?}: numeral centre {centre}"
        );
        let inset = day.cell.right() - day.ad.right();
        assert!((inset - 1.5).abs() < 0.01, "{month:?}: AD inset {inset}");
    }
}

#[test]
fn w8_4a_todays_cell_keeps_the_ad_day_where_the_month_would_overlap_the_disc() {
    // Kartik 15, 2083 is Nov 1, 2026: an AD 1st, so a cell shows "Nov" for the day, but not today's.
    let (popup, state, _) = app(date(2083, 7, 15));
    let first = |popup: &PopupWindow| {
        days(popup)
            .into_iter()
            .find(|day| day.name.starts_with("15 /"))
            .expect("the 15th")
    };
    let today = first(&popup);
    assert!(today.is_today);
    assert!(
        today.ad.w < 12.0,
        "today shows the AD day: {} px",
        today.ad.w
    );
    state.borrow_mut().today_override = Some(date(2083, 7, 16));
    update_popup(&state);
    let other = first(&popup);
    assert!(!other.is_today);
    assert!(
        other.ad.w > 15.0,
        "another day shows the month: {} px",
        other.ad.w
    );
}

// WR-03 AC4: the event dot.

#[test]
fn w8_4a_the_event_dot_is_the_same_distance_from_one_and_two_digit_days() {
    let mut one_digit = 0;
    let mut two_digits = 0;
    for (month, day) in every_day_of_2083("en") {
        let Some(dot) = day.dot else { continue };
        if day.is_today {
            continue;
        }
        let after_numeral = dot.x - day.numeral.right();
        assert!(
            (after_numeral - 1.0).abs() < 0.01,
            "{month:?}: the dot is {after_numeral} px after the number"
        );
        assert!(
            dot.y > day.cell.y && dot.bottom() < day.numeral.bottom(),
            "{month:?}: the dot is level with the number"
        );
        if day.numeral.w < 12.0 {
            one_digit += 1;
        } else {
            two_digits += 1;
        }
    }
    assert!(
        one_digit > 5 && two_digits > 5,
        "{one_digit} one-digit and {two_digits} two-digit days with events"
    );
}

#[test]
fn w8_4a_todays_dot_clears_the_disc() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    let today = days(&popup)
        .into_iter()
        .find(|day| day.is_today)
        .expect("today is in the shown month");
    let dot = today.dot.expect("2083-07-05 has an event");
    let disc_right = today.cell.x + 14.0 + 14.0;
    assert!(
        dot.x >= disc_right - 0.01,
        "the dot at {} is over the disc",
        dot.x
    );
}

// WR-03 AC4: the tithi.

/// Every month of the patro's coverage (92 have data) in `lang`: every name must fit its cell, whole.
fn assert_every_tithi_fits(lang: &str) {
    let (popup, state, _) = app(date(2083, 7, 5));
    set_language(&state, lang);
    let coverage = patro::coverage();
    let mut seen = BTreeSet::new();
    for year in coverage.start().year()..=coverage.end().year() {
        for month in Month::ALL {
            if NepaliDate::new(year, month.number(), 1).is_err() {
                continue;
            }
            show_month(&state, year, month);
            for day in days(&popup) {
                let Some((text, tithi)) = day.tithi else {
                    continue;
                };
                seen.insert(text.clone());
                assert!(
                    tithi.w <= day.cell.w - 4.0,
                    "{lang} {year} {month:?}: {text:?} is {:.1} px wide in a {:.1} px cell",
                    tithi.w,
                    day.cell.w
                );
                assert!(
                    tithi.bottom() <= day.cell.bottom() + 0.5,
                    "{lang} {year} {month:?}: {text:?} ends {:.1} px below its cell",
                    tithi.bottom() - day.cell.bottom()
                );
                assert!(
                    tithi.y >= day.numeral.bottom(),
                    "{lang} {year} {month:?}: {text:?} starts inside the numeral's box"
                );
            }
        }
    }
    set_language(&state, "en");
    assert_eq!(
        seen.len(),
        32,
        "{lang}: every name of the table was on screen"
    );
}

#[test]
fn w8_4a_every_latin_tithi_fits_its_cell_whole() {
    assert_every_tithi_fits("en");
}

#[test]
fn w8_4a_every_devanagari_tithi_fits_its_cell_whole() {
    assert_every_tithi_fits("ne");
}

// WR-02 AC1: the grid fills its area.

#[test]
fn w8_4a_a_five_row_and_a_six_row_month_both_end_at_the_holiday_line() {
    let (popup, state, _) = app(date(2083, 7, 5));
    let holiday = ElementHandle::find_by_accessible_label(&popup, "Upcoming holidays")
        .next()
        .map(|element| Rect::of(&element))
        .expect("the holiday line is shown");
    let mut heights = Vec::new();
    // Kartik 2083 has five rows, Falgun 2083 six (its first day is a Saturday).
    for (month, rows) in [(Month::Kartik, 5), (Month::Falgun, 6)] {
        show_month(&state, 2083, month);
        assert_eq!(
            popup.get_cells().row_count(),
            rows * 7,
            "{month:?} has {rows} rows"
        );
        let days = days(&popup);
        let last = days.iter().map(|day| day.cell.bottom()).fold(0.0, f32::max);
        let band = holiday.y - last;
        assert!(
            (0.0..=4.5).contains(&band),
            "{month:?}: {band:.1} px between the last row and the holiday line"
        );
        let height = days[0].cell.h;
        assert!(height >= 52.0, "{month:?}: cells are {height} px high");
        heights.push(height);
    }
    assert!(
        heights[0] > heights[1] + 5.0,
        "the five-row month's cells are taller: {heights:?}"
    );
}

// Icons and captions.

/// The boxes of the `LineIcon`s and the texts that sit inside `container`.
fn parts_in(popup: &PopupWindow, container: Rect) -> (Vec<Rect>, Vec<Rect>) {
    let inside = |type_name: &str| {
        ElementHandle::find_by_element_type_name(popup, type_name)
            .map(|element| Rect::of(&element))
            .filter(|rect| container.holds(*rect))
            .collect::<Vec<_>>()
    };
    (inside("LineIcon"), inside("Text"))
}

fn pill(popup: &PopupWindow, label: &str) -> Rect {
    let found = ElementHandle::find_by_accessible_label(popup, label)
        .next()
        .unwrap_or_else(|| panic!("no element labelled {label:?}"));
    Rect::of(&found)
}

#[test]
fn w8_4a_the_icon_of_an_icon_only_pill_is_in_the_middle() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    for label in [
        "Settings",
        "Pin, not pinned",
        "Previous month",
        "Next month",
    ] {
        let pill = pill(&popup, label);
        let (icons, _) = parts_in(&popup, pill);
        assert_eq!(icons.len(), 1, "{label}");
        let icon = icons[0];
        let dx = icon.x + icon.w / 2.0 - (pill.x + pill.w / 2.0);
        let dy = icon.y + icon.h / 2.0 - (pill.y + pill.h / 2.0);
        assert!(
            dx.abs() < 0.01 && dy.abs() < 0.01,
            "{label}: the icon is {dx} px right and {dy} px below the middle"
        );
    }
}

#[test]
fn w8_4a_a_pill_with_an_icon_and_text_has_the_same_air_on_both_sides_of_the_ink() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    let pill = pill(&popup, "Convert");
    let (icons, texts) = parts_in(&popup, pill);
    let (icon, text) = (icons[0], texts[0]);
    // The icon's drawing leaves 2 px of air inside its 16 px box, so its box starts 2 px closer
    // to the edge than the text's end.
    let left = icon.x - pill.x;
    let right = pill.right() - text.right();
    assert!(
        (right - left - 2.0).abs() < 0.01,
        "the icon box is {left} px from the edge and the text ends {right} px from the other"
    );
    let gap = text.x - icon.right();
    assert!((gap - 6.0).abs() < 0.01, "icon and text are {gap} px apart");
    let dy = icon.y + icon.h / 2.0 - (text.y + text.h / 2.0);
    assert!(dy.abs() < 0.01, "the icon is {dy} px off the text's middle");
}

#[test]
fn w8_4a_the_holiday_line_chevron_is_level_with_its_text() {
    let (popup, _state, _) = app(date(2083, 7, 5));
    let line = pill(&popup, "Upcoming holidays");
    let (icons, texts) = parts_in(&popup, line);
    let (icon, text) = (icons[0], texts[0]);
    let dy = icon.y + icon.h / 2.0 - (text.y + text.h / 2.0);
    assert!(
        dy.abs() < 0.01,
        "the chevron is {dy} px off the text's middle"
    );
    let dline = icon.y + icon.h / 2.0 - (line.y + line.h / 2.0);
    assert!(
        dline.abs() < 0.01,
        "the chevron is {dline} px off the line's middle"
    );
}
