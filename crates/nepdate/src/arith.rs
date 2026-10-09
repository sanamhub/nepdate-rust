//! Day, month and year arithmetic (ALGORITHM §8) and the calendar breakdown between two dates
//! (ALGORITHM §8a). Every operation goes through the serial day or the month index, so nothing
//! loops over days or months (ADR-0004 §4).

use crate::calendar::{MAX_SERIAL, MIN_YEAR, MONTHS, month_length};
use crate::{Error, Month, NepaliDate};

/// What [`NepaliDate::checked_add_months`] does when the day doesn't exist in the target month.
///
/// ```
/// use nepdate::{MonthOverflow, NepaliDate};
///
/// let d = NepaliDate::new(2081, 4, 32)?;
/// let clamp = d.checked_add_months(2, MonthOverflow::Clamp)?;
/// let overflow = d.checked_add_months(2, MonthOverflow::Overflow)?;
/// assert_eq!(clamp.to_string(), "2081/06/30");
/// assert_eq!(overflow.to_string(), "2081/07/02");
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MonthOverflow {
    /// Use the last day of the target month. C# `AddMonths(n)`.
    #[default]
    Clamp,
    /// Carry the extra days into the next month. C# `AddMonths(n, awayFromMonthEnd: true)`.
    Overflow,
}

/// The calendar distance between two dates (ALGORITHM §8a), from [`NepaliDate::diff`].
///
/// `years`, `months` and `days` are magnitudes; `negative` says the second date is earlier.
/// `total_days` is signed, like [`NepaliDate::days_until`].
///
/// ```
/// use nepdate::NepaliDate;
///
/// let d = NepaliDate::new(2080, 1, 1)?.diff(NepaliDate::new(2081, 4, 15)?);
/// assert_eq!((d.negative, d.years, d.months, d.days, d.total_days), (false, 1, 3, 14, 473));
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DateDiff {
    /// `true` when the second date is before the first.
    pub negative: bool,
    /// Whole years, 0 to 298.
    pub years: u16,
    /// Whole months after the years, 0 to 11.
    pub months: u8,
    /// Days after the months, 0 to 32.
    pub days: u8,
    /// Signed day count from the first date to the second.
    pub total_days: i32,
}

impl NepaliDate {
    /// The date `days` days later, or earlier for a negative `days` (D-03).
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] when the result is outside 1901-01-01 to 2199-12-30.
    ///
    /// ```
    /// use nepdate::{Error, NepaliDate};
    ///
    /// let d = NepaliDate::new(2080, 12, 30)?;
    /// assert_eq!(d.checked_add_days(5)?.to_string(), "2081/01/05");
    /// assert_eq!(NepaliDate::MAX.checked_add_days(1), Err(Error::OutOfRange));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the result may be out of range"]
    pub fn checked_add_days(self, days: i32) -> Result<Self, Error> {
        // i64, so that `i32::MAX` days is an out-of-range result and not an overflow.
        let s = i64::from(self.to_serial()) + i64::from(days);
        if s < 0 || s > i64::from(MAX_SERIAL) {
            return Err(Error::OutOfRange);
        }
        // The check above keeps s in 0..=MAX_SERIAL, so it fits in u32.
        let s = u32::try_from(s).map_err(|_| Error::OutOfRange)?;
        Self::from_serial(s)
    }

    /// The date `months` months later, or earlier for a negative `months` (ALGORITHM §8, D-02).
    ///
    /// When the day doesn't exist in the target month, `overflow` decides: [`MonthOverflow::Clamp`]
    /// gives the month's last day, [`MonthOverflow::Overflow`] carries into the next month.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] when the target month, or the month an overflow carries into, is
    /// outside 1901 to 2199.
    ///
    /// ```
    /// use nepdate::{MonthOverflow, NepaliDate};
    ///
    /// let d = NepaliDate::new(2081, 4, 32)?;
    /// assert_eq!(d.checked_add_months(5, MonthOverflow::Clamp)?.to_string(), "2081/09/29");
    /// assert_eq!(d.checked_add_months(5, MonthOverflow::Overflow)?.to_string(), "2081/10/03");
    /// assert_eq!(d.checked_add_months(-2, MonthOverflow::Clamp)?.to_string(), "2081/02/32");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the result may be out of range"]
    pub fn checked_add_months(self, months: i32, overflow: MonthOverflow) -> Result<Self, Error> {
        self.add_months_i64(i64::from(months), overflow)
    }

    /// The date `years` years later, or earlier for a negative `years`: `12 * years` months, with
    /// the same `overflow` rule as [`checked_add_months`](Self::checked_add_months).
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] when the result is outside the supported range.
    ///
    /// ```
    /// use nepdate::{MonthOverflow, NepaliDate};
    ///
    /// let d = NepaliDate::new(2081, 4, 32)?;
    /// assert_eq!(d.checked_add_years(1, MonthOverflow::Clamp)?.to_string(), "2082/04/32");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the result may be out of range"]
    pub fn checked_add_years(self, years: i32, overflow: MonthOverflow) -> Result<Self, Error> {
        // 12 * i32::MAX doesn't fit in i32 but does in i64.
        self.add_months_i64(12 * i64::from(years), overflow)
    }

    /// ALGORITHM §8 `add_months` with the month count in `i64`.
    #[allow(clippy::cast_possible_wrap)]
    fn add_months_i64(self, months: i64, overflow: MonthOverflow) -> Result<Self, Error> {
        // month_index() is below MONTHS = 3588, so the cast is lossless and the sum can't
        // overflow for any i64 that came from an i32 (at most 12 * 2^31).
        let total = self.month_index() as i64 + months;
        let Some(i) = usize::try_from(total).ok().filter(|&i| i < MONTHS) else {
            return Err(Error::OutOfRange);
        };
        let target = self.in_month_clamped(i);
        if target.day() == self.day() || overflow == MonthOverflow::Clamp {
            return Ok(target);
        }
        // Overflow: the day after the target month's last day, plus the days that didn't fit.
        // At most 3 days spill (32 - 29), and every month has at least 29 days.
        let spill = self.day() - target.day();
        if i + 1 >= MONTHS {
            return Err(Error::OutOfRange);
        }
        let next = Self::first_of_month_index(i + 1);
        Ok(Self::new_unchecked(next.year(), next.month(), spill))
    }

    /// The first day of month index `i` (`(y - 1901) * 12 + m - 1`). `i` must be below `MONTHS`.
    // SAFETY-OF-INDEX: i % 12 is 0..=11. i < MONTHS = 3588 keeps i / 12 below 299, so the year
    // cast can't truncate.
    #[allow(clippy::indexing_slicing, clippy::cast_possible_truncation)]
    fn first_of_month_index(i: usize) -> Self {
        Self::new_unchecked(MIN_YEAR + (i / 12) as u16, Month::ALL[i % 12], 1)
    }

    /// This date's day in month index `i`, or that month's last day when the day doesn't exist
    /// there. `i` must be below `MONTHS`.
    fn in_month_clamped(self, i: usize) -> Self {
        let first = Self::first_of_month_index(i);
        let len = month_length(first.year(), first.month_number());
        Self::new_unchecked(first.year(), first.month(), self.day().min(len))
    }

    /// Years, months and days from `self` to `other` (ALGORITHM §8a).
    ///
    /// The months are counted with [`MonthOverflow::Clamp`]: `n` is the largest month count with
    /// `self + n months <= other`, and the days are what is left. When `other` is earlier the
    /// dates are swapped and `negative` is `true`.
    ///
    /// ```
    /// use nepdate::NepaliDate;
    ///
    /// let a = NepaliDate::new(2081, 4, 32)?;
    /// let b = NepaliDate::new(2081, 6, 1)?;
    /// let d = a.diff(b);
    /// assert_eq!((d.years, d.months, d.days, d.total_days), (0, 1, 1, 32));
    /// assert!(b.diff(a).negative);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    // A month count below 3588 and a rest of 0 to 32 days fit in the narrow fields.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn diff(self, other: Self) -> DateDiff {
        let total_days = self.days_until(other);
        let (a, b) = if self > other {
            (other, self)
        } else {
            (self, other)
        };
        // a <= b, so b's month index is at least a's, and n0 is the month count that lands in
        // b's month. The clamped date there is either <= b or one month too far.
        let mut n = b.month_index() - a.month_index();
        let mut landed = a.in_month_clamped(b.month_index());
        if landed > b {
            n -= 1;
            landed = a.in_month_clamped(b.month_index() - 1);
        }
        DateDiff {
            negative: self > other,
            years: (n / 12) as u16,
            months: (n % 12) as u8,
            days: landed.days_until(b) as u8,
            total_days,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bs(y: u16, m: u8, d: u8) -> NepaliDate {
        NepaliDate::new(y, m, d).unwrap_or(NepaliDate::MIN)
    }

    fn months(d: NepaliDate, n: i32, o: MonthOverflow) -> Result<NepaliDate, Error> {
        d.checked_add_months(n, o)
    }

    #[test]
    fn algorithm_8_examples() {
        use MonthOverflow::{Clamp, Overflow};
        let d = bs(2081, 4, 32);
        assert_eq!(months(d, 2, Clamp), Ok(bs(2081, 6, 30)));
        assert_eq!(months(d, 2, Overflow), Ok(bs(2081, 7, 2)));
        assert_eq!(months(d, -2, Clamp), Ok(bs(2081, 2, 32)));
        assert_eq!(months(d, -2, Overflow), Ok(bs(2081, 2, 32)));
        assert_eq!(months(d, 5, Clamp), Ok(bs(2081, 9, 29)));
        assert_eq!(months(d, 5, Overflow), Ok(bs(2081, 10, 3)));
        assert_eq!(bs(2080, 12, 30).checked_add_days(5), Ok(bs(2081, 1, 5)));
        assert_eq!(bs(2081, 1, 5).checked_add_days(-5), Ok(bs(2080, 12, 30)));
    }

    #[test]
    fn months_across_years_and_zero() {
        use MonthOverflow::Clamp;
        assert_eq!(months(bs(2080, 11, 15), 5, Clamp), Ok(bs(2081, 4, 15)));
        assert_eq!(months(bs(2080, 3, 15), -5, Clamp), Ok(bs(2079, 10, 15)));
        assert_eq!(months(bs(2081, 4, 32), 0, Clamp), Ok(bs(2081, 4, 32)));
        assert_eq!(
            bs(2081, 4, 32).checked_add_years(-1, Clamp),
            Ok(bs(2080, 4, 32))
        );
    }

    #[test]
    fn extreme_inputs_are_out_of_range() {
        use MonthOverflow::{Clamp, Overflow};
        let max = NepaliDate::MAX;
        let min = NepaliDate::MIN;
        assert_eq!(max.checked_add_days(1), Err(Error::OutOfRange));
        assert_eq!(min.checked_add_days(-1), Err(Error::OutOfRange));
        assert_eq!(min.checked_add_days(i32::MAX), Err(Error::OutOfRange));
        assert_eq!(max.checked_add_days(i32::MIN), Err(Error::OutOfRange));
        assert_eq!(min.checked_add_days(109_211), Ok(max));
        assert_eq!(max.checked_add_days(-109_211), Ok(min));
        for o in [Clamp, Overflow] {
            assert_eq!(min.checked_add_months(i32::MIN, o), Err(Error::OutOfRange));
            assert_eq!(max.checked_add_months(i32::MAX, o), Err(Error::OutOfRange));
            assert_eq!(min.checked_add_years(i32::MIN, o), Err(Error::OutOfRange));
            assert_eq!(max.checked_add_years(i32::MAX, o), Err(Error::OutOfRange));
            assert_eq!(min.checked_add_months(-1, o), Err(Error::OutOfRange));
            assert_eq!(max.checked_add_months(1, o), Err(Error::OutOfRange));
            assert_eq!(
                min.checked_add_months(3587, o).map(NepaliDate::year),
                Ok(2199)
            );
        }
    }

    #[test]
    fn overflow_past_the_last_month_is_out_of_range() {
        // Chaitra 2199 has 30 days and Bhadra 2199 has 31, so day 31 spills past the table.
        assert_eq!(bs(2199, 12, 1).month_length(), 30);
        let d = bs(2199, 5, 31);
        assert_eq!(
            d.checked_add_months(7, MonthOverflow::Overflow),
            Err(Error::OutOfRange)
        );
        assert_eq!(
            d.checked_add_months(7, MonthOverflow::Clamp),
            Ok(NepaliDate::MAX)
        );
    }

    #[test]
    fn diff_examples_from_algorithm_8a() {
        let cases = [
            (bs(2080, 1, 1), bs(2081, 4, 15), (false, 1, 3, 14, 473)),
            (bs(2081, 4, 15), bs(2080, 1, 1), (true, 1, 3, 14, -473)),
            (bs(2081, 4, 32), bs(2081, 5, 31), (false, 0, 1, 0, 31)),
            (bs(2081, 4, 32), bs(2081, 6, 1), (false, 0, 1, 1, 32)),
            (bs(2081, 1, 31), bs(2081, 2, 30), (false, 0, 0, 30, 30)),
            (bs(2081, 3, 15), bs(2081, 3, 15), (false, 0, 0, 0, 0)),
            (bs(2080, 1, 1), bs(2081, 1, 1), (false, 1, 0, 0, 365)),
            (bs(2081, 4, 32), bs(2081, 5, 1), (false, 0, 0, 1, 1)),
        ];
        for (a, b, (negative, years, months, days, total_days)) in cases {
            let expected = DateDiff {
                negative,
                years,
                months,
                days,
                total_days,
            };
            assert_eq!(a.diff(b), expected, "{a:?} -> {b:?}");
        }
    }

    #[test]
    fn diff_reversed_has_same_magnitudes() {
        let (a, b) = (bs(2081, 4, 32), bs(2083, 2, 7));
        let (f, r) = (a.diff(b), b.diff(a));
        assert!(!f.negative && r.negative);
        assert_eq!((f.years, f.months, f.days), (r.years, r.months, r.days));
        assert_eq!(f.total_days, -r.total_days);
        let whole = NepaliDate::MIN.diff(NepaliDate::MAX);
        assert_eq!((whole.years, whole.months), (298, 11));
        assert_eq!(whole.total_days, 109_211);
    }
}
