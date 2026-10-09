//! Every public type is `Send + Sync`, and conversion gives the same answers on many threads
//! (L2-09, ADR-0014).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use nepdate::{Error, GregorianDate, Lang, Month, NepaliDate, ParseErrorKind, Weekday};

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
