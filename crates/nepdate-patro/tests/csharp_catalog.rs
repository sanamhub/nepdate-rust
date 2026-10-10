//! C# `NepDate`'s own test suite, phase 4 rows of `shared/spec/vectors/csharp-tests/catalog.tsv`
//! (L5-07). Names come from `cargo xtask catalog-check 4 --list`.
//!
//! C# keeps its patro tables in public classes (`CalendarOffsets`, `TithiData`, `EventData`,
//! `HolidayData`) with offsets from BS 2001. This crate's tables are private and per day (ADR-0004
//! §6), so the rows about C#'s layout assert the same fact through the public API, each with a
//! one-line comment naming the L5-01 table fact it stands for.
// `non_snake_case`: the L2-08 name rule turns the `.` in a row id into `__`.
#![allow(non_snake_case, clippy::unwrap_used, clippy::indexing_slicing)]

use std::collections::BTreeSet;

use nepdate::{Error, Lang, NepaliDate, NepaliDateRange};
use nepdate_patro::{coverage, info, public_holidays};

fn bs(year: u16, month: u8, day: u8) -> NepaliDate {
    NepaliDate::new(year, month, day).unwrap()
}

/// Every day of BS 2001 to 2089, the years C# NepDate's offset tables span.
fn c_sharp_years() -> impl Iterator<Item = NepaliDate> {
    NepaliDateRange::new(bs(2001, 1, 1), NepaliDateRange::year(2089).unwrap().end())
        .unwrap()
        .days()
}

/// Every event name of the coverage, in both languages.
fn all_event_names() -> Vec<(&'static str, &'static str)> {
    coverage()
        .days()
        .flat_map(|d| info(d).events())
        .map(|e| (e.name(Lang::Ne), e.name(Lang::En)))
        .collect()
}

fn tithi_days() -> Vec<NepaliDate> {
    coverage()
        .days()
        .filter(|&d| info(d).tithi().is_some())
        .collect()
}

fn event_days() -> Vec<NepaliDate> {
    coverage()
        .days()
        .filter(|&d| info(d).has_events())
        .collect()
}

fn ne_en_events(date: NepaliDate) -> (Vec<&'static str>, Vec<&'static str>) {
    let info = info(date);
    (
        info.events().map(|e| e.name(Lang::Ne)).collect(),
        info.events().map(|e| e.name(Lang::En)).collect(),
    )
}

fn tithi(date: NepaliDate, lang: Lang) -> &'static str {
    info(date).tithi().map_or("", |t| t.name(lang))
}

// ---------- Core/CalendarDataTests.cs ----------

#[test]
fn cs_calendar_data_tests__calendar_strings_pool_is_non_empty() {
    // L5-01: NAMES holds 721 distinct event name pairs.
    let distinct: BTreeSet<_> = all_event_names().into_iter().collect();
    assert_eq!(distinct.len(), 721);
}

#[test]
fn cs_calendar_data_tests__calendar_strings_pool_all_entries_have_non_empty_np_and_en() {
    // L5-01: every entry of NAMES and TITHI_NAMES has both names.
    assert!(
        all_event_names()
            .iter()
            .all(|(np, en)| !np.is_empty() && !en.is_empty())
    );
    for date in tithi_days() {
        assert!(!tithi(date, Lang::Ne).is_empty() && !tithi(date, Lang::En).is_empty());
    }
}

#[test]
fn cs_calendar_data_tests__calendar_offsets_month_start_has_expected_length() {
    // L5-01: the per-day tables span PATRO_DAYS = 2793 days, whole months 2077-01 to 2084-07
    // plus 20 days of 2084-08.
    let months = (2077..=2084)
        .flat_map(|y| (1..=12).map(move |m| (y, m)))
        .take_while(|&(y, m)| (y, m) < (2084, 8));
    let days: u32 = months
        .map(|(y, m)| NepaliDateRange::month(y, m).unwrap().len())
        .sum();
    assert_eq!(days + 20, 2793);
    assert_eq!(coverage().len(), 2793);
}

#[test]
fn cs_calendar_data_tests__calendar_offsets_month_start_is_strictly_monotonic() {
    // L5-01: table index o = serial - PATRO_FIRST grows by one per day of the coverage.
    let days: Vec<NepaliDate> = coverage().days().collect();
    assert!(days.windows(2).all(|w| w[0].days_until(w[1]) == 1));
}

#[test]
fn cs_calendar_data_tests__calendar_offsets_total_days_matches_expected_count() {
    // L5-01: PATRO_DAYS is 2793 (C#'s 32509 counts BS 2001 to 2089).
    assert_eq!(coverage().len(), 2793);
    assert_eq!(c_sharp_years().count(), 32509);
}

#[test]
fn cs_calendar_data_tests__tithi_data_day_offsets_is_strictly_sorted() {
    // L5-01: tithi days are one gap-free run (codegen fails on a gap).
    let days = tithi_days();
    assert!(days.windows(2).all(|w| w[0].days_until(w[1]) == 1));
}

#[test]
fn cs_calendar_data_tests__tithi_data_all_pool_ids_are_in_range() {
    // L5-01: every DAY_TITHI entry indexes the 32 TITHI_NAMES.
    let names: BTreeSet<&str> = tithi_days()
        .into_iter()
        .map(|d| tithi(d, Lang::En))
        .collect();
    assert_eq!(names.len(), 32);
    assert!(!names.contains(""));
}

#[test]
fn cs_calendar_data_tests__tithi_data_entry_count_matches2190() {
    let days = tithi_days();
    assert_eq!(days.len(), 2190);
    assert_eq!(days.first(), Some(&bs(2078, 1, 2)));
    assert_eq!(days.last(), Some(&bs(2083, 12, 30)));
}

#[test]
fn cs_calendar_data_tests__event_data_day_offsets_is_strictly_sorted() {
    // L5-01: DAY_EVENTS is per day, so event days come out in date order.
    let days = event_days();
    assert!(days.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn cs_calendar_data_tests__event_data_slice_start_is_non_decreasing() {
    // L5-01: DAY_EVENTS[o] <= DAY_EVENTS[o + 1], seen as each day's slice length.
    for date in coverage().days() {
        let events = info(date).events();
        assert_eq!(events.len(), events.clone().count(), "{date}");
    }
}

#[test]
fn cs_calendar_data_tests__event_data_slice_start_sentinel_equals_entries_length() {
    // L5-01: DAY_EVENTS[PATRO_DAYS] == EVENT_IDS.len() == 3032.
    let total: usize = coverage().days().map(|d| info(d).events().len()).sum();
    assert_eq!(total, 3032);
}

#[test]
fn cs_calendar_data_tests__event_data_all_entries_are_in_range() {
    // L5-01: every EVENT_IDS entry indexes NAMES; an out-of-range one would give "".
    assert!(
        all_event_names()
            .iter()
            .all(|(np, en)| !np.is_empty() && !en.is_empty())
    );
}

#[test]
fn cs_calendar_data_tests__event_data_day_count_matches1671() {
    assert_eq!(event_days().len(), 1671);
}

#[test]
fn cs_calendar_data_tests__holiday_data_day_offsets_is_strictly_sorted() {
    let holidays: Vec<NepaliDate> = public_holidays().collect();
    assert!(holidays.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn cs_calendar_data_tests__holiday_data_count_matches294() {
    assert_eq!(public_holidays().len(), 294);
}

#[test]
fn cs_calendar_data_tests__calendar_offsets_get_offset_base_year_month1_day1_returns_zero() {
    // L5-01: o = 0 is COVERAGE_FIRST, 2077-01-01, and it has data.
    let first = coverage().start();
    assert_eq!(first, bs(2077, 1, 1));
    assert!(info(first).is_public_holiday());
}

#[test]
fn cs_calendar_data_tests__calendar_offsets_get_offset_out_of_range_returns_negative_one() {
    // L5-01: a date outside the tables gets the empty info, never an error.
    for date in [bs(2000, 1, 1), bs(2090, 1, 1), NepaliDate::MIN] {
        let info = info(date);
        assert!(info.tithi().is_none() && !info.is_public_holiday() && !info.has_events());
    }
}

#[test]
fn cs_calendar_data_tests__calendar_offsets_get_offset_all_offsets_are_unique_sentinel() {
    // L5-01: both ends of the tables hold data, so neither is padding.
    assert!(info(coverage().start()).is_public_holiday());
    assert!(info(coverage().end()).has_events());
}

#[test]
fn cs_calendar_data_tests__nepali_date_2081_01_01_calendar_data() {
    let date = bs(2081, 1, 1);
    assert_eq!(tithi(date, Lang::Ne), "पञ्चमी");
    assert_eq!(tithi(date, Lang::En), "Panchami");
    assert!(info(date).is_public_holiday());
    let (np, en) = ne_en_events(date);
    assert!(np.contains(&"नयाँ वर्ष"));
    assert!(en.contains(&"New Year"));
}

#[test]
fn cs_calendar_data_tests__nepali_date_2081_01_02_tithi_is_shashthi() {
    assert_eq!(tithi(bs(2081, 1, 2), Lang::Ne), "षष्ठी");
    assert_eq!(tithi(bs(2081, 1, 2), Lang::En), "Shashthi");
}

#[test]
fn cs_calendar_data_tests__nepali_date_2081_01_05_is_public_holiday() {
    let date = bs(2081, 1, 5);
    assert!(info(date).is_public_holiday());
    assert_eq!(tithi(date, Lang::Ne), "नवमी");
    assert_eq!(tithi(date, Lang::En), "Navami");
}

#[test]
fn cs_calendar_data_tests__nepali_date_2081_02_15_is_public_holiday_ganatantra_diwas() {
    let date = bs(2081, 2, 15);
    assert!(info(date).is_public_holiday());
    let (np, en) = ne_en_events(date);
    assert!(np.contains(&"गणतन्त्र दिवस"));
    assert!(en.contains(&"Republic Day"));
}

#[test]
fn cs_calendar_data_tests__nepali_date_2080_11_01_calendar_data() {
    let date = bs(2080, 11, 1);
    assert_eq!(tithi(date, Lang::Ne), "चतुर्थी");
    assert_eq!(tithi(date, Lang::En), "Chaturthi");
    assert!(!info(date).is_public_holiday());
    let (np, en) = ne_en_events(date);
    assert!(np.contains(&"जनयुद्ध दिवस"));
    assert!(en.contains(&"People's War Day"));
}

#[test]
fn cs_calendar_data_tests__nepali_date_before_range_all_properties_return_defaults() {
    let info = info(bs(2000, 6, 15));
    assert!(info.tithi().is_none());
    assert!(!info.is_public_holiday());
    assert_eq!(info.events().len(), 0);
}

#[test]
fn cs_calendar_data_tests__nepali_date_after_range_all_properties_return_defaults() {
    let info = info(bs(2090, 1, 1));
    assert!(info.tithi().is_none());
    assert!(!info.is_public_holiday());
    assert_eq!(info.events().len(), 0);
}

#[test]
fn cs_calendar_data_tests__nepali_date_outside_tithi_range_tithi_is_empty() {
    assert!(info(bs(2070, 6, 15)).tithi().is_none());
}

#[test]
fn cs_calendar_data_tests__nepali_date_events_np_and_events_en_always_have_same_length() {
    for date in [
        bs(2081, 1, 1),
        bs(2081, 1, 5),
        bs(2081, 2, 10),
        bs(2080, 11, 1),
        bs(2075, 1, 1),
    ] {
        let (np, en) = ne_en_events(date);
        assert_eq!(np.len(), en.len(), "{date}");
    }
}

#[test]
fn cs_calendar_data_tests__get_calendar_info_tithi_matches_flat_properties() {
    // The port has no flat properties on `NepaliDate` (D-13); two lookups must agree.
    let (a, b) = (info(bs(2081, 1, 1)), info(bs(2081, 1, 1)));
    assert_eq!(a.tithi(), b.tithi());
    assert_eq!(a.is_public_holiday(), b.is_public_holiday());
}

#[test]
fn cs_calendar_data_tests__all_days_in_scraped_range_offsets_are_in_bounds() {
    // L5-01: only days inside the coverage reach the tables; every other day is empty.
    let mut outside_with_data = 0;
    for date in c_sharp_years() {
        let info = info(date);
        let has_data = info.tithi().is_some() || info.is_public_holiday() || info.has_events();
        if has_data && !coverage().contains(date) {
            outside_with_data += 1;
        }
    }
    assert_eq!(outside_with_data, 0);
}

#[test]
fn cs_calendar_data_tests__all_days_in_scraped_range_no_exception_on_property_access() {
    let range =
        NepaliDateRange::new(bs(2078, 1, 1), NepaliDateRange::year(2082).unwrap().end()).unwrap();
    for date in range.days() {
        let info = info(date);
        let _ = info.tithi().map(|t| (t.name(Lang::Ne), t.name(Lang::En)));
        let _ = info.is_public_holiday();
        let _ = info.events().map(|e| e.name(Lang::En)).count();
    }
}

#[test]
fn cs_calendar_data_tests__all_tithi_entries_resolve_to_non_empty_strings() {
    for date in tithi_days() {
        assert!(!tithi(date, Lang::Ne).is_empty(), "{date}");
        assert!(!tithi(date, Lang::En).is_empty(), "{date}");
    }
}

#[test]
fn cs_calendar_data_tests__all_holiday_offsets_are_in_bounds() {
    assert!(public_holidays().all(|d| coverage().contains(d)));
}

#[test]
fn cs_calendar_data_tests__all_event_offsets_are_in_bounds() {
    assert!(event_days().iter().all(|&d| coverage().contains(d)));
}

#[test]
fn cs_calendar_data_tests__holiday_count_via_api_matches294() {
    let count = c_sharp_years()
        .filter(|&d| info(d).is_public_holiday())
        .count();
    assert_eq!(count, 294);
}

#[test]
fn cs_calendar_data_tests__calendar_info_event_arrays_are_never_null() {
    // An iterator is never null; a day without events gives an empty one.
    assert!(info(bs(2081, 1, 1)).events().len() > 0);
    assert_eq!(info(bs(2001, 1, 2)).events().len(), 0);
}

// ---------- Extensions/DateTimeExtensionsTests.cs ----------
// `DateTime.ToNepaliDate()` maps to `TryFrom` on the `jiff` and `chrono` date types; each row
// checks both. C#'s `ArgumentOutOfRangeException` is `Error::OutOfRange`.

fn from_jiff(y: i16, m: i8, d: i8) -> Result<NepaliDate, Error> {
    NepaliDate::try_from(jiff::civil::date(y, m, d))
}

fn from_chrono(y: i32, m: u32, d: u32) -> Result<NepaliDate, Error> {
    NepaliDate::try_from(chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap())
}

#[test]
fn cs_date_time_extensions_tests__to_nepali_date_valid_date_time_returns_correct_nepali_date() {
    assert_eq!(from_jiff(2023, 8, 30), Ok(bs(2080, 5, 13)));
    assert_eq!(from_chrono(2023, 8, 30), Ok(bs(2080, 5, 13)));
}

#[test]
fn cs_date_time_extensions_tests__to_nepali_date_min_value_throws_argument_out_of_range_exception()
{
    assert_eq!(from_jiff(1, 1, 1), Err(Error::OutOfRange));
    assert_eq!(from_chrono(1, 1, 1), Err(Error::OutOfRange));
}

#[test]
fn cs_date_time_extensions_tests__to_nepali_date_max_value_throws_argument_out_of_range_exception()
{
    assert_eq!(from_jiff(9999, 12, 31), Err(Error::OutOfRange));
    assert_eq!(from_chrono(9999, 12, 31), Err(Error::OutOfRange));
}

#[test]
fn cs_date_time_extensions_tests__to_nepali_date_date_before_min_supported_throws_argument_out_of_range_exception()
 {
    assert_eq!(from_jiff(1843, 1, 1), Err(Error::OutOfRange));
    assert_eq!(from_chrono(1843, 1, 1), Err(Error::OutOfRange));
}

#[test]
fn cs_date_time_extensions_tests__to_nepali_date_date_after_max_supported_throws_argument_out_of_range_exception()
 {
    assert_eq!(from_jiff(2200, 1, 1), Err(Error::OutOfRange));
    assert_eq!(from_chrono(2200, 1, 1), Err(Error::OutOfRange));
}
