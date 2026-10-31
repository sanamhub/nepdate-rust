#![doc = include_str!("../README.md")]
#![no_std]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use core::iter::FusedIterator;

use nepdate::{Lang, NepaliDate, NepaliDateRange};

mod generated {
    pub(crate) mod patro;
}

use crate::generated::patro::{
    COVERAGE_FIRST, COVERAGE_LAST, DAY_EVENTS, DAY_TITHI, EVENT_IDS, HOLIDAY_BITS, HOLIDAY_OFFSETS,
    NAMES, NO_TITHI, PATRO_DAYS, PATRO_FIRST, TITHI_NAMES,
};

// Table facts the lookups rely on, checked when the crate compiles.
const _: () = assert!(TITHI_NAMES.len() < NO_TITHI as usize);
const _: () = assert!(NAMES.len() <= u16::MAX as usize);
const _: () = assert!(EVENT_IDS.len() <= u16::MAX as usize);
const _: () = assert!(HOLIDAY_BITS.len() * 64 >= PATRO_DAYS);

/// `PATRO_FIRST` as an index. Widening `u32` to `usize` is lossless on 32- and 64-bit targets.
const FIRST: usize = PATRO_FIRST as usize;

// The `panic!` arms run only while compiling, so a bad table fails the build, never a caller.
#[allow(clippy::panic)]
const COVERAGE: NepaliDateRange = {
    let Ok(first) = NepaliDate::new(COVERAGE_FIRST.0, COVERAGE_FIRST.1, COVERAGE_FIRST.2) else {
        panic!("COVERAGE_FIRST is not a valid BS date");
    };
    let Ok(last) = NepaliDate::new(COVERAGE_LAST.0, COVERAGE_LAST.1, COVERAGE_LAST.2) else {
        panic!("COVERAGE_LAST is not a valid BS date");
    };
    match NepaliDateRange::new(first, last) {
        Some(range) => range,
        None => panic!("COVERAGE_FIRST is after COVERAGE_LAST"),
    }
};

/// Info of a day without data: no tithi, no holiday, no events.
const EMPTY: CalendarInfo = CalendarInfo {
    tithi: NO_TITHI,
    holiday: false,
    events: (0, 0),
};

/// The tithi, public holiday flag and events of one BS day (PATRO §2).
///
/// Never fails: a day without data, inside or outside [`coverage`], gets an empty info.
///
/// ```
/// use nepdate::{Lang, NepaliDate};
///
/// let info = nepdate_patro::info(NepaliDate::new(2081, 1, 1)?);
/// assert_eq!(info.tithi().map(|t| t.name(Lang::En)), Some("Panchami"));
/// assert!(info.is_public_holiday());
/// let events: Vec<&str> = info.events().map(|e| e.name(Lang::En)).collect();
/// assert_eq!(events, ["New Year", "Mesh Sankranti", "Bisket Jatra"]);
/// assert_eq!(info.events().next().map(|e| e.name(Lang::Ne)), Some("नयाँ वर्ष"));
///
/// let none = nepdate_patro::info(NepaliDate::new(2090, 1, 1)?);
/// assert!(none.tithi().is_none() && !none.is_public_holiday() && !none.has_events());
/// # Ok::<(), nepdate::Error>(())
/// ```
#[must_use]
pub fn info(date: NepaliDate) -> CalendarInfo {
    // A date before the coverage wraps to a huge index, so the `get`s below are the only range
    // check. `days_until` from MIN is never negative.
    let o = usize::try_from(NepaliDate::MIN.days_until(date))
        .unwrap_or(usize::MAX)
        .wrapping_sub(FIRST);
    let (Some(&tithi), Some(&word), Some(&start), Some(&end)) = (
        DAY_TITHI.get(o),
        HOLIDAY_BITS.get(o / 64),
        DAY_EVENTS.get(o),
        DAY_EVENTS.get(o.wrapping_add(1)),
    ) else {
        return EMPTY;
    };
    CalendarInfo {
        tithi,
        holiday: (word >> (o % 64)) & 1 == 1,
        events: (start, end),
    }
}

/// The smallest BS range that holds every day with any data: 2077-01-01 to 2084-08-20 with
/// the data compiled into this version.
///
/// ```
/// let range = nepdate_patro::coverage();
/// assert_eq!(range.start().to_string(), "2077/01/01");
/// assert_eq!(range.end().to_string(), "2084/08/20");
/// ```
#[must_use]
pub fn coverage() -> NepaliDateRange {
    COVERAGE
}

/// Every public holiday, in ascending date order.
///
/// ```
/// let mut holidays = nepdate_patro::public_holidays();
/// assert_eq!(holidays.next().map(|d| d.to_string()).as_deref(), Some("2077/01/01"));
/// assert_eq!(holidays.next_back().map(|d| d.to_string()).as_deref(), Some("2084/08/18"));
/// ```
#[must_use]
pub fn public_holidays() -> impl DoubleEndedIterator<Item = NepaliDate> + ExactSizeIterator {
    PublicHolidays {
        offsets: HOLIDAY_OFFSETS.iter(),
    }
}

/// The iterator behind [`public_holidays`].
///
/// Every offset is inside the coverage (a unit test checks it), so the conversion never fails
/// and `len` is exact. If it ever did fail, the iterator would end early instead of panicking.
#[derive(Clone, Debug)]
struct PublicHolidays {
    offsets: core::slice::Iter<'static, u16>,
}

fn holiday_date(o: u16) -> Option<NepaliDate> {
    let serial = i32::try_from(FIRST + usize::from(o)).ok()?;
    NepaliDate::MIN.checked_add_days(serial).ok()
}

impl Iterator for PublicHolidays {
    type Item = NepaliDate;

    fn next(&mut self) -> Option<NepaliDate> {
        self.offsets.next().copied().and_then(holiday_date)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.offsets.size_hint()
    }
}

impl DoubleEndedIterator for PublicHolidays {
    fn next_back(&mut self) -> Option<NepaliDate> {
        self.offsets.next_back().copied().and_then(holiday_date)
    }
}

impl ExactSizeIterator for PublicHolidays {}

impl FusedIterator for PublicHolidays {}

/// What the patro says about one day, from [`info`]. Six bytes, `Copy`.
///
/// ```
/// use nepdate::NepaliDate;
///
/// let info = nepdate_patro::info(NepaliDate::new(2081, 1, 2)?);
/// assert!(info.tithi().is_some() && !info.has_events());
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct CalendarInfo {
    tithi: u8,
    holiday: bool,
    events: (u16, u16),
}

impl CalendarInfo {
    /// The tithi (lunar day), or `None` outside 2078-01-02 to 2083-12-30.
    ///
    /// ```
    /// use nepdate::{Lang, NepaliDate};
    ///
    /// let tithi = nepdate_patro::info(NepaliDate::new(2081, 1, 2)?).tithi();
    /// assert_eq!(tithi.map(|t| t.name(Lang::En)), Some("Shashthi"));
    /// assert!(nepdate_patro::info(NepaliDate::new(2078, 1, 1)?).tithi().is_none());
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn tithi(self) -> Option<Tithi> {
        if self.tithi == NO_TITHI {
            None
        } else {
            Some(Tithi(self.tithi))
        }
    }

    /// Whether the day is a gazetted public holiday.
    ///
    /// ```
    /// use nepdate::NepaliDate;
    ///
    /// assert!(nepdate_patro::info(NepaliDate::new(2081, 2, 15)?).is_public_holiday());
    /// assert!(!nepdate_patro::info(NepaliDate::new(2081, 1, 2)?).is_public_holiday());
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn is_public_holiday(self) -> bool {
        self.holiday
    }

    /// The day's events, in the order of the source data (display order).
    ///
    /// ```
    /// use nepdate::{Lang, NepaliDate};
    ///
    /// let info = nepdate_patro::info(NepaliDate::new(2081, 2, 15)?);
    /// let events: Vec<&str> = info.events().map(|e| e.name(Lang::En)).collect();
    /// assert_eq!(events, ["Republic Day"]);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn events(self) -> Events {
        Events {
            front: self.events.0,
            back: self.events.1,
        }
    }

    /// Whether the day has at least one event.
    ///
    /// ```
    /// use nepdate::NepaliDate;
    ///
    /// assert!(nepdate_patro::info(NepaliDate::new(2081, 1, 1)?).has_events());
    /// assert!(!nepdate_patro::info(NepaliDate::new(2081, 1, 2)?).has_events());
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn has_events(self) -> bool {
        self.events.0 < self.events.1
    }
}

/// The events of one day, from [`CalendarInfo::events`].
///
/// ```
/// use nepdate::NepaliDate;
///
/// let events = nepdate_patro::info(NepaliDate::new(2080, 11, 1)?).events();
/// assert!(events.len() > 0);
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct Events {
    front: u16,
    back: u16,
}

impl Iterator for Events {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        if self.front >= self.back {
            return None;
        }
        let id = *EVENT_IDS.get(usize::from(self.front))?;
        self.front += 1;
        Some(Event(id))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = usize::from(self.back.saturating_sub(self.front));
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Events {
    fn next_back(&mut self) -> Option<Event> {
        if self.front >= self.back {
            return None;
        }
        let id = *EVENT_IDS.get(usize::from(self.back - 1))?;
        self.back -= 1;
        Some(Event(id))
    }
}

impl ExactSizeIterator for Events {}

impl FusedIterator for Events {}

/// A tithi (lunar day), from [`CalendarInfo::tithi`].
///
/// ```
/// use nepdate::{Lang, NepaliDate};
///
/// let tithi = nepdate_patro::info(NepaliDate::new(2081, 1, 1)?).tithi();
/// assert_eq!(tithi.map(|t| t.name(Lang::Ne)), Some("पञ्चमी"));
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tithi(u8);

impl Tithi {
    /// The tithi's name: Devanagari for [`Lang::Ne`], Latin for [`Lang::En`].
    ///
    /// ```
    /// use nepdate::{Lang, NepaliDate};
    ///
    /// let tithi = nepdate_patro::info(NepaliDate::new(2080, 11, 1)?).tithi();
    /// assert_eq!(tithi.map(|t| t.name(Lang::En)), Some("Chaturthi"));
    /// assert_eq!(tithi.map(|t| t.name(Lang::Ne)), Some("चतुर्थी"));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn name(self, lang: Lang) -> &'static str {
        // A `Tithi` only comes from `DAY_TITHI`, whose entries are all in range (unit test),
        // so the empty fallback is never returned.
        TITHI_NAMES
            .get(usize::from(self.0))
            .map_or("", |&(np, en)| pick(np, en, lang))
    }
}

/// One event of a day, from [`Events`].
///
/// ```
/// use nepdate::{Lang, NepaliDate};
///
/// let info = nepdate_patro::info(NepaliDate::new(2081, 4, 15)?);
/// let second = info.events().nth(1).map(|e| e.name(Lang::En));
/// assert_eq!(second, Some("World Day Against Human Trafficking"));
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event(u16);

impl Event {
    /// The event's name: Devanagari for [`Lang::Ne`], Latin for [`Lang::En`].
    ///
    /// ```
    /// use nepdate::{Lang, NepaliDate};
    ///
    /// let info = nepdate_patro::info(NepaliDate::new(2081, 2, 15)?);
    /// assert_eq!(info.events().next().map(|e| e.name(Lang::En)), Some("Republic Day"));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn name(self, lang: Lang) -> &'static str {
        // An `Event` only comes from `EVENT_IDS`, whose entries are all in range (unit test).
        NAMES
            .get(usize::from(self.0))
            .map_or("", |&(np, en)| pick(np, en, lang))
    }
}

fn pick(np: &'static str, en: &'static str, lang: Lang) -> &'static str {
    match lang {
        Lang::Ne => np,
        Lang::En => en,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

    extern crate std;

    use std::borrow::ToOwned;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::Path;
    use std::string::String;

    use super::*;

    /// The numeric fields of `shared/spec/vectors/patro-summary.json`, the counts the hub
    /// generates from the same data. Its shape is fixed, so reading the `"key": value` lines is
    /// enough; the workspace has no JSON crate and this test must not add one.
    fn summary() -> BTreeMap<String, usize> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../shared/spec/vectors/patro-summary.json");
        let text = fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!(
                "reading {}: {err}; shared/ may predate the summary, re-sync it",
                path.display()
            )
        });
        text.lines()
            .filter_map(|line| {
                let (key, value) = line.trim().trim_end_matches(',').split_once(':')?;
                Some((
                    key.trim().trim_matches('"').to_owned(),
                    value.trim().parse().ok()?,
                ))
            })
            .collect()
    }

    fn count(summary: &BTreeMap<String, usize>, field: &str) -> usize {
        *summary
            .get(field)
            .unwrap_or_else(|| panic!("patro-summary.json has no numeric {field}"))
    }

    #[test]
    fn table_counts_match_the_summary() {
        let summary = summary();
        assert_eq!(PATRO_DAYS, count(&summary, "days"));
        let tithi_days = DAY_TITHI.iter().filter(|&&t| t != NO_TITHI).count();
        assert_eq!(tithi_days, count(&summary, "tithi_days"));
        assert_eq!(HOLIDAY_OFFSETS.len(), count(&summary, "holidays"));
        let bits: u32 = HOLIDAY_BITS.iter().map(|word| word.count_ones()).sum();
        assert_eq!(usize::try_from(bits).unwrap(), count(&summary, "holidays"));
        assert_eq!(
            usize::from(DAY_EVENTS[PATRO_DAYS]),
            count(&summary, "events")
        );
        assert_eq!(EVENT_IDS.len(), count(&summary, "events"));
        let event_days = DAY_EVENTS.windows(2).filter(|w| w[0] < w[1]).count();
        assert_eq!(event_days, count(&summary, "event_days"));
        assert_eq!(NAMES.len(), count(&summary, "event_names"));
        assert_eq!(TITHI_NAMES.len(), count(&summary, "tithi_names"));

        // Distinct pairs and their UTF-8 bytes, counted over tithi and events together.
        let names: BTreeSet<_> = NAMES.iter().chain(&TITHI_NAMES).collect();
        assert_eq!(names.len(), count(&summary, "names"));
        let name_bytes: usize = names.iter().map(|name| name.0.len() + name.1.len()).sum();
        assert_eq!(name_bytes, count(&summary, "name_bytes"));
    }

    #[test]
    fn indices_are_in_range() {
        assert!(EVENT_IDS.iter().all(|&id| usize::from(id) < NAMES.len()));
        assert!(
            DAY_TITHI
                .iter()
                .all(|&t| usize::from(t) < TITHI_NAMES.len() || t == NO_TITHI)
        );
        assert!(DAY_EVENTS.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(DAY_EVENTS[0], 0);
    }

    #[test]
    fn holiday_offsets_match_the_bits_and_the_coverage() {
        assert!(HOLIDAY_OFFSETS.windows(2).all(|w| w[0] < w[1]));
        for &o in &HOLIDAY_OFFSETS {
            assert!(holiday_date(o).is_some());
            let o = usize::from(o);
            assert!(o < PATRO_DAYS);
            assert_eq!((HOLIDAY_BITS[o / 64] >> (o % 64)) & 1, 1);
        }
        let unused_bits = HOLIDAY_BITS.len() * 64 - PATRO_DAYS;
        let last_word = HOLIDAY_BITS[HOLIDAY_BITS.len() - 1];
        assert!(last_word.leading_zeros() as usize >= unused_bits);
    }

    #[test]
    fn patro_first_is_the_serial_of_the_coverage_start() {
        let serial = NepaliDate::MIN.days_until(COVERAGE.start());
        assert_eq!(usize::try_from(serial).unwrap(), FIRST);
        assert_eq!(usize::try_from(COVERAGE.len()).unwrap(), PATRO_DAYS);
    }

    #[test]
    fn names_are_never_empty() {
        assert!(
            NAMES
                .iter()
                .chain(&TITHI_NAMES)
                .all(|(np, en)| !np.is_empty() && !en.is_empty())
        );
    }

    #[test]
    fn info_is_six_bytes() {
        assert_eq!(core::mem::size_of::<CalendarInfo>(), 6);
    }
}
