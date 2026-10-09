//! Nepal's fiscal year, 1 Shrawan to the last day of Ashad, and its quarters (ALGORITHM §8b,
//! D-08).

use core::fmt;

use crate::range::NepaliDateRange;
use crate::{Error, Lang, Month, NepaliDate};

/// A fiscal year, named by the BS year it starts in: FY 2082/83 is `FiscalYear::new(2082)` and
/// runs from 2082-04-01 to the last day of 2083-03.
///
/// Any `u16` can be held. The dates are checked when asked for, so `FiscalYear::new(1900)` (the
/// fiscal year of 1901-01-01) exists but its [`start`](Self::start) is out of range.
///
/// ```
/// use nepdate::{FiscalYear, Lang, NepaliDate};
///
/// let fy = FiscalYear::containing(NepaliDate::new(2083, 2, 10)?);
/// assert_eq!(fy, FiscalYear::new(2082));
/// assert_eq!(fy.label(Lang::En).to_string(), "2082/83");
/// assert_eq!(fy.end()?.to_string(), "2083/03/32");
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FiscalYear(u16);

impl FiscalYear {
    /// The fiscal year that starts in BS year `start_year`.
    ///
    /// ```
    /// assert_eq!(nepdate::FiscalYear::new(2082).start_year(), 2082);
    /// ```
    #[must_use]
    pub const fn new(start_year: u16) -> Self {
        Self(start_year)
    }

    /// The BS year this fiscal year starts in.
    ///
    /// ```
    /// assert_eq!(nepdate::FiscalYear::new(2082).start_year(), 2082);
    /// ```
    #[must_use]
    pub const fn start_year(self) -> u16 {
        self.0
    }

    /// The fiscal year `date` is in: its own year from Shrawan on, the year before in Baishakh to
    /// Ashad.
    ///
    /// ```
    /// use nepdate::{FiscalYear, NepaliDate};
    ///
    /// assert_eq!(FiscalYear::containing(NepaliDate::new(2081, 3, 31)?), FiscalYear::new(2080));
    /// assert_eq!(FiscalYear::containing(NepaliDate::new(2081, 4, 1)?), FiscalYear::new(2081));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn containing(date: NepaliDate) -> Self {
        if date.month() >= Month::Shrawan {
            Self(date.year())
        } else {
            // date.year() is at least 1901, so this can't underflow.
            Self(date.year() - 1)
        }
    }

    /// The first day, 1 Shrawan of the start year.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] when that day is outside the supported range (start year 1900 or
    /// after 2199).
    ///
    /// ```
    /// use nepdate::{Error, FiscalYear};
    ///
    /// assert_eq!(FiscalYear::new(2082).start()?.to_string(), "2082/04/01");
    /// assert_eq!(FiscalYear::new(1900).start(), Err(Error::OutOfRange));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the date may be out of range"]
    pub fn start(self) -> Result<NepaliDate, Error> {
        NepaliDate::new(self.0, 4, 1)
    }

    /// The last day, the last day of Ashad in the following year.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] when that day is outside the supported range (start year before 1900
    /// or 2199 and later).
    ///
    /// ```
    /// use nepdate::{Error, FiscalYear};
    ///
    /// assert_eq!(FiscalYear::new(1900).end()?.to_string(), "1901/03/32");
    /// assert_eq!(FiscalYear::new(2199).end(), Err(Error::OutOfRange));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the date may be out of range"]
    pub fn end(self) -> Result<NepaliDate, Error> {
        let year = self.0.checked_add(1).ok_or(Error::OutOfRange)?;
        Ok(NepaliDate::new(year, 3, 1)?.last_day_of_month())
    }

    /// Every day of the fiscal year.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] when the start or the end is out of range.
    ///
    /// ```
    /// let fy = nepdate::FiscalYear::new(2080).range()?;
    /// assert_eq!(fy.start().to_string(), "2080/04/01");
    /// assert_eq!(fy.end().to_string(), "2081/03/31");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the range may be out of range"]
    pub fn range(self) -> Result<NepaliDateRange, Error> {
        Ok(NepaliDateRange::new_unchecked(self.start()?, self.end()?))
    }

    /// Every day of one quarter. Q1 to Q3 fall in the start year, Q4 (Baishakh to Ashad) in the
    /// next one.
    ///
    /// # Errors
    ///
    /// [`Error::OutOfRange`] when the quarter's first or last day is out of range.
    ///
    /// ```
    /// use nepdate::{FiscalYear, Quarter};
    ///
    /// let q4 = FiscalYear::new(2080).quarter_range(Quarter::Q4)?;
    /// assert_eq!(q4.start().to_string(), "2081/01/01");
    /// assert_eq!(q4.end().to_string(), "2081/03/31");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "the range may be out of range"]
    pub fn quarter_range(self, q: Quarter) -> Result<NepaliDateRange, Error> {
        let [first, _, last] = q.months();
        let year = match q {
            Quarter::Q1 | Quarter::Q2 | Quarter::Q3 => Some(self.0),
            Quarter::Q4 => self.0.checked_add(1),
        };
        let year = year.ok_or(Error::OutOfRange)?;
        let start = NepaliDate::new(year, first.number(), 1)?;
        let end = NepaliDate::new(year, last.number(), 1)?.last_day_of_month();
        Ok(NepaliDateRange::new_unchecked(start, end))
    }

    /// The label `2082/83`: the start year, a slash and the last two digits of the next year,
    /// in the digits of `lang`.
    ///
    /// ```
    /// use nepdate::{FiscalYear, Lang};
    ///
    /// assert_eq!(FiscalYear::new(2082).label(Lang::Ne).to_string(), "२०८२/८३");
    /// assert_eq!(FiscalYear::new(2099).label(Lang::En).to_string(), "2099/00");
    /// ```
    #[must_use]
    pub const fn label(self, lang: Lang) -> FiscalLabel {
        FiscalLabel { year: self, lang }
    }
}

/// A fiscal year's label in a [`Lang`], from [`FiscalYear::label`].
///
/// ```
/// let mut out = String::new();
/// nepdate::FiscalYear::new(2082).label(nepdate::Lang::En).write_to(&mut out)?;
/// assert_eq!(out, "2082/83");
/// # Ok::<(), core::fmt::Error>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FiscalLabel {
    year: FiscalYear,
    lang: Lang,
}

impl FiscalLabel {
    /// Writes the label to `out` without allocating.
    ///
    /// # Errors
    ///
    /// Whatever `out.write_str` returns.
    ///
    /// ```
    /// let mut out = String::new();
    /// nepdate::FiscalYear::new(2082).label(nepdate::Lang::Ne).write_to(&mut out)?;
    /// assert_eq!(out, "२०८२/८३");
    /// # Ok::<(), core::fmt::Error>(())
    /// ```
    pub fn write_to<W: fmt::Write + ?Sized>(&self, out: &mut W) -> fmt::Result {
        let start = u32::from(self.year.start_year());
        self.lang.digits(start, 1).write_to(out)?;
        out.write_str("/")?;
        self.lang.digits((start + 1) % 100, 2).write_to(out)
    }
}

impl fmt::Display for FiscalLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(f)
    }
}

/// A quarter of the fiscal year: Q1 is Shrawan to Ashoj, Q4 is Baishakh to Ashad (D-08).
///
/// ```
/// use nepdate::{Month, Quarter};
///
/// assert_eq!(Quarter::Q1.months(), [Month::Shrawan, Month::Bhadra, Month::Ashoj]);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Quarter {
    /// Shrawan, Bhadra, Ashoj.
    Q1,
    /// Kartik, Mangsir, Poush.
    Q2,
    /// Magh, Falgun, Chaitra.
    Q3,
    /// Baishakh, Jestha, Ashad (of the next BS year).
    Q4,
}

impl Quarter {
    /// The three months of the quarter, in order.
    ///
    /// ```
    /// use nepdate::{Month, Quarter};
    ///
    /// assert_eq!(Quarter::Q4.months(), [Month::Baishakh, Month::Jestha, Month::Ashad]);
    /// ```
    #[must_use]
    pub const fn months(self) -> [Month; 3] {
        match self {
            Quarter::Q1 => [Month::Shrawan, Month::Bhadra, Month::Ashoj],
            Quarter::Q2 => [Month::Kartik, Month::Mangsir, Month::Poush],
            Quarter::Q3 => [Month::Magh, Month::Falgun, Month::Chaitra],
            Quarter::Q4 => [Month::Baishakh, Month::Jestha, Month::Ashad],
        }
    }
}

impl NepaliDate {
    /// The fiscal year this date is in. For 1901-01-01 to 1901-03-32 that is
    /// `FiscalYear::new(1900)`, whose start is out of range.
    ///
    /// ```
    /// use nepdate::{FiscalYear, NepaliDate};
    ///
    /// assert_eq!(NepaliDate::new(2082, 4, 1)?.fiscal_year(), FiscalYear::new(2082));
    /// assert_eq!(NepaliDate::MIN.fiscal_year(), FiscalYear::new(1900));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn fiscal_year(self) -> FiscalYear {
        FiscalYear::containing(self)
    }

    /// The fiscal quarter this date is in.
    ///
    /// ```
    /// use nepdate::{NepaliDate, Quarter};
    ///
    /// assert_eq!(NepaliDate::new(2082, 3, 15)?.quarter(), Quarter::Q4);
    /// assert_eq!(NepaliDate::new(2082, 7, 1)?.quarter(), Quarter::Q2);
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub fn quarter(self) -> Quarter {
        match self.month() {
            Month::Shrawan | Month::Bhadra | Month::Ashoj => Quarter::Q1,
            Month::Kartik | Month::Mangsir | Month::Poush => Quarter::Q2,
            Month::Magh | Month::Falgun | Month::Chaitra => Quarter::Q3,
            Month::Baishakh | Month::Jestha | Month::Ashad => Quarter::Q4,
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::string::ToString;

    fn bs(y: u16, m: u8, d: u8) -> NepaliDate {
        NepaliDate::new(y, m, d).unwrap_or(NepaliDate::MIN)
    }

    #[test]
    fn labels() {
        assert_eq!(FiscalYear::new(2082).label(Lang::Ne).to_string(), "२०८२/८३");
        assert_eq!(FiscalYear::new(2082).label(Lang::En).to_string(), "2082/83");
        assert_eq!(FiscalYear::new(2099).label(Lang::En).to_string(), "2099/00");
        assert_eq!(FiscalYear::new(2100).label(Lang::En).to_string(), "2100/01");
        assert_eq!(FiscalYear::new(1900).label(Lang::Ne).to_string(), "१९००/०१");
        assert_eq!(
            FiscalYear::new(u16::MAX).label(Lang::En).to_string(),
            "65535/36"
        );
    }

    #[test]
    fn containing_and_quarter() {
        assert_eq!(
            FiscalYear::containing(bs(2081, 3, 31)),
            FiscalYear::new(2080)
        );
        assert_eq!(
            FiscalYear::containing(bs(2081, 4, 1)),
            FiscalYear::new(2081)
        );
        for m in 1..=12u8 {
            let d = bs(2081, m, 1);
            let q = d.quarter();
            assert!(q.months().contains(&d.month()), "{d:?}");
            let fy = d.fiscal_year().quarter_range(q);
            assert!(fy.is_ok_and(|r| r.contains(d)), "{d:?}");
        }
    }

    #[test]
    fn range_ends() {
        assert_eq!(FiscalYear::new(1900).start(), Err(Error::OutOfRange));
        assert_eq!(FiscalYear::new(1900).end(), Ok(bs(1901, 3, 32)));
        assert_eq!(FiscalYear::new(1900).range(), Err(Error::OutOfRange));
        assert_eq!(FiscalYear::new(2199).end(), Err(Error::OutOfRange));
        assert_eq!(FiscalYear::new(2199).start(), Ok(bs(2199, 4, 1)));
        assert_eq!(FiscalYear::new(u16::MAX).end(), Err(Error::OutOfRange));
        assert_eq!(
            FiscalYear::new(u16::MAX).quarter_range(Quarter::Q4),
            Err(Error::OutOfRange)
        );
        assert_eq!(
            FiscalYear::new(1900)
                .quarter_range(Quarter::Q4)
                .map(NepaliDateRange::start),
            Ok(NepaliDate::MIN)
        );
        assert_eq!(
            FiscalYear::new(2199)
                .quarter_range(Quarter::Q3)
                .map(NepaliDateRange::end),
            Ok(NepaliDate::MAX)
        );
    }

    #[test]
    fn quarters_split_the_year() {
        let fy = FiscalYear::new(2080);
        let total: u32 = [Quarter::Q1, Quarter::Q2, Quarter::Q3, Quarter::Q4]
            .into_iter()
            .map(|q| fy.quarter_range(q).map_or(0, NepaliDateRange::len))
            .sum();
        assert_eq!(fy.range().map(NepaliDateRange::len), Ok(total));
    }
}
