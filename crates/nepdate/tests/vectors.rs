//! Conformance with `shared/spec/vectors/month-boundaries.csv`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names
)]

mod common;

use nepdate::{Lang, NepaliDate};

#[test]
fn month_boundaries() {
    let rows = common::rows("spec/vectors/month-boundaries.csv", ',');
    assert_eq!(rows.len(), 7176, "a truncated vector file can't pass");
    for row in &rows {
        let [bs, ad, weekday, len] = row.as_slice() else {
            panic!("bad row {row:?}");
        };
        let date = common::bs(bs);
        assert_eq!(date.to_gregorian().to_string(), *ad, "BS to AD for {bs}");
        let (y, m, d) = common::ymd(ad);
        assert_eq!(
            NepaliDate::from_gregorian(y, m, d),
            Ok(date),
            "AD to BS for {ad}"
        );
        assert_eq!(date.weekday().name(Lang::En), weekday, "weekday of {bs}");
        assert_eq!(
            date.month_length().to_string(),
            *len,
            "month length of {bs}"
        );
    }
}
