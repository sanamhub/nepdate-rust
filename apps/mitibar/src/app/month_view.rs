//! Month grid model: the title and the cells the grid renders (WR-03). Pure Rust, no Slint and no
//! OS calls (ADR-0015); patro data comes only through `nepdate_patro::info` (W2-01, W2-04).
// W2-01 is pure and unit-tested; W2-02 and W2-03 wire these items into the UI. Remove this allow
// once they are wired.
#![allow(dead_code)]

use nepdate::{Lang, Month, NepaliDate, Weekday};
use nepdate_patro as patro;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// `None` for the leading and trailing padding cells.
    pub date: Option<NepaliDate>,
    /// The BS day in `lang`'s digits (WR-03 AC2).
    pub bs_day: String,
    /// The AD day of month in `lang`'s digits (WR-03 AC2).
    pub ad_day: String,
    /// Set on the cell whose AD date is the 1st of a new AD month (WR-03 AC2).
    pub ad_month_marker: Option<&'static str>,
    pub is_today: bool,
    /// Saturday or a public holiday (WR-03 AC3).
    pub is_holiday_tint: bool,
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
    let mut seen_ad_month = first_ad.month();
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
        let changed_ad_month = ad.month() != seen_ad_month;
        if changed_ad_month {
            seen_ad_month = ad.month();
        }
        cells.push(Cell {
            date: Some(date),
            bs_day: lang.digits(u32::from(day), 1).to_string(),
            ad_day: lang.digits(u32::from(ad.day()), 1).to_string(),
            // WR-03 AC2: the marker is on day 1 of the new AD month, not on the month's first cell.
            ad_month_marker: (changed_ad_month && ad.day() == 1)
                .then_some(ad_month_name(ad.month())),
            is_today: date == today,
            is_holiday_tint: date.weekday() == Weekday::Saturday || info.is_public_holiday(),
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
        ad_range(first_ad, last.to_gregorian()),
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
            is_holiday_tint: false,
            tithi: None,
            has_events: false,
        }
    }
}

/// The AD month range for the title: `Sep 2026`, `Sep/Oct 2026`, or `Dec 2026/Jan 2027`.
fn ad_range(first: nepdate::GregorianDate, last: nepdate::GregorianDate) -> String {
    let first_name = ad_month_name(first.month());
    let last_name = ad_month_name(last.month());
    if first.year() == last.year() {
        if first.month() == last.month() {
            format!("{first_name} {}", first.year())
        } else {
            format!("{first_name}/{last_name} {}", first.year())
        }
    } else {
        format!("{first_name} {}/{last_name} {}", first.year(), last.year())
    }
}

/// `1..=12` to the three-letter AD month name. The `wrapping_sub` keeps a bad 0 from panicking.
fn ad_month_name(month: u8) -> &'static str {
    AD_MONTHS
        .get(usize::from(month).wrapping_sub(1))
        .copied()
        .unwrap_or("")
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
    fn wr03_ac3_saturday_and_holiday_tint() {
        // Republic Day 2081-02-15 is a Tuesday, so this checks the public-holiday branch on its
        // own; New Year 2081-01-01 would pass on the Saturday branch alone.
        let view = month_view(2081, Month::Jestha, date(2081, 2, 15), Lang::En);
        let holiday = date(2081, 2, 15);
        assert_ne!(holiday.weekday(), Weekday::Saturday);
        let holiday_cell = view
            .cells
            .iter()
            .find(|cell| cell.date == Some(holiday))
            .expect("Republic Day");
        assert!(
            holiday_cell.is_holiday_tint,
            "2081-02-15 is a public holiday"
        );
        let saturday = view
            .cells
            .iter()
            .find(|cell| {
                cell.date
                    .is_some_and(|date| date.weekday() == Weekday::Saturday)
            })
            .expect("a Saturday");
        assert!(saturday.is_holiday_tint);
        assert!(
            view.cells
                .iter()
                .any(|cell| cell.date.is_some() && !cell.is_holiday_tint),
            "a plain weekday cell"
        );
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
        assert_eq!(ne.title, "असोज २०८३ · Sep/Oct 2026");
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
}
