//! `NepaliDateRange`, an inclusive span of days, and its iterators (D-01, D-16).

use core::cmp::{max, min};
use core::fmt;
use core::iter::FusedIterator;

use crate::{DateOrder, Error, Lang, NepaliDate, Separator};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

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

    /// A range of one date.
    ///
    /// ```
    /// use nepdate::{NepaliDate, NepaliDateRange};
    ///
    /// let d = NepaliDate::new(2081, 4, 15)?;
    /// assert_eq!(NepaliDateRange::single(d), NepaliDateRange::new(d, d).unwrap());
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn single(date: NepaliDate) -> Self {
        Self {
            start: date,
            end: date,
        }
    }

    /// A range of `days` days starting at `start`; `days` must be at least 1.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] when `days` is 0 or the last day is outside the supported range.
    ///
    /// ```
    /// use nepdate::{NepaliDate, NepaliDateRange};
    ///
    /// let start = NepaliDate::new(2081, 4, 15)?;
    /// assert_eq!(NepaliDateRange::from_day_count(start, 5)?.end().to_string(), "2081/04/19");
    /// assert!(NepaliDateRange::from_day_count(start, 0).is_err());
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the range may be out of range"]
    pub fn from_day_count(start: NepaliDate, days: u32) -> Result<Self, Error> {
        if days == 0 {
            return Err(Error::OutOfRange);
        }
        let offset = i32::try_from(days - 1).map_err(|_| Error::OutOfRange)?;
        Ok(Self::new_unchecked(start, start.checked_add_days(offset)?))
    }

    /// Whether this range fully contains `other`.
    ///
    /// ```
    /// use nepdate::{NepaliDateRange};
    ///
    /// let outer = NepaliDateRange::month(2081, 4)?;
    /// let inner = NepaliDateRange::new(
    ///     nepdate::NepaliDate::new(2081, 4, 10)?,
    ///     nepdate::NepaliDate::new(2081, 4, 20)?,
    /// ).unwrap();
    /// assert!(outer.contains_range(inner));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn contains_range(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    /// Whether the two ranges share at least one date.
    ///
    /// ```
    /// use nepdate::{NepaliDate, NepaliDateRange};
    ///
    /// let a = NepaliDateRange::month(2081, 4)?;
    /// let b = NepaliDateRange::single(NepaliDate::new(2081, 4, 32)?);
    /// assert!(a.overlaps(b));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    /// Whether one range starts the day after the other ends.
    ///
    /// `false` at the edges of the supported range, where the next day does not exist (D-16).
    ///
    /// ```
    /// use nepdate::{NepaliDate, NepaliDateRange};
    ///
    /// let a = NepaliDateRange::new(
    ///     NepaliDate::new(2081, 4, 10)?,
    ///     NepaliDate::new(2081, 4, 20)?,
    /// ).unwrap();
    /// let b = NepaliDateRange::single(NepaliDate::new(2081, 4, 21)?);
    /// assert!(a.is_adjacent_to(b));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn is_adjacent_to(self, other: Self) -> bool {
        if let Some(next) = self.end.succ() {
            if other.start == next {
                return true;
            }
        }
        if let Some(next) = other.end.succ() {
            return self.start == next;
        }
        false
    }

    /// The dates in both ranges, or `None` when they do not overlap.
    ///
    /// ```
    /// use nepdate::{NepaliDate, NepaliDateRange};
    ///
    /// let a = NepaliDateRange::month(2081, 4)?;
    /// let b = NepaliDateRange::single(NepaliDate::new(2081, 4, 10)?);
    /// assert_eq!(a.intersect(b), Some(b));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn intersect(self, other: Self) -> Option<Self> {
        let start = max(self.start, other.start);
        let end = min(self.end, other.end);
        (start <= end).then_some(Self::new_unchecked(start, end))
    }

    /// The smallest range that covers both.
    ///
    /// ```
    /// use nepdate::{NepaliDate, NepaliDateRange};
    ///
    /// let a = NepaliDateRange::month(2081, 4)?;
    /// let b = NepaliDateRange::single(NepaliDate::new(2081, 5, 1)?);
    /// assert_eq!(a.union(b).end(), b.end());
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self::new_unchecked(min(self.start, other.start), max(self.end, other.end))
    }

    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    /// This range with `other` removed: zero, one or two ranges.
    ///
    /// ```
    /// use nepdate::{NepaliDate, NepaliDateRange};
    ///
    /// let r = NepaliDateRange::month(2081, 4)?;
    /// let hole = NepaliDateRange::new(
    ///     NepaliDate::new(2081, 4, 10)?,
    ///     NepaliDate::new(2081, 4, 12)?,
    /// ).unwrap();
    /// assert_eq!(r.except(hole).len(), 2);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn except(self, other: Self) -> Vec<Self> {
        if !self.overlaps(other) {
            return alloc::vec![self];
        }
        let mut out = Vec::with_capacity(2);
        if self.start < other.start {
            if let Some(end) = other.start.pred() {
                out.push(Self::new_unchecked(self.start, end));
            }
        }
        if other.end < self.end {
            if let Some(start) = other.end.succ() {
                out.push(Self::new_unchecked(start, self.end));
            }
        }
        out
    }

    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    /// One range per BS month touched, clipped at the range ends.
    ///
    /// ```
    /// use nepdate::NepaliDateRange;
    ///
    /// let r = NepaliDateRange::new(
    ///     nepdate::NepaliDate::new(2081, 4, 30)?,
    ///     nepdate::NepaliDate::new(2081, 6, 2)?,
    /// ).unwrap();
    /// assert_eq!(r.split_by_month().len(), 3);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn split_by_month(self) -> Vec<Self> {
        let mut out = Vec::new();
        let mut year = self.start.year();
        let mut month = self.start.month_number();
        while let Ok(first) = NepaliDate::new(year, month, 1) {
            let last = first.last_day_of_month();
            out.push(Self::new_unchecked(
                max(self.start, first),
                min(self.end, last),
            ));
            // Stop at the range end, so the loop never asks for a month past the data.
            if last >= self.end {
                break;
            }
            if month == 12 {
                year += 1;
                month = 1;
            } else {
                month += 1;
            }
        }
        out
    }

    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    /// One range per fiscal quarter touched, clipped at the range ends.
    ///
    /// ```
    /// use nepdate::NepaliDateRange;
    ///
    /// let r = NepaliDateRange::new(
    ///     nepdate::NepaliDate::new(2081, 3, 30)?,
    ///     nepdate::NepaliDate::new(2081, 7, 2)?,
    /// ).unwrap();
    /// assert_eq!(r.split_by_fiscal_quarter().len(), 3);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn split_by_fiscal_quarter(self) -> Vec<Self> {
        let mut out = Vec::new();
        let mut cursor = self.start;
        loop {
            let quarter = cursor.quarter();
            let Ok(q) = cursor.fiscal_year().quarter_range(quarter) else {
                out.push(Self::new_unchecked(cursor, self.end));
                break;
            };
            out.push(Self::new_unchecked(
                max(self.start, q.start),
                min(self.end, q.end),
            ));
            if q.end >= self.end {
                break;
            }
            match q.end.succ() {
                Some(next) => cursor = next,
                None => break,
            }
        }
        out
    }

    /// Every `interval` days from the start, including the start.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] when `interval` is 0.
    ///
    /// ```
    /// use nepdate::NepaliDateRange;
    ///
    /// let r = NepaliDateRange::month(2081, 4)?;
    /// assert_eq!(r.dates_with_interval(3)?.count(), 11);
    /// assert!(r.dates_with_interval(0).is_err());
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the interval may be 0"]
    pub fn dates_with_interval(self, interval: u32) -> Result<DatesWithInterval, Error> {
        if interval == 0 {
            return Err(Error::OutOfRange);
        }
        Ok(DatesWithInterval {
            next: Some(self.start.to_serial()),
            end: self.end.to_serial(),
            interval,
        })
    }

    /// Every day except Saturday, and except Sunday when `exclude_sunday` is true.
    ///
    /// ```
    /// use nepdate::NepaliDateRange;
    ///
    /// let r = NepaliDateRange::month(2081, 4)?;
    /// assert!(r.working_days(false).count() < r.len() as usize);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    pub fn working_days(self, exclude_sunday: bool) -> impl Iterator<Item = NepaliDate> + Clone {
        self.days().filter(move |d| {
            let i = d.weekday().index_from_sunday();
            i != 6 && !(exclude_sunday && i == 0)
        })
    }

    /// Every Saturday, and every Sunday when `include_sunday` is true.
    ///
    /// ```
    /// use nepdate::NepaliDateRange;
    ///
    /// let r = NepaliDateRange::month(2081, 4)?;
    /// assert!(r.weekend_days(true).count() >= r.weekend_days(false).count());
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    pub fn weekend_days(self, include_sunday: bool) -> impl Iterator<Item = NepaliDate> + Clone {
        self.days().filter(move |d| {
            let i = d.weekday().index_from_sunday();
            i == 6 || (include_sunday && i == 0)
        })
    }

    /// The two dates with the same short options, separated by `" - "`.
    ///
    /// ```
    /// use nepdate::{DateOrder, Lang, NepaliDateRange, Separator};
    ///
    /// let r = NepaliDateRange::month(2081, 4)?;
    /// assert_eq!(
    ///     r.to_string_with(DateOrder::Dmy, Separator::Dash, Lang::En).to_string(),
    ///     "01-04-2081 - 32-04-2081"
    /// );
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn to_string_with(self, order: DateOrder, sep: Separator, lang: Lang) -> RangeFormat {
        RangeFormat {
            range: self,
            order,
            sep,
            lang,
        }
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

impl fmt::Display for NepaliDateRange {
    /// `"start - end"` in the default short form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::format::write_once(f, |out| {
            self.start
                .short(DateOrder::Ymd, Separator::Slash, true, Lang::En)
                .write_to(out)?;
            out.write_str(" - ")?;
            self.end
                .short(DateOrder::Ymd, Separator::Slash, true, Lang::En)
                .write_to(out)
        })
    }
}

/// A range written with the short-date options, from [`NepaliDateRange::to_string_with`].
///
/// ```
/// use nepdate::{DateOrder, Lang, NepaliDateRange, Separator};
///
/// let r = NepaliDateRange::month(2081, 4)?;
/// assert_eq!(
///     r.to_string_with(DateOrder::Ymd, Separator::Dash, Lang::En).to_string(),
///     "2081-04-01 - 2081-04-32"
/// );
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct RangeFormat {
    range: NepaliDateRange,
    order: DateOrder,
    sep: Separator,
    lang: Lang,
}

impl RangeFormat {
    /// Writes the two formatted dates to `out` without allocating.
    ///
    /// # Errors
    ///
    /// Whatever `out.write_str` returns.
    pub fn write_to<W: fmt::Write + ?Sized>(&self, out: &mut W) -> fmt::Result {
        self.range
            .start
            .short(self.order, self.sep, true, self.lang)
            .write_to(out)?;
        out.write_str(" - ")?;
        self.range
            .end
            .short(self.order, self.sep, true, self.lang)
            .write_to(out)
    }
}

impl fmt::Display for RangeFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::format::write_once(f, |out| self.write_to(out))
    }
}

/// The dates of a range at a fixed interval, from
/// [`NepaliDateRange::dates_with_interval`].
///
/// ```
/// use nepdate::NepaliDateRange;
///
/// let r = NepaliDateRange::month(2081, 4)?;
/// let days: Vec<String> = r.dates_with_interval(10)?.map(|d| d.to_string()).collect();
/// assert_eq!(days, ["2081/04/01", "2081/04/11", "2081/04/21", "2081/04/31"]);
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct DatesWithInterval {
    next: Option<u32>,
    end: u32,
    interval: u32,
}

impl Iterator for DatesWithInterval {
    type Item = NepaliDate;

    fn next(&mut self) -> Option<NepaliDate> {
        let serial = self.next?;
        let date = NepaliDate::from_serial(serial).ok()?;
        self.next = serial
            .checked_add(self.interval)
            .filter(|&next| next <= self.end);
        Some(date)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = match self.next {
            Some(serial) => {
                usize::try_from((self.end - serial) / self.interval + 1).unwrap_or(usize::MAX)
            }
            None => 0,
        };
        (n, Some(n))
    }
}

impl ExactSizeIterator for DatesWithInterval {}

impl FusedIterator for DatesWithInterval {}

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
