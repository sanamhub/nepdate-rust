//! `NepaliDateRange`, an inclusive span of days, and its `Days` iterator (D-09).

use core::iter::FusedIterator;

use crate::{Error, NepaliDate};

/// The days from `start` to `end`, both included. Never empty: `start <= end`.
///
/// ```
/// use nepdate::NepaliDateRange;
///
/// let shrawan = NepaliDateRange::month(2081, 4)?;
/// assert_eq!(shrawan.len(), 32);
/// assert_eq!(shrawan.days().last(), Some(shrawan.end()));
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NepaliDateRange {
    start: NepaliDate,
    end: NepaliDate,
}

impl NepaliDateRange {
    /// The range from `start` to `end`, or `None` when `start` is after `end`.
    ///
    /// `const`, so a range can be a constant.
    ///
    /// ```
    /// use nepdate::{NepaliDate, NepaliDateRange};
    ///
    /// let a = NepaliDate::new(2081, 1, 1)?;
    /// let b = NepaliDate::new(2081, 1, 10)?;
    /// assert_eq!(NepaliDateRange::new(a, b).map(NepaliDateRange::len), Some(10));
    /// assert_eq!(NepaliDateRange::new(b, a), None);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn new(start: NepaliDate, end: NepaliDate) -> Option<Self> {
        // `start > end` calls `PartialOrd::gt`, a trait method, which a const fn can't call.
        // Comparing the fields in order gives the same answer as the derived `Ord`.
        let (sy, sm, sd) = (start.year(), start.month_number(), start.day());
        let (ey, em, ed) = (end.year(), end.month_number(), end.day());
        let after = sy > ey || (sy == ey && (sm > em || (sm == em && sd > ed)));
        if after {
            None
        } else {
            Some(Self { start, end })
        }
    }

    /// A range the caller has already ordered.
    pub(crate) const fn new_unchecked(start: NepaliDate, end: NepaliDate) -> Self {
        Self { start, end }
    }

    /// Every day of one month.
    ///
    /// # Errors
    ///
    /// The errors of [`NepaliDate::new`] for day 1 of that month: [`Error::OutOfRange`] for a
    /// year outside 1901 to 2199, [`Error::InvalidMonth`] for a month outside 1 to 12.
    ///
    /// ```
    /// use nepdate::NepaliDateRange;
    ///
    /// let r = NepaliDateRange::month(2081, 4)?;
    /// assert_eq!(r.end().to_string(), "2081/04/32");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the month may not exist"]
    pub fn month(year: u16, month: u8) -> Result<Self, Error> {
        let first = NepaliDate::new(year, month, 1)?;
        Ok(Self::new_unchecked(first, first.last_day_of_month()))
    }

    /// Every day of one BS year, Baishakh 1 to the last day of Chaitra (364 to 367 days, D-12).
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] for a year outside 1901 to 2199.
    ///
    /// ```
    /// use nepdate::NepaliDateRange;
    ///
    /// assert_eq!(NepaliDateRange::year(2087)?.len(), 367);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the year may be out of range"]
    pub fn year(year: u16) -> Result<Self, Error> {
        let first = NepaliDate::new(year, 1, 1)?;
        let chaitra = NepaliDate::new(year, 12, 1)?;
        Ok(Self::new_unchecked(first, chaitra.last_day_of_month()))
    }

    /// The first day.
    ///
    /// ```
    /// let r = nepdate::NepaliDateRange::month(2081, 4)?;
    /// assert_eq!(r.start().to_string(), "2081/04/01");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn start(self) -> NepaliDate {
        self.start
    }

    /// The last day (included).
    ///
    /// ```
    /// let r = nepdate::NepaliDateRange::month(2081, 4)?;
    /// assert_eq!(r.end().to_string(), "2081/04/32");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn end(self) -> NepaliDate {
        self.end
    }

    /// The number of days, at least 1.
    ///
    /// ```
    /// use nepdate::{NepaliDate, NepaliDateRange};
    ///
    /// let d = NepaliDate::new(2081, 1, 1)?;
    /// assert_eq!(NepaliDateRange::new(d, d).map(NepaliDateRange::len), Some(1));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    #[allow(clippy::len_without_is_empty)] // a range is never empty
    pub fn len(self) -> u32 {
        self.end.to_serial() - self.start.to_serial() + 1
    }

    /// Whether `date` is between `start` and `end`, both included.
    ///
    /// ```
    /// use nepdate::{NepaliDate, NepaliDateRange};
    ///
    /// let r = NepaliDateRange::month(2081, 4)?;
    /// assert!(r.contains(NepaliDate::new(2081, 4, 32)?));
    /// assert!(!r.contains(NepaliDate::new(2081, 5, 1)?));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn contains(self, date: NepaliDate) -> bool {
        self.start <= date && date <= self.end
    }

    /// An iterator over every day, in order. It also runs backwards (`rev`) and knows its length.
    ///
    /// ```
    /// use nepdate::{NepaliDateRange, Weekday};
    ///
    /// let r = NepaliDateRange::month(2081, 4)?;
    /// let saturdays = r.days().filter(|d| d.weekday() == Weekday::Saturday).count();
    /// assert_eq!(saturdays, 4);
    /// assert_eq!(r.days().rev().next(), Some(r.end()));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn days(self) -> Days {
        Days {
            front: self.start.to_serial(),
            back: self.end.to_serial(),
            done: false,
        }
    }
}

impl IntoIterator for NepaliDateRange {
    type Item = NepaliDate;
    type IntoIter = Days;

    fn into_iter(self) -> Days {
        self.days()
    }
}

/// The days of a [`NepaliDateRange`], from [`NepaliDateRange::days`].
///
/// A cursor over serial days: `front` and `back` are the next days to yield from each end, and
/// `done` is set once they have met, so a one-day range needs no sentinel value.
///
/// ```
/// let r = nepdate::NepaliDateRange::year(2081)?;
/// let mut days = r.days();
/// assert_eq!(days.len(), 366);
/// assert_eq!(days.next(), Some(r.start()));
/// assert_eq!(days.next_back(), Some(r.end()));
/// assert_eq!(days.len(), 364);
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct Days {
    front: u32,
    back: u32,
    done: bool,
}

impl Iterator for Days {
    type Item = NepaliDate;

    fn next(&mut self) -> Option<NepaliDate> {
        if self.done {
            return None;
        }
        let date = NepaliDate::from_serial(self.front).ok()?;
        if self.front == self.back {
            self.done = true;
        } else {
            self.front += 1;
        }
        Some(date)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if self.done {
            0
        } else {
            // At most 109_212 days, which fits in usize on every target Rust supports.
            usize::try_from(self.back - self.front + 1).unwrap_or(usize::MAX)
        };
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Days {
    fn next_back(&mut self) -> Option<NepaliDate> {
        if self.done {
            return None;
        }
        let date = NepaliDate::from_serial(self.back).ok()?;
        if self.front == self.back {
            self.done = true;
        } else {
            self.back -= 1;
        }
        Some(date)
    }
}

impl ExactSizeIterator for Days {}

impl FusedIterator for Days {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::{MAX_YEAR, MIN_YEAR};

    fn bs(y: u16, m: u8, d: u8) -> NepaliDate {
        NepaliDate::new(y, m, d).unwrap_or(NepaliDate::MIN)
    }

    #[test]
    fn month_and_year_lengths() {
        assert_eq!(
            NepaliDateRange::month(2081, 4).map(NepaliDateRange::len),
            Ok(32)
        );
        assert_eq!(
            NepaliDateRange::year(2087).map(NepaliDateRange::len),
            Ok(367)
        );
        assert_eq!(NepaliDateRange::year(2200), Err(Error::OutOfRange));
        assert_eq!(NepaliDateRange::year(1900), Err(Error::OutOfRange));
        assert_eq!(
            NepaliDateRange::month(2081, 13),
            Err(Error::InvalidMonth(13))
        );
    }

    #[test]
    fn every_year_length_is_the_sum_of_its_months() {
        for y in MIN_YEAR..=MAX_YEAR {
            let sum: u32 = (1..=12)
                .map(|m| NepaliDateRange::month(y, m).map_or(0, NepaliDateRange::len))
                .sum();
            assert_eq!(
                NepaliDateRange::year(y).map(NepaliDateRange::len),
                Ok(sum),
                "{y}"
            );
        }
    }

    #[test]
    fn new_orders_like_ord() {
        let dates = [
            bs(2080, 12, 30),
            bs(2081, 1, 1),
            bs(2081, 1, 2),
            bs(2081, 2, 1),
            NepaliDate::MIN,
            NepaliDate::MAX,
        ];
        for a in dates {
            for b in dates {
                assert_eq!(NepaliDateRange::new(a, b).is_some(), a <= b, "{a:?} {b:?}");
            }
        }
        let a = bs(2081, 1, 1);
        assert_eq!(
            NepaliDateRange::new(a, a).map(NepaliDateRange::len),
            Some(1)
        );
    }

    #[test]
    fn const_construction() {
        const R: Option<NepaliDateRange> = NepaliDateRange::new(NepaliDate::MIN, NepaliDate::MAX);
        assert_eq!(R.map(NepaliDateRange::len), Some(109_212));
    }

    #[test]
    fn days_iterator_both_ends_and_fused() {
        let r = NepaliDateRange::new_unchecked(bs(2081, 4, 1), bs(2081, 4, 32));
        assert_eq!(NepaliDateRange::month(2081, 4), Ok(r));
        let mut days = r.days();
        assert_eq!(days.len(), 32);
        assert_eq!(days.size_hint(), (32, Some(32)));
        assert_eq!(days.next_back(), Some(bs(2081, 4, 32)));
        assert_eq!(days.next(), Some(bs(2081, 4, 1)));
        assert_eq!(days.by_ref().count(), 30);
        assert_eq!(days.len(), 0);
        assert_eq!(days.next(), None);
        assert_eq!(days.next_back(), None);
        assert_eq!(days.next(), None);

        let one = NepaliDateRange::new_unchecked(NepaliDate::MAX, NepaliDate::MAX);
        let mut it = one.days();
        assert_eq!(it.next_back(), Some(NepaliDate::MAX));
        assert_eq!(it.next(), None);
        assert_eq!(one.into_iter().count(), 1);
    }
}
