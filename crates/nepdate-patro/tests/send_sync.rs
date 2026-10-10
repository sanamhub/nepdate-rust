//! Every public type is `Send + Sync`, and `info` gives the same answers on many threads (L5-07,
//! ADR-0014).
#![allow(clippy::unwrap_used)]

use nepdate::{Lang, NepaliDate};
use nepdate_patro::{CalendarInfo, Event, Events, Tithi, coverage, info, public_holidays};

fn assert_send_sync<T: Send + Sync>() {}

fn assert_value_send_sync<T: Send + Sync>(_: &T) {}

#[test]
fn public_types_are_send_and_sync() {
    assert_send_sync::<CalendarInfo>();
    assert_send_sync::<Events>();
    assert_send_sync::<Tithi>();
    assert_send_sync::<Event>();
    // `public_holidays()` returns an unnamed type, so check a value of it.
    assert_value_send_sync(&public_holidays());
}

type Day = (
    NepaliDate,
    Option<&'static str>,
    bool,
    Vec<(&'static str, &'static str)>,
);

fn all_days() -> Vec<Day> {
    coverage()
        .days()
        .map(|d| {
            let info = info(d);
            (
                d,
                info.tithi().map(|t| t.name(Lang::Ne)),
                info.is_public_holiday(),
                info.events()
                    .map(|e| (e.name(Lang::Ne), e.name(Lang::En)))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn eight_threads_agree_with_one() {
    let expected = all_days();
    assert_eq!(expected.len(), 2793);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| assert_eq!(all_days(), expected));
        }
    });
}
