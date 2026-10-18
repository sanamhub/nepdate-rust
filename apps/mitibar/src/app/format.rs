//! AD date text for the UI. The month names stay Latin in both languages (FORMATTING §1), but
//! every number follows the app language's digit set (widget-design §4). Pure: no Slint and no
//! OS calls (ADR-0015).

use nepdate::{GregorianDate, Lang};

/// Full English AD month names, 1 to 12. The library has no Nepali AD month names, and the
/// design keeps AD dates in Latin, so both languages use these.
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The three-letter AD month names used by the month title and the taskbar text.
const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `YYYY-MM-DD` with the language's digits, for the header, the cell labels and the tray tooltip.
#[must_use]
pub fn ad_short(ad: GregorianDate, lang: Lang) -> String {
    format!(
        "{}-{}-{}",
        lang.digits(year(ad), 4),
        lang.digits(u32::from(ad.month()), 2),
        lang.digits(u32::from(ad.day()), 2)
    )
}

/// `April 13, 2024` with the language's digits, for the converter and the day detail.
#[must_use]
pub fn ad_long(ad: GregorianDate, lang: Lang) -> String {
    format!(
        "{} {}, {}",
        month_name(ad.month(), &MONTHS),
        lang.digits(u32::from(ad.day()), 1),
        lang.digits(year(ad), 1)
    )
}

/// `Oct 15, 2026` with the language's digits, for the Windows taskbar text.
#[must_use]
pub fn ad_day_month_year(ad: GregorianDate, lang: Lang) -> String {
    format!(
        "{} {}, {}",
        month_name(ad.month(), &MONTHS_SHORT),
        lang.digits(u32::from(ad.day()), 1),
        lang.digits(year(ad), 1)
    )
}

/// The AD year as a `u32` for the digit formatter. Supported AD dates are 1844 to 2143, so the
/// conversion cannot fail there; a bad value prints as year 0 rather than panicking.
fn year(ad: GregorianDate) -> u32 {
    u32::try_from(ad.year()).unwrap_or(0)
}

/// The month name at `month` (1 to 12); an out-of-range month prints as empty rather than
/// panicking.
fn month_name(month: u8, names: &'static [&'static str; 12]) -> &'static str {
    names
        .get(usize::from(month).wrapping_sub(1))
        .copied()
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u8, day: u8) -> GregorianDate {
        GregorianDate::new(year, month, day).expect("valid AD date")
    }

    #[test]
    fn ad_text_follows_the_language_digits() {
        let ad = date(2024, 4, 13);
        assert_eq!(ad_short(ad, Lang::En), "2024-04-13");
        assert_eq!(ad_short(ad, Lang::Ne), "२०२४-०४-१३");
        assert_eq!(ad_long(ad, Lang::En), "April 13, 2024");
        assert_eq!(ad_long(ad, Lang::Ne), "April १३, २०२४");
        assert_eq!(ad_day_month_year(ad, Lang::En), "Apr 13, 2024");
        assert_eq!(ad_day_month_year(ad, Lang::Ne), "Apr १३, २०२४");
    }

    #[test]
    fn every_month_has_a_name() {
        for month in 1..=12 {
            assert!(!month_name(month, &MONTHS).is_empty(), "{month}");
            assert!(!month_name(month, &MONTHS_SHORT).is_empty(), "{month}");
        }
        assert_eq!(month_name(0, &MONTHS), "");
        assert_eq!(month_name(13, &MONTHS_SHORT), "");
    }
}
