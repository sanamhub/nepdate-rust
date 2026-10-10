//! PATRO §4 anchors, the tithi range edges, `coverage()` and `public_holidays()` (L5-02).
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use nepdate::{Lang, NepaliDate};
use nepdate_patro::{coverage, info, public_holidays};

fn bs(y: u16, m: u8, d: u8) -> NepaliDate {
    NepaliDate::new(y, m, d).unwrap()
}

/// Tithi (en), holiday flag and event names (en) of one day.
fn summary(date: NepaliDate) -> (Option<&'static str>, bool, Vec<&'static str>) {
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
        assert_eq!(summary(date), (tithi, holiday, events.to_vec()), "{date}");
    }
    let first = info(bs(2081, 1, 1)).events().next().unwrap();
    assert_eq!(first.name(Lang::Ne), "नयाँ वर्ष");
}

#[test]
fn tithi_range_edges() {
    assert!(info(bs(2078, 1, 1)).tithi().is_none());
    assert!(info(bs(2078, 1, 2)).tithi().is_some());
    assert!(info(bs(2083, 12, 30)).tithi().is_some());
    assert!(info(bs(2084, 1, 1)).tithi().is_none());
}

#[test]
fn coverage_is_2077_01_01_to_2084_08_20() {
    assert_eq!(coverage().start(), bs(2077, 1, 1));
    assert_eq!(coverage().end(), bs(2084, 8, 20));
}

#[test]
fn public_holidays_ascending() {
    let all: Vec<NepaliDate> = public_holidays().collect();
    assert_eq!(all.len(), 294);
    assert_eq!(public_holidays().len(), 294);
    assert!(all.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(all.first(), Some(&bs(2077, 1, 1)));
    assert_eq!(public_holidays().next_back(), Some(bs(2084, 8, 18)));
    assert!(all.iter().all(|&d| info(d).is_public_holiday()));
}

#[test]
fn every_covered_day_and_the_counts() {
    let (mut tithi, mut holidays, mut event_days, mut events) = (0, 0, 0, 0);
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
        (2190, 294, 1671, 3032)
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
        assert_eq!(summary(date), (None, false, vec![]), "{date}");
    }
}
