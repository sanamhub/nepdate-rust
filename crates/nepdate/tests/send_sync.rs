//! Every public type is `Send + Sync`, and conversion gives the same answers on many threads
//! (ADR-0014).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use nepdate::{
    DateDiff, DateOrder, Days, Digits, Error, FiscalLabel, FiscalYear, GregorianDate, Lang,
    LongDate, LongFormat, Month, MonthOverflow, NepaliDate, NepaliDateRange, ParseErrorKind,
    PatternFormat, Quarter, Separator, ShortFormat, Weekday,
};

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn public_types_are_send_and_sync() {
    assert_send_sync::<NepaliDate>();
    assert_send_sync::<GregorianDate>();
    assert_send_sync::<Lang>();
    assert_send_sync::<Month>();
    assert_send_sync::<Weekday>();
    assert_send_sync::<Error>();
    assert_send_sync::<ParseErrorKind>();
    assert_send_sync::<DateDiff>();
    assert_send_sync::<MonthOverflow>();
    assert_send_sync::<FiscalYear>();
    assert_send_sync::<FiscalLabel>();
    assert_send_sync::<Quarter>();
    assert_send_sync::<NepaliDateRange>();
    assert_send_sync::<Days>();
    assert_send_sync::<Digits>();
    assert_send_sync::<DateOrder>();
    assert_send_sync::<Separator>();
    assert_send_sync::<LongDate>();
    assert_send_sync::<ShortFormat>();
    assert_send_sync::<LongFormat>();
    assert_send_sync::<PatternFormat<'static>>();
}

fn all_days() -> Vec<(NepaliDate, GregorianDate)> {
    let mut out = Vec::with_capacity(109_212);
    let mut date = Some(NepaliDate::MIN);
    while let Some(d) = date {
        out.push((d, d.to_gregorian()));
        date = d.succ();
    }
    out
}

#[test]
fn eight_threads_agree_with_one() {
    let expected = all_days();
    assert_eq!(expected.len(), 109_212);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| assert_eq!(all_days(), expected));
        }
    });
}
