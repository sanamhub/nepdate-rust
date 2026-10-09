//! `NepaliDate`, a valid BS date in the supported range (ADR-0004 §2).

use core::fmt;

use crate::calendar::{
    AD_MONTH_START, AD_MONTHS, BS_MONTH_AD, MAX_SERIAL, MAX_YEAR, MIN_YEAR, MONTH_INFO,
    month_length,
};
use crate::civil::greg_month_length;
use crate::{DateOrder, Error, GregorianDate, Lang, Month, Separator, Weekday};

/// `round(2^22 / (109_212 / 3_588))`: multiplying a serial by this and shifting right by 22
/// estimates its month index to within one (ADR-0004 §4, tested below).
const EST_MUL: u64 = 137_797;

/// A valid Bikram Sambat date from 1901-01-01 to 2199-12-30.
///
/// Four bytes, `Copy`, ordered by date. Every value is valid: the only ways to build one check
/// the calendar table.
///
/// ```
/// use nepdate::{Month, NepaliDate};
///
/// let date = NepaliDate::new(2081, 4, 15)?;
/// assert_eq!(date.month(), Month::Shrawan);
/// assert_eq!(date.to_string(), "2081/04/15");
/// # Ok::<(), nepdate::Error>(())
/// ```
///
/// The fields are private, so a date can't be built without the checks:
///
/// ```compile_fail
/// let date = nepdate::NepaliDate { year: 2081, month: nepdate::Month::Shrawan, day: 40 };
/// ```
// Field order is year, month, day so the derived `Ord` is date order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NepaliDate {
    year: u16,
    month: Month,
    day: u8,
}

impl NepaliDate {
    /// The first supported date, BS 1901-01-01 (AD 1844-04-11).
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::MIN.to_string(), "1901/01/01");
    /// ```
    pub const MIN: NepaliDate = NepaliDate::new_unchecked(1901, Month::Baishakh, 1);

    /// The last supported date, BS 2199-12-30 (AD 2143-04-15).
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::MAX.to_string(), "2199/12/30");
    /// ```
    pub const MAX: NepaliDate = NepaliDate::new_unchecked(2199, Month::Chaitra, 30);

    /// A date from year, month and day, checked against the calendar table.
    ///
    /// # Errors
    ///
    /// Checked in this order (ALGORITHM §3): [`Error::OutOfRange`] for a year outside 1901 to
    /// 2199, [`Error::InvalidMonth`], then [`Error::InvalidDay`].
    ///
    /// ```
    /// use nepdate::{Error, NepaliDate};
    ///
    /// assert!(NepaliDate::new(2081, 4, 32).is_ok());
    /// assert_eq!(
    ///     NepaliDate::new(2081, 4, 33),
    ///     Err(Error::InvalidDay { month_length: 32, day: 33 })
    /// );
    ///
    /// // const fn, so a date can be a constant:
    /// const NEW_YEAR: NepaliDate = match NepaliDate::new(2081, 1, 1) {
    ///     Ok(d) => d,
    ///     Err(_) => panic!(),
    /// };
    /// assert_eq!(NEW_YEAR.day(), 1);
    /// ```
    pub const fn new(year: u16, month: u8, day: u8) -> Result<Self, Error> {
        if year < MIN_YEAR || year > MAX_YEAR {
            return Err(Error::OutOfRange);
        }
        let Some(month_value) = Month::from_number(month) else {
            return Err(Error::InvalidMonth(month));
        };
        let len = month_length(year, month);
        if day < 1 || day > len {
            return Err(Error::InvalidDay {
                month_length: len,
                day,
            });
        }
        Ok(Self {
            year,
            month: month_value,
            day,
        })
    }

    /// A date the caller has already checked. Only for constants and proven values.
    pub(crate) const fn new_unchecked(year: u16, month: Month, day: u8) -> Self {
        Self { year, month, day }
    }

    /// The BS year, 1901 to 2199.
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::new(2081, 4, 15)?.year(), 2081);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn year(self) -> u16 {
        self.year
    }

    /// The month.
    ///
    /// ```
    /// use nepdate::{Month, NepaliDate};
    ///
    /// assert_eq!(NepaliDate::new(2081, 4, 15)?.month(), Month::Shrawan);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn month(self) -> Month {
        self.month
    }

    /// The month number, 1 to 12.
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::new(2081, 4, 15)?.month_number(), 4);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn month_number(self) -> u8 {
        self.month.number()
    }

    /// The day of the month, 1 to 32.
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::new(2081, 4, 15)?.day(), 15);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    /// Days in this date's month, 29 to 32.
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::new(2081, 4, 1)?.month_length(), 32);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn month_length(self) -> u8 {
        month_length(self.year, self.month.number())
    }

    /// The first day of this date's month.
    ///
    /// ```
    /// let d = nepdate::NepaliDate::new(2081, 4, 15)?;
    /// assert_eq!(d.first_day_of_month().to_string(), "2081/04/01");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn first_day_of_month(self) -> Self {
        Self::new_unchecked(self.year, self.month, 1)
    }

    /// The last day of this date's month.
    ///
    /// ```
    /// let d = nepdate::NepaliDate::new(2081, 4, 15)?;
    /// assert_eq!(d.last_day_of_month().to_string(), "2081/04/32");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn last_day_of_month(self) -> Self {
        Self::new_unchecked(self.year, self.month, self.month_length())
    }
}

impl NepaliDate {
    /// Index of this date's month in the calendar tables: `(y - 1901) * 12 + m - 1`.
    pub(crate) const fn month_index(self) -> usize {
        (self.year - MIN_YEAR) as usize * 12 + self.month.number() as usize - 1
    }

    /// Days since BS 1901-01-01 (ALGORITHM §4): one table load, no loop.
    // SAFETY-OF-INDEX: month_index() < MONTHS because every NepaliDate is in range.
    #[allow(clippy::indexing_slicing)]
    pub(crate) fn to_serial(self) -> u32 {
        (MONTH_INFO[self.month_index()] >> 2) + u32::from(self.day) - 1
    }

    /// The date `s` days after BS 1901-01-01: an estimate of the month plus at most one
    /// correction (ADR-0004 §4), no loop and no search.
    // SAFETY-OF-INDEX: s <= MAX_SERIAL keeps the estimate in 0..=3587 (tested for every serial),
    // and the correction moves it by one inside 0..MONTHS, so i and i + 1 index MONTH_INFO.
    #[allow(clippy::indexing_slicing, clippy::cast_possible_truncation)]
    pub(crate) fn from_serial(s: u32) -> Result<Self, Error> {
        if s > MAX_SERIAL {
            return Err(Error::OutOfRange);
        }
        let mut i = ((u64::from(s) * EST_MUL) >> 22) as usize;
        if MONTH_INFO[i] >> 2 > s {
            i -= 1;
        } else if MONTH_INFO[i + 1] >> 2 <= s {
            i += 1;
        }
        // i < MONTHS = 3588, so i / 12 < 299 and the casts can't truncate.
        let year = MIN_YEAR + (i / 12) as u16;
        let month = Month::ALL[i % 12];
        let day = (s - (MONTH_INFO[i] >> 2) + 1) as u8;
        Ok(Self::new_unchecked(year, month, day))
    }

    /// A BS date from an AD date.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidGregorian`] when the AD date doesn't exist, then [`Error::OutOfRange`]
    /// outside AD 1844-04-11 to 2143-04-15.
    ///
    /// ```
    /// use nepdate::NepaliDate;
    ///
    /// assert_eq!(NepaliDate::from_gregorian(2024, 4, 13)?, NepaliDate::new(2081, 1, 1)?);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the conversion may fail"]
    // SAFETY-OF-INDEX: j is checked to be in 0..AD_MONTHS before the load.
    #[allow(
        clippy::indexing_slicing,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn from_gregorian(year: i32, month: u8, day: u8) -> Result<Self, Error> {
        let ad = GregorianDate::new(year, month, day)?;
        // Range check on the year first, so the month index below can't overflow.
        if !(1844..=2143).contains(&ad.year()) {
            return Err(Error::OutOfRange);
        }
        let j = (ad.year() - 1844) * 12 + i32::from(ad.month()) - 4;
        if j < 0 || j as usize >= AD_MONTHS {
            return Err(Error::OutOfRange);
        }
        let s = AD_MONTH_START[j as usize] + i32::from(ad.day()) - 1;
        if s < 0 {
            return Err(Error::OutOfRange);
        }
        Self::from_serial(s as u32)
    }

    /// The AD date: the AD date of the month's first day plus `day - 1`, with at most one carry
    /// into the next AD month (the offset is at most 31, every AD month has at least 28 days).
    ///
    /// ```
    /// use nepdate::NepaliDate;
    ///
    /// assert_eq!(NepaliDate::new(2081, 1, 1)?.to_gregorian().to_string(), "2024-04-13");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    // SAFETY-OF-INDEX: month_index() < MONTHS because every NepaliDate is in range.
    #[allow(
        clippy::indexing_slicing,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap
    )]
    pub fn to_gregorian(self) -> GregorianDate {
        let v = BS_MONTH_AD[self.month_index()];
        let mut year = (v >> 9) as i32;
        let mut month = ((v >> 5) & 0xF) as u8;
        let mut day = (v & 0x1F) as u8 + self.day - 1;
        let len = greg_month_length(i64::from(year), month);
        if day > len {
            day -= len;
            if month == 12 {
                month = 1;
                year += 1;
            } else {
                month += 1;
            }
        }
        GregorianDate::new_unchecked(year, month, day)
    }

    /// The day of the week (ALGORITHM §7: BS 1901-01-01 was a Thursday).
    ///
    /// ```
    /// use nepdate::{NepaliDate, Weekday};
    ///
    /// assert_eq!(NepaliDate::new(2081, 1, 1)?.weekday(), Weekday::Saturday);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    #[allow(clippy::indexing_slicing)] // % 7 is 0..=6
    pub fn weekday(self) -> Weekday {
        Weekday::ALL[((self.to_serial() + 4) % 7) as usize]
    }

    /// The day of the year, 1 for Baishakh 1.
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::new(2081, 2, 1)?.day_of_year(), 32);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    // SAFETY-OF-INDEX: the year's first month index is at most month_index() < MONTHS.
    // A year has at most 367 days, so the cast can't truncate.
    #[allow(clippy::indexing_slicing, clippy::cast_possible_truncation)]
    pub fn day_of_year(self) -> u16 {
        let first = MONTH_INFO[(self.year - MIN_YEAR) as usize * 12] >> 2;
        (self.to_serial() - first + 1) as u16
    }

    /// The next day, or `None` after BS 2199-12-30.
    ///
    /// ```
    /// use nepdate::NepaliDate;
    ///
    /// assert_eq!(NepaliDate::new(2080, 12, 30)?.succ(), Some(NepaliDate::new(2081, 1, 1)?));
    /// assert_eq!(NepaliDate::MAX.succ(), None);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn succ(self) -> Option<Self> {
        Self::from_serial(self.to_serial() + 1).ok()
    }

    /// The previous day, or `None` before BS 1901-01-01.
    ///
    /// ```
    /// assert_eq!(nepdate::NepaliDate::MIN.pred(), None);
    /// ```
    #[must_use]
    pub fn pred(self) -> Option<Self> {
        let s = self.to_serial().checked_sub(1)?;
        Self::from_serial(s).ok()
    }

    /// Days from `self` to `other`: positive when `other` is later (D-07).
    ///
    /// ```
    /// use nepdate::NepaliDate;
    ///
    /// assert_eq!(NepaliDate::MIN.days_until(NepaliDate::MAX), 109_211);
    /// ```
    #[must_use]
    #[allow(clippy::cast_possible_wrap)] // serials are at most 109_211
    pub fn days_until(self, other: Self) -> i32 {
        other.to_serial() as i32 - self.to_serial() as i32
    }
}

impl TryFrom<GregorianDate> for NepaliDate {
    type Error = Error;

    fn try_from(ad: GregorianDate) -> Result<Self, Error> {
        Self::from_gregorian(ad.year(), ad.month(), ad.day())
    }
}

impl From<NepaliDate> for GregorianDate {
    fn from(date: NepaliDate) -> Self {
        date.to_gregorian()
    }
}

impl fmt::Display for NepaliDate {
    /// `YYYY/MM/DD`, zero padded, ASCII digits (FORMATTING §2).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.short(DateOrder::Ymd, Separator::Slash, true, Lang::En)
            .write_to(f)
    }
}

impl fmt::Debug for NepaliDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NepaliDate({:04}-{:02}-{:02})",
            self.year,
            self.month.number(),
            self.day
        )
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use core::mem::size_of;
    use std::format;

    #[test]
    fn error_precedence() {
        assert_eq!(NepaliDate::new(1900, 13, 40), Err(Error::OutOfRange));
        assert_eq!(NepaliDate::new(2080, 13, 40), Err(Error::InvalidMonth(13)));
        assert_eq!(
            NepaliDate::new(2080, 1, 32),
            Err(Error::InvalidDay {
                month_length: 31,
                day: 32
            })
        );
        assert!(NepaliDate::new(2081, 4, 32).is_ok());
        assert_eq!(
            NepaliDate::new(2081, 4, 33),
            Err(Error::InvalidDay {
                month_length: 32,
                day: 33
            })
        );
        assert_eq!(NepaliDate::new(2081, 0, 1), Err(Error::InvalidMonth(0)));
        assert_eq!(NepaliDate::new(2200, 1, 1), Err(Error::OutOfRange));
        assert_eq!(
            NepaliDate::new(2081, 1, 0),
            Err(Error::InvalidDay {
                month_length: 31,
                day: 0
            })
        );
    }

    #[test]
    fn sizes() {
        assert_eq!(size_of::<NepaliDate>(), 4);
        assert_eq!(size_of::<Option<NepaliDate>>(), 4);
        assert!(size_of::<Result<NepaliDate, Error>>() <= 4);
    }

    #[test]
    fn ordering() {
        let a = NepaliDate::new(2080, 12, 30);
        let b = NepaliDate::new(2081, 1, 1);
        assert!(matches!((a, b), (Ok(a), Ok(b)) if a < b));
        assert!(NepaliDate::MIN < NepaliDate::MAX);
    }

    #[test]
    fn const_construction() {
        const D: Result<NepaliDate, Error> = NepaliDate::new(2081, 4, 15);
        assert!(matches!(D, Ok(d) if d.day() == 15));
    }

    #[test]
    fn min_max_are_valid() {
        assert_eq!(NepaliDate::new(1901, 1, 1), Ok(NepaliDate::MIN));
        assert_eq!(NepaliDate::new(2199, 12, 30), Ok(NepaliDate::MAX));
        assert_eq!(NepaliDate::MAX.month_length(), 30);
    }

    #[test]
    fn serial_ends() {
        assert_eq!(NepaliDate::MIN.to_serial(), 0);
        assert_eq!(NepaliDate::MAX.to_serial(), 109_211);
        assert_eq!(NepaliDate::from_serial(109_212), Err(Error::OutOfRange));
        assert_eq!(NepaliDate::from_serial(0), Ok(NepaliDate::MIN));
    }

    #[test]
    fn month_estimate_is_off_by_at_most_one() {
        let starts: std::vec::Vec<u32> = MONTH_INFO.iter().map(|v| v >> 2).collect();
        for s in 0..=MAX_SERIAL {
            let truth = starts.partition_point(|&start| start <= s) - 1;
            #[allow(clippy::cast_possible_truncation)]
            let estimate = ((u64::from(s) * EST_MUL) >> 22) as usize;
            assert!(
                estimate.abs_diff(truth) <= 1,
                "serial {s}: {estimate} vs {truth}"
            );
            assert!(estimate < crate::calendar::MONTHS, "serial {s}");
        }
    }

    #[test]
    fn gregorian_edges() {
        assert_eq!(NepaliDate::from_gregorian(1844, 4, 11), Ok(NepaliDate::MIN));
        assert_eq!(NepaliDate::from_gregorian(2143, 4, 15), Ok(NepaliDate::MAX));
        for (y, m, d) in [
            (1844, 4, 10),
            (1844, 4, 1),
            (2143, 4, 16),
            (1844, 3, 31),
            (2143, 5, 1),
        ] {
            assert_eq!(
                NepaliDate::from_gregorian(y, m, d),
                Err(Error::OutOfRange),
                "{y}-{m}-{d}"
            );
        }
        assert_eq!(
            NepaliDate::from_gregorian(2023, 2, 29),
            Err(Error::InvalidGregorian)
        );
        assert_eq!(
            NepaliDate::from_gregorian(i32::MAX, 1, 1),
            Err(Error::OutOfRange)
        );
        assert_eq!(
            NepaliDate::from_gregorian(i32::MIN, 1, 1),
            Err(Error::OutOfRange)
        );
        assert_eq!(NepaliDate::MAX.succ(), None);
        assert_eq!(NepaliDate::MIN.pred(), None);
        assert_eq!(NepaliDate::MIN.days_until(NepaliDate::MAX), 109_211);
        assert_eq!(NepaliDate::MAX.days_until(NepaliDate::MIN), -109_211);
    }

    #[test]
    fn display_and_debug() {
        let d = NepaliDate::new_unchecked(2081, Month::Shrawan, 5);
        assert_eq!(format!("{d}"), "2081/04/05");
        assert_eq!(format!("{d:?}"), "NepaliDate(2081-04-05)");
    }

    #[test]
    fn every_valid_day_builds_and_every_invalid_day_fails() {
        for year in MIN_YEAR..=MAX_YEAR {
            for month in 1..=12u8 {
                let len = month_length(year, month);
                for day in 1..=len {
                    assert!(NepaliDate::new(year, month, day).is_ok());
                }
                assert_eq!(
                    NepaliDate::new(year, month, len + 1),
                    Err(Error::InvalidDay {
                        month_length: len,
                        day: len + 1
                    })
                );
            }
        }
    }
}
