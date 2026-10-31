//! PATRO §4 anchors, the tithi range edges, `coverage()` and `public_holidays()`.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use nepdate::{Lang, NepaliDate};
use nepdate_patro::{coverage, info, public_holidays};

mod summary;

fn bs(y: u16, m: u8, d: u8) -> NepaliDate {
    NepaliDate::new(y, m, d).unwrap()
}

/// Tithi (en), holiday flag and event names (en) of one day.
fn facts(date: NepaliDate) -> (Option<&'static str>, bool, Vec<&'static str>) {
    let info = info(date);
    (
        info.tithi().map(|t| t.name(Lang::En)),
        info.is_public_holiday(),
        info.events().map(|e| e.name(Lang::En)).collect(),
    )
}

#[test]
fn patro_section_4_anchors() {
    let rows: [(NepaliDate, Option<&str>, bool, &[&str]); 7] = [
        (
            bs(2081, 1, 1),
            Some("Panchami"),
            true,
            &["New Year", "Mesh Sankranti", "Bisket Jatra"],
        ),
        (bs(2081, 1, 2), Some("Shashthi"), false, &[]),
        (bs(2081, 2, 15), Some("Panchami"), true, &["Republic Day"]),
        (
            bs(2080, 11, 1),
            Some("Chaturthi"),
            false,
            &[
                "Mangala Chauthi Brata",
                "People's War Day",
                "Tilakund Chauthi",
                "Kumbha Sankranti",
                "World Radio Day",
            ],
        ),
        (
            bs(2081, 4, 15),
            Some("Dashami"),
            false,
            &[
                "Kheer Khane Din",
                "World Day Against Human Trafficking",
                "International Friendship Day",
            ],
        ),
        (bs(2070, 6, 15), None, false, &[]),
        (bs(2090, 1, 1), None, false, &[]),
    ];
    for (date, tithi, holiday, events) in rows {
        assert_eq!(facts(date), (tithi, holiday, events.to_vec()), "{date}");
    }
    let first = info(bs(2081, 1, 1)).events().next().unwrap();
    assert_eq!(first.name(Lang::Ne), "नयाँ वर्ष");
}

#[test]
fn tithi_range_edges_match_the_summary() {
    let (first_year, first_month, first_day) = summary::day("tithi_first_day");
    let (last_year, last_month, last_day) = summary::day("tithi_last_day");
    let first = bs(first_year, first_month, first_day);
    let last = bs(last_year, last_month, last_day);
    assert!(info(first).tithi().is_some());
    assert!(info(last).tithi().is_some());
    assert!(info(first.checked_add_days(-1).unwrap()).tithi().is_none());
    assert!(info(last.checked_add_days(1).unwrap()).tithi().is_none());
}

#[test]
fn coverage_matches_the_summary() {
    let (first_year, first_month, first_day) = summary::day("first_day");
    let (last_year, last_month, last_day) = summary::day("last_day");
    assert_eq!(coverage().start(), bs(first_year, first_month, first_day));
    assert_eq!(coverage().end(), bs(last_year, last_month, last_day));
    assert_eq!(
        usize::try_from(coverage().len()).unwrap(),
        summary::number("days")
    );
}

#[test]
fn public_holidays_ascending() {
    let all: Vec<NepaliDate> = public_holidays().collect();
    assert_eq!(all.len(), summary::number("holidays"));
    assert_eq!(public_holidays().len(), summary::number("holidays"));
    assert!(all.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(all.first(), Some(&bs(2077, 1, 1)));
    assert_eq!(public_holidays().next_back(), Some(bs(2084, 8, 18)));
    assert!(all.iter().all(|&d| info(d).is_public_holiday()));
}

#[test]
fn every_covered_day_and_the_counts() {
    let (mut tithi, mut holidays, mut event_days, mut events) = (0usize, 0, 0, 0);
    for date in coverage().days() {
        let info = info(date);
        let n = info.events().len();
        assert_eq!(info.has_events(), n > 0, "{date}");
        assert_eq!(info.events().count(), n, "{date}");
        assert_eq!(info.events().rev().count(), n, "{date}");
        tithi += usize::from(info.tithi().is_some());
        holidays += usize::from(info.is_public_holiday());
        event_days += usize::from(info.has_events());
        events += n;
    }
    // PATRO §3: T, H, E and the event total.
    assert_eq!(
        (tithi, holidays, event_days, events),
        (
            summary::number("tithi_days"),
            summary::number("holidays"),
            summary::number("event_days"),
            summary::number("events"),
        )
    );
}

#[test]
fn edges_of_the_supported_range_have_no_data() {
    for date in [
        NepaliDate::MIN,
        NepaliDate::MAX,
        bs(2076, 12, 30),
        bs(2084, 8, 21),
    ] {
        assert_eq!(facts(date), (None, false, vec![]), "{date}");
    }
}
