//! Features `serde`, `jiff` and `chrono`. Each test needs only its own
//! feature, so `cargo test --features serde` alone runs the serde tests.
#![allow(clippy::unwrap_used)]

#[cfg(feature = "serde")]
mod serde_feature {
    use nepdate::NepaliDate;

    #[test]
    fn round_trips_as_yyyy_mm_dd() {
        let date: NepaliDate = serde_json::from_str("\"2080-04-15\"").unwrap();
        assert_eq!(date, NepaliDate::new(2080, 4, 15).unwrap());
        assert_eq!(serde_json::to_string(&date).unwrap(), "\"2080-04-15\"");
    }

    #[test]
    fn pads_month_and_day() {
        let date = NepaliDate::new(2081, 1, 1).unwrap();
        assert_eq!(serde_json::to_string(&date).unwrap(), "\"2081-01-01\"");
    }

    #[test]
    fn invalid_strings_are_errors() {
        assert!(serde_json::from_str::<NepaliDate>("\"2080-13-01\"").is_err());
        assert!(serde_json::from_str::<NepaliDate>("\"2080-04\"").is_err());
        assert!(serde_json::from_str::<NepaliDate>("20800415").is_err());
    }
}

#[cfg(feature = "chrono")]
mod chrono_feature {
    use chrono::NaiveDate;
    use nepdate::{Error, NepaliDate};

    #[test]
    fn new_year_2081_both_ways() {
        let ad = NaiveDate::from_ymd_opt(2024, 4, 13).unwrap();
        let bs = NepaliDate::try_from(ad).unwrap();
        assert_eq!(bs, NepaliDate::new(2081, 1, 1).unwrap());
        assert_eq!(NaiveDate::from(bs), ad);
    }

    #[test]
    fn range_edges() {
        let before = NaiveDate::from_ymd_opt(1844, 4, 10).unwrap();
        assert_eq!(NepaliDate::try_from(before), Err(Error::OutOfRange));
        let after = NaiveDate::from_ymd_opt(2143, 4, 16).unwrap();
        assert_eq!(NepaliDate::try_from(after), Err(Error::OutOfRange));
        assert_eq!(
            NaiveDate::from(NepaliDate::MIN),
            NaiveDate::from_ymd_opt(1844, 4, 11).unwrap()
        );
        assert_eq!(
            NaiveDate::from(NepaliDate::MAX),
            NaiveDate::from_ymd_opt(2143, 4, 15).unwrap()
        );
    }
}

#[cfg(feature = "jiff")]
mod jiff_feature {
    use jiff::Timestamp;
    use jiff::civil::{Date, date};
    use jiff::tz::TimeZone;
    use nepdate::{Error, NepaliDate};

    #[test]
    fn new_year_2081_both_ways() {
        let bs = NepaliDate::try_from(date(2024, 4, 13)).unwrap();
        assert_eq!(bs, NepaliDate::new(2081, 1, 1).unwrap());
        assert_eq!(Date::from(bs), date(2024, 4, 13));
        assert_eq!(
            NepaliDate::try_from(date(1844, 4, 10)),
            Err(Error::OutOfRange)
        );
        assert_eq!(Date::from(NepaliDate::MAX), date(2143, 4, 15));
    }

    #[test]
    fn clock_test_through_the_conversion() {
        // 2024-07-30 18:30 UTC is BS 2081-04-15 in UTC, 2081-04-16 in Nepal.
        let now: Timestamp = "2024-07-30T18:30:00Z".parse().unwrap();
        let utc = now.to_zoned(TimeZone::UTC).date();
        let nepal = now
            .to_zoned(TimeZone::get("Asia/Kathmandu").unwrap())
            .date();
        assert_eq!(NepaliDate::try_from(utc), NepaliDate::new(2081, 4, 15));
        assert_eq!(NepaliDate::try_from(nepal), NepaliDate::new(2081, 4, 16));
    }

    #[test]
    fn today_is_the_date_in_nepal() {
        // Read the clock on both sides of `today()`, so a midnight in between can't fail it.
        let kathmandu = TimeZone::get("Asia/Kathmandu").unwrap();
        let before = NepaliDate::try_from(Timestamp::now().to_zoned(kathmandu.clone()).date());
        let today = NepaliDate::today();
        let after = NepaliDate::try_from(Timestamp::now().to_zoned(kathmandu.clone()).date());
        assert!(today == before || today == after, "{today:?}");
        assert!(
            NepaliDate::today_in(&kathmandu) == today || NepaliDate::today_in(&kathmandu) == after
        );
    }

    #[test]
    fn is_today_only_for_today() {
        let today = NepaliDate::today().unwrap();
        // Midnight may pass between the calls; then the next day is today.
        assert!(today.is_today() || today.succ().unwrap().is_today());
        assert!(!today.pred().unwrap().is_today());
        assert!(!NepaliDate::MIN.is_today());
    }
}
