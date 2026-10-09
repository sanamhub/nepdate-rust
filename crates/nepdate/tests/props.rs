//! Property tests (L3-05, ADR-0014).
//!
//! proptest runs without its `std` feature: that feature pulls in `getrandom`, which needs an
//! assembler to build on the `windows-gnu` host. Without it the generator seed is fixed, so every
//! run checks the same cases; the cases are still random over the whole range.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use nepdate::{MonthOverflow, NepaliDate};
use proptest::prelude::*;

/// Any supported date: a serial from 0 to `109_211` turned into a date.
fn any_date() -> impl Strategy<Value = NepaliDate> {
    (0i32..=109_211).prop_map(|s| NepaliDate::MIN.checked_add_days(s).unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    #[test]
    fn add_days_is_undone_by_subtracting(d in any_date(), n in -120_000i32..=120_000) {
        if let Ok(e) = d.checked_add_days(n) {
            prop_assert_eq!(e.checked_add_days(-n), Ok(d));
            prop_assert_eq!(d.days_until(e), n);
        } else {
            let target = i64::from(NepaliDate::MIN.days_until(d)) + i64::from(n);
            prop_assert!(!(0..=109_211).contains(&target));
        }
    }

    #[test]
    fn bs_order_is_ad_order(a in any_date(), b in any_date()) {
        prop_assert_eq!(a.cmp(&b), a.to_gregorian().cmp(&b.to_gregorian()));
    }

    #[test]
    fn days_until_is_antisymmetric(a in any_date(), b in any_date()) {
        prop_assert_eq!(a.days_until(b), -b.days_until(a));
    }

    #[test]
    fn diff_rebuilds_the_later_date(a in any_date(), b in any_date()) {
        let diff = a.diff(b);
        let (from, to) = if diff.negative { (b, a) } else { (a, b) };
        let months = i32::from(diff.years) * 12 + i32::from(diff.months);
        let rebuilt = from
            .checked_add_months(months, MonthOverflow::Clamp)
            .and_then(|d| d.checked_add_days(i32::from(diff.days)));
        prop_assert_eq!(rebuilt, Ok(to));
        prop_assert_eq!(diff.total_days, a.days_until(b));
    }
}
