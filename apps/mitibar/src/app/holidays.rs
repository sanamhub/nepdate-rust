//! Next-holiday search within the patro data (WR-03 AC8). Pure Rust, no Slint and no OS calls
//! (ADR-0015); the sentence around the result is built in `.slint` with `@tr`.

use nepdate::{Lang, NepaliDate};
use nepdate_patro as patro;

/// The next public holiday within 400 days (WR-03 AC8). `name` is the first event of that day, so
/// it is `None` when the holiday has no events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextHoliday {
    pub date: NepaliDate,
    pub name: Option<&'static str>,
    /// 0 for today, 1 for tomorrow, and so on.
    pub days_away: u32,
}

/// Finds the first public holiday on or after `from`, at most 400 days away. Holidays are
/// ascending, so the first one past the limit means nothing later can be within it.
#[must_use]
pub fn next_holiday(from: NepaliDate, lang: Lang) -> Option<NextHoliday> {
    for date in patro::public_holidays() {
        if date < from {
            continue;
        }
        let days_away = from.days_until(date);
        if days_away > 400 {
            return None;
        }
        return Some(NextHoliday {
            date,
            name: patro::info(date)
                .events()
                .next()
                .map(|event| event.name(lang)),
            days_away: u32::try_from(days_away).unwrap_or(0),
        });
    }
    None
}

/// The name of `date`'s public holiday: its first event, as `next_holiday` names a holiday. `None`
/// when the day is not a public holiday, or the holiday has no events.
#[must_use]
pub fn holiday_name(date: NepaliDate, lang: Lang) -> Option<&'static str> {
    let info = patro::info(date);
    if !info.is_public_holiday() {
        return None;
    }
    info.events().next().map(|event| event.name(lang))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: u16, month: u8, day: u8) -> NepaliDate {
        NepaliDate::new(year, month, day).expect("valid BS date")
    }

    #[test]
    fn holiday_name_is_none_on_an_ordinary_day() {
        assert_eq!(holiday_name(date(2081, 1, 1), Lang::En), Some("New Year"));
        assert_eq!(holiday_name(date(2081, 1, 2), Lang::En), None);
        assert_eq!(holiday_name(date(2090, 1, 1), Lang::En), None);
    }

    #[test]
    fn wr03_ac8_today() {
        let holiday = next_holiday(date(2081, 1, 1), Lang::En).expect("New Year");
        assert_eq!(holiday.date, date(2081, 1, 1));
        assert_eq!(holiday.days_away, 0);
        assert_eq!(holiday.name, Some("New Year"));
    }

    #[test]
    fn wr03_ac8_next_after_new_year() {
        let holiday = next_holiday(date(2081, 1, 2), Lang::En).expect("Ram Navami");
        assert_eq!(holiday.date, date(2081, 1, 5));
        assert_eq!(holiday.days_away, 3);
        assert_eq!(holiday.name, Some("Shree Ram Navami Brata"));
    }

    #[test]
    fn wr03_ac8_nepali_name() {
        let holiday = next_holiday(date(2081, 1, 1), Lang::Ne).expect("New Year");
        assert_eq!(holiday.name, Some("नयाँ वर्ष"));
    }

    #[test]
    fn wr03_ac8_four_hundred_day_limit() {
        // The first public holiday is 2077-01-01 (nepdate-patro doctest).
        let first_holiday = date(2077, 1, 1);
        let within = first_holiday
            .checked_add_days(-400)
            .expect("400 days before");
        let holiday = next_holiday(within, Lang::En).expect("exactly 400 days");
        assert_eq!(holiday.date, first_holiday);
        assert_eq!(holiday.days_away, 400);
        let beyond = first_holiday
            .checked_add_days(-401)
            .expect("401 days before");
        assert_eq!(next_holiday(beyond, Lang::En), None);
    }

    #[test]
    fn wr03_ac8_after_coverage_is_none() {
        assert_eq!(next_holiday(date(2084, 8, 19), Lang::En), None);
        assert_eq!(next_holiday(date(2090, 1, 1), Lang::En), None);
    }
}
