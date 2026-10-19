//! Month grid model: the title and the cells the grid renders (WR-03). Pure Rust, no Slint and no
//! OS calls (ADR-0015); patro data comes only through `nepdate_patro::info`.

use nepdate::{DateOrder, GregorianDate, Lang, LongDate, Month, NepaliDate, Separator, Weekday};
use nepdate_patro as patro;

use crate::app::format;

/// English AD month abbreviations. The AD side of a title stays Latin in both languages
/// (FORMATTING §1, WR-03 AC5); the BS side follows `lang`.
const AD_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// One BS month as the grid shows it (WR-03). `cells` always has 35 or 42 entries: the leading
/// padding, the month's days, then the trailing padding to a whole week.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthView {
    pub year: u16,
    pub month: Month,
    /// WR-03 AC5: `Ashoj 2083 · Sep/Oct 2026` (the BS side is Nepali for `Lang::Ne`).
    pub title: String,
    /// Sunday first (WR-03 AC1).
    pub weekday_headers: [&'static str; 7],
    pub cells: Vec<Cell>,
    pub can_prev: bool,
    pub can_next: bool,
}

/// One grid cell. Padding cells have no date and no content.
// The flags are independent calendar facts, not a state machine: a day can be today, a Saturday
// and a public holiday at once, so separate bools read better than an enum here.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// `None` for the leading and trailing padding cells.
    pub date: Option<NepaliDate>,
    /// The BS day in `lang`'s digits (WR-03 AC2).
    pub bs_day: String,
    /// The AD day of month in `lang`'s digits (WR-03 AC2).
    pub ad_day: String,
    /// Set on every cell whose AD date is the 1st. The UI shows this short month name in place of
    /// the AD day, so day 1 is implied (WR-03 AC2).
    pub ad_month_marker: Option<&'static str>,
    pub is_today: bool,
    /// The date falls on a Saturday (WR-03 AC3).
    pub is_saturday: bool,
    /// A public holiday from `nepdate-patro`. The UI gives this colour priority over the
    /// Saturday colour, so a holiday that falls on a Saturday stays a holiday (WR-03 AC3).
    pub is_public_holiday: bool,
    /// Tithi name in `lang`, or `None` outside the patro coverage (WR-03 AC4).
    pub tithi: Option<&'static str>,
    pub has_events: bool,
}

/// Builds one month for the grid. The date changed while hidden handling (WR-02 AC2) and the
/// range edges (WR-03 AC6) are the caller's job; this function never panics and returns an empty
/// view for a year outside 1901..=2199.
#[must_use]
pub fn month_view(year: u16, month: Month, today: NepaliDate, lang: Lang) -> MonthView {
    let weekday_headers = Weekday::ALL.map(|w| w.short_name(lang));
    let Ok(first) = NepaliDate::new(year, month.number(), 1) else {
        return MonthView {
            year,
            month,
            title: String::new(),
            weekday_headers,
            cells: Vec::new(),
            can_prev: false,
            can_next: false,
        };
    };
    let month_len = usize::from(first.month_length());
    let lead = usize::from(first.weekday().index_from_sunday());
    let total = (lead + month_len).div_ceil(7) * 7;

    let first_ad = first.to_gregorian();
    let mut cells = Vec::with_capacity(total);
    for _ in 0..lead {
        cells.push(Cell::padding());
    }
    for day in 1..=first.month_length() {
        let Ok(date) = NepaliDate::new(year, month.number(), day) else {
            continue; // `day` is inside the month length, so this never happens.
        };
        let info = patro::info(date);
        let ad = date.to_gregorian();
        cells.push(Cell {
            date: Some(date),
            bs_day: lang.digits(u32::from(day), 1).to_string(),
            ad_day: lang.digits(u32::from(ad.day()), 1).to_string(),
            // WR-03 AC2: every AD 1st carries the short month; the UI shows it in place of the
            // AD day.
            ad_month_marker: ad_marker(ad),
            is_today: date == today,
            is_saturday: date.weekday() == Weekday::Saturday,
            is_public_holiday: info.is_public_holiday(),
            tithi: info.tithi().map(|t| t.name(lang)),
            has_events: info.has_events(),
        });
    }
    while cells.len() < total {
        cells.push(Cell::padding());
    }

    let last = NepaliDate::new(year, month.number(), first.month_length()).unwrap_or(first);
    let title = format!(
        "{} {} · {}",
        month.name(lang),
        lang.digits(u32::from(year), 1),
        ad_range(first_ad, last.to_gregorian(), lang),
    );

    MonthView {
        year,
        month,
        title,
        weekday_headers,
        cells,
        can_prev: !(year == 1901 && month == Month::Baishakh),
        can_next: !(year == 2199 && month == Month::Chaitra),
    }
}

impl Cell {
    fn padding() -> Self {
        Cell {
            date: None,
            bs_day: String::new(),
            ad_day: String::new(),
            ad_month_marker: None,
            is_today: false,
            is_saturday: false,
            is_public_holiday: false,
            tithi: None,
            has_events: false,
        }
    }
}

/// The AD month range for the title: `Sep 2026`, `Sep/Oct 2026`, or `Dec 2026/Jan 2027`. The
/// month names stay Latin; the years follow the language's digits (widget-design §4).
fn ad_range(first: GregorianDate, last: GregorianDate, lang: Lang) -> String {
    let first_name = ad_month_name(first.month());
    let last_name = ad_month_name(last.month());
    let first_year = lang.digits(u32::try_from(first.year()).unwrap_or(0), 1);
    let last_year = lang.digits(u32::try_from(last.year()).unwrap_or(0), 1);
    if first.year() == last.year() {
        if first.month() == last.month() {
            format!("{first_name} {first_year}")
        } else {
            format!("{first_name}/{last_name} {first_year}")
        }
    } else {
        format!("{first_name} {first_year}/{last_name} {last_year}")
    }
}

/// `1..=12` to the three-letter AD month name. The `wrapping_sub` keeps a bad 0 from panicking.
fn ad_month_name(month: u8) -> &'static str {
    AD_MONTHS
        .get(usize::from(month).wrapping_sub(1))
        .copied()
        .unwrap_or("")
}

/// The short AD month for a cell whose AD date is the 1st, `None` otherwise (WR-03 AC2). The UI
/// shows it in place of the day number; the rule does not depend on the cell's position.
fn ad_marker(ad: GregorianDate) -> Option<&'static str> {
    (ad.day() == 1).then_some(ad_month_name(ad.month()))
}

/// Steps one month forward (`delta > 0`) or back (`delta < 0`), stopping at 1901 Baishakh and
/// 2199 Chaitra (WR-03 AC6). `None` means "no move", not an error.
#[must_use]
pub fn step_month(year: u16, month: Month, delta: i32) -> Option<(u16, Month)> {
    match delta.cmp(&0) {
        std::cmp::Ordering::Less => {
            if year == 1901 && month == Month::Baishakh {
                return None;
            }
            if month == Month::Baishakh {
                Some((year - 1, Month::Chaitra))
            } else {
                Some((year, month.prev()))
            }
        }
        std::cmp::Ordering::Greater => {
            if year == 2199 && month == Month::Chaitra {
                return None;
            }
            if month == Month::Chaitra {
                Some((year + 1, Month::Baishakh))
            } else {
                Some((year, month.next()))
            }
        }
        std::cmp::Ordering::Equal => None,
    }
}

/// Maps a grid index back to its date, or `None` for a padding cell. The index is the
/// `cells` position the repeater hands to `day-clicked`.
#[must_use]
pub fn date_at(year: u16, month: Month, index: i32) -> Option<NepaliDate> {
    let first = NepaliDate::new(year, month.number(), 1).ok()?;
    let lead = i32::from(first.weekday().index_from_sunday());
    let day = index.checked_sub(lead)?.checked_add(1)?;
    if day < 1 || day > i32::from(first.month_length()) {
        return None;
    }
    let day = u8::try_from(day).ok()?;
    NepaliDate::new(year, month.number(), day).ok()
}

/// The month to show on reopen (WR-02 AC2): the last viewed month while the date is unchanged,
/// today's month when the date moved on while the popup was hidden.
#[must_use]
pub fn month_on_show(
    viewed: (u16, Month),
    hidden_on: NepaliDate,
    today: NepaliDate,
) -> (u16, Month) {
    if today == hidden_on {
        viewed
    } else {
        (today.year(), today.month())
    }
}

/// The day detail (WR-04): the long BS and AD dates, weekday, tithi, holiday flag and events,
/// plus the two clipboard strings (WR-04 AC2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayDetail {
    pub bs_long: String,
    pub ad_long: String,
    pub weekday: String,
    pub tithi: Option<&'static str>,
    pub is_holiday: bool,
    pub events: Vec<&'static str>,
    pub copy_bs: String,
    pub copy_ad: String,
}

/// Builds the day detail for `date`. Outside the patro coverage the tithi and events are empty
/// and the holiday flag is false (WR-03 AC4), and nothing panics.
#[must_use]
pub fn day_detail(date: NepaliDate, lang: Lang) -> DayDetail {
    let info = patro::info(date);
    let ad = date.to_gregorian();
    DayDetail {
        bs_long: date
            .long(
                LongDate {
                    weekday: false,
                    year: true,
                    pad: false,
                },
                lang,
            )
            .to_string(),
        ad_long: format::ad_long(ad, lang),
        weekday: date.weekday().name(lang).to_owned(),
        tithi: info.tithi().map(|t| t.name(lang)),
        is_holiday: info.is_public_holiday(),
        events: info.events().map(|e| e.name(lang)).collect(),
        copy_bs: date
            .short(DateOrder::Ymd, Separator::Slash, true, lang)
            .to_string(),
        copy_ad: format::ad_short(ad, lang),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: u16, month: u8, day: u8) -> NepaliDate {
        NepaliDate::new(year, month, day).expect("valid BS date")
    }

    #[test]
    fn wr03_ac1_rows() {
        for year in 2080..=2084 {
            for month in Month::ALL {
                let view = month_view(year, month, date(2081, 1, 1), Lang::En);
                assert!(
                    view.cells.len() == 35 || view.cells.len() == 42,
                    "{year} {month:?}: {} cells",
                    view.cells.len()
                );
                let first = date(year, month.number(), 1);
                let lead = usize::from(first.weekday().index_from_sunday());
                let padding = view
                    .cells
                    .iter()
                    .take_while(|cell| cell.date.is_none())
                    .count();
                assert_eq!(padding, lead, "{year} {month:?} leading padding");
                let month_len = usize::from(first.month_length());
                let trailing = view
                    .cells
                    .iter()
                    .rev()
                    .take_while(|cell| cell.date.is_none())
                    .count();
                assert_eq!(
                    view.cells.len(),
                    lead + month_len + trailing,
                    "{year} {month:?} cell count"
                );
            }
        }
    }

    #[test]
    fn wr03_ac2_ad_month_marker() {
        // 2083 Ashoj (month 6) 1 is 2026-09-17; Ashoj 15 is 2026-10-01, the first cell of a new
        // AD month.
        let view = month_view(2083, Month::Ashoj, date(2083, 6, 1), Lang::En);
        let first = view
            .cells
            .iter()
            .find(|cell| cell.date == Some(date(2083, 6, 1)))
            .expect("Ashoj 1");
        assert_eq!(first.ad_month_marker, None);
        let october = view
            .cells
            .iter()
            .find(|cell| cell.date == Some(date(2083, 6, 15)))
            .expect("Ashoj 15");
        assert_eq!(october.ad_day, "1");
        assert_eq!(october.ad_month_marker, Some("Oct"));
        assert_eq!(
            view.cells
                .iter()
                .filter(|cell| cell.ad_month_marker.is_some())
                .count(),
            1
        );
    }

    #[test]
    fn wr03_ac2_ad_month_marker_on_every_first() {
        // The rule itself: an AD 1st carries the short month, whatever its position in the grid.
        // The UI shows it in place of the day number, so day 1 is implied.
        assert_eq!(
            ad_marker(GregorianDate::new(2024, 4, 1).expect("AD date")),
            Some("Apr")
        );
        assert_eq!(
            ad_marker(GregorianDate::new(2024, 4, 2).expect("AD date")),
            None
        );

        // No BS month in 1901..=2199 starts on an AD 1st: the first day is always the 10th to
        // the 20th of an AD month. The old code special-cased that first cell, so the branch
        // was unreachable; this assertion records the data fact and fails if the data changes.
        let mut starts_on_first = Vec::new();
        for year in 1901..=2199 {
            for month in Month::ALL {
                if NepaliDate::new(year, month.number(), 1)
                    .is_ok_and(|first| first.to_gregorian().day() == 1)
                {
                    starts_on_first.push((year, month));
                }
            }
        }
        assert!(
            starts_on_first.is_empty(),
            "a BS month now starts on an AD 1st: {starts_on_first:?}"
        );

        // Every cell whose AD day is 1 has the marker, and no other cell does.
        for year in 1901..=2199 {
            for month in Month::ALL {
                let view = month_view(year, month, date(year, month.number(), 1), Lang::En);
                for cell in &view.cells {
                    let Some(bs) = cell.date else {
                        continue;
                    };
                    let ad = bs.to_gregorian();
                    assert_eq!(cell.ad_month_marker.is_some(), ad.day() == 1, "{bs} / {ad}");
                }
            }
        }
    }

    #[test]
    fn wr03_ac3_saturday_and_holiday_tint() {
        // New Year 2081-01-01 is a Saturday and a public holiday. Both flags stay set; the UI
        // gives the holiday colour priority, so the cell is red, not slate.
        let new_year = month_view(2081, Month::Baishakh, date(2081, 1, 1), Lang::En);
        let new_year_cell = new_year
            .cells
            .iter()
            .find(|cell| cell.date == Some(date(2081, 1, 1)))
            .expect("New Year");
        assert_eq!(date(2081, 1, 1).weekday(), Weekday::Saturday);
        assert!(new_year_cell.is_saturday);
        assert!(new_year_cell.is_public_holiday);

        // Republic Day 2081-02-15 is a Tuesday: a holiday without the Saturday flag.
        let view = month_view(2081, Month::Jestha, date(2081, 2, 15), Lang::En);
        let holiday = date(2081, 2, 15);
        let holiday_cell = view
            .cells
            .iter()
            .find(|cell| cell.date == Some(holiday))
            .expect("Republic Day");
        assert_ne!(holiday.weekday(), Weekday::Saturday);
        assert!(holiday_cell.is_public_holiday);
        assert!(!holiday_cell.is_saturday);

        // A plain Saturday is Saturday only, and a plain weekday is neither.
        let plain_saturday = view
            .cells
            .iter()
            .find(|cell| {
                cell.date
                    .is_some_and(|date| date.weekday() == Weekday::Saturday)
                    && !cell.is_public_holiday
            })
            .expect("a plain Saturday");
        assert!(plain_saturday.is_saturday);
        let plain_weekday = view
            .cells
            .iter()
            .find(|cell| {
                cell.date.is_some_and(|date| {
                    date.weekday() != Weekday::Saturday && !cell.is_public_holiday
                })
            })
            .expect("a plain weekday");
        assert!(!plain_weekday.is_saturday && !plain_weekday.is_public_holiday);
    }

    #[test]
    fn wr03_ac4_no_event_data_outside_coverage() {
        let outside = month_view(2090, Month::Baishakh, date(2090, 1, 1), Lang::En);
        assert!(
            outside
                .cells
                .iter()
                .all(|cell| { cell.date.is_none() || (cell.tithi.is_none() && !cell.has_events) })
        );
        let inside = month_view(2081, Month::Baishakh, date(2081, 1, 1), Lang::En);
        assert!(inside.cells.iter().any(|cell| cell.tithi.is_some()));
        assert!(inside.cells.iter().any(|cell| cell.has_events));
    }

    #[test]
    fn wr03_ac5_title_en_ne() {
        let en = month_view(2083, Month::Ashoj, date(2083, 7, 1), Lang::En);
        assert_eq!(en.title, "Ashoj 2083 · Sep/Oct 2026");
        let ne = month_view(2083, Month::Ashoj, date(2083, 7, 1), Lang::Ne);
        assert_eq!(ne.title, "असोज २०८३ · Sep/Oct २०२६");
    }

    #[test]
    fn wr03_ac6_edges() {
        let start = month_view(1901, Month::Baishakh, date(1901, 1, 1), Lang::En);
        assert!(!start.can_prev);
        assert!(start.can_next);
        let end = month_view(2199, Month::Chaitra, date(2199, 12, 1), Lang::En);
        assert!(end.can_prev);
        assert!(!end.can_next);
        assert!(month_view(1901, Month::Jestha, date(1901, 1, 1), Lang::En).can_prev);
        assert!(month_view(2199, Month::Mangsir, date(2199, 12, 1), Lang::En).can_next);
    }

    #[test]
    fn wr03_ac1_weekday_headers_sunday_first() {
        let en = month_view(2081, Month::Baishakh, date(2081, 1, 1), Lang::En);
        assert_eq!(en.weekday_headers[0], "Sun");
        assert_eq!(en.weekday_headers[6], "Sat");
        let ne = month_view(2081, Month::Baishakh, date(2081, 1, 1), Lang::Ne);
        assert_eq!(ne.weekday_headers[0], "आइत");
        assert_eq!(ne.weekday_headers[6], "शनि");
    }

    #[test]
    fn wr03_ac2_day_digits_follow_the_language() {
        let en = month_view(2083, Month::Ashoj, date(2083, 6, 1), Lang::En);
        let first = &en.cells[usize::from(date(2083, 6, 1).weekday().index_from_sunday())];
        assert_eq!(first.bs_day, "1");
        assert_eq!(first.ad_day, "17");
        let ne = month_view(2083, Month::Ashoj, date(2083, 6, 1), Lang::Ne);
        let first = &ne.cells[usize::from(date(2083, 6, 1).weekday().index_from_sunday())];
        assert_eq!(first.bs_day, "१");
        assert_eq!(first.ad_day, "१७");
    }

    #[test]
    fn month_view_rejects_years_outside_the_range() {
        let view = month_view(1900, Month::Baishakh, date(1901, 1, 1), Lang::En);
        assert!(view.cells.is_empty());
        assert!(!view.can_prev && !view.can_next);
    }

    #[test]
    fn wr03_ac6_step_month_stops_at_the_edges() {
        assert_eq!(step_month(1901, Month::Baishakh, -1), None);
        assert_eq!(
            step_month(1901, Month::Baishakh, 1),
            Some((1901, Month::Jestha))
        );
        assert_eq!(step_month(2199, Month::Chaitra, 1), None);
        assert_eq!(
            step_month(2199, Month::Chaitra, -1),
            Some((2199, Month::Falgun))
        );
        assert_eq!(
            step_month(2081, Month::Baishakh, -1),
            Some((2080, Month::Chaitra))
        );
        assert_eq!(
            step_month(2081, Month::Chaitra, 1),
            Some((2082, Month::Baishakh))
        );
        assert_eq!(step_month(2081, Month::Ashoj, 0), None);
    }

    #[test]
    fn date_at_maps_the_grid_index() {
        let first = date(2083, 6, 1);
        let lead = i32::from(first.weekday().index_from_sunday());
        let last = date(2083, 6, first.month_length());
        assert_eq!(date_at(2083, Month::Ashoj, lead), Some(first));
        assert_eq!(date_at(2083, Month::Ashoj, lead - 1), None);
        assert_eq!(
            date_at(
                2083,
                Month::Ashoj,
                lead + i32::from(first.month_length()) - 1
            ),
            Some(last)
        );
        assert_eq!(
            date_at(2083, Month::Ashoj, lead + i32::from(first.month_length())),
            None
        );
    }

    #[test]
    fn wr04_ac1_day_detail_en() {
        let detail = day_detail(date(2081, 1, 1), Lang::En);
        assert_eq!(detail.bs_long, "Baishakh 1, 2081");
        assert_eq!(detail.ad_long, "April 13, 2024");
        assert_eq!(detail.weekday, "Saturday");
        assert_eq!(detail.tithi, Some("Panchami"));
        assert!(detail.is_holiday);
        assert_eq!(
            detail.events,
            ["New Year", "Mesh Sankranti", "Bisket Jatra"]
        );
    }

    #[test]
    fn wr04_ac1_day_detail_ne() {
        let detail = day_detail(date(2081, 1, 1), Lang::Ne);
        assert_eq!(detail.bs_long, "बैशाख १, २०८१");
        assert_eq!(detail.weekday, "शनिवार");
        assert_eq!(detail.tithi, Some("पञ्चमी"));
        assert_eq!(detail.events.first(), Some(&"नयाँ वर्ष"));
        // The AD month name stays Latin (the design keeps AD dates in Latin), but its numbers
        // follow the language's digit set (widget-design §4).
        assert_eq!(detail.ad_long, "April १३, २०२४");
    }

    #[test]
    fn wr04_ac2_copy_formats() {
        let detail = day_detail(date(2081, 1, 1), Lang::En);
        assert_eq!(detail.copy_bs, "2081/01/01");
        assert_eq!(detail.copy_ad, "2024-04-13");

        // Copy AD follows the language's digits, like Copy BS.
        let detail = day_detail(date(2081, 1, 1), Lang::Ne);
        assert_eq!(detail.copy_bs, "२०८१/०१/०१");
        assert_eq!(detail.copy_ad, "२०२४-०४-१३");
    }

    #[test]
    fn wr04_ac1_day_detail_outside_coverage() {
        let detail = day_detail(date(2090, 1, 1), Lang::En);
        assert_eq!(detail.tithi, None);
        assert!(!detail.is_holiday);
        assert!(detail.events.is_empty());
    }

    #[test]
    fn wr02_ac2_reopen_keeps_or_resets_the_month() {
        let viewed = (2080, Month::Chaitra);
        let hidden_on = date(2081, 1, 5);
        assert_eq!(month_on_show(viewed, hidden_on, hidden_on), viewed);
        assert_eq!(
            month_on_show(viewed, hidden_on, date(2081, 1, 6)),
            (2081, Month::Baishakh)
        );
    }
}
