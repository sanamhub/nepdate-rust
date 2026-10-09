//! Language, month and weekday names (FORMATTING §1). Names are exactly the C# strings; the short
//! forms are new (D-04).

use crate::Error;

/// Output language for names and digits.
///
/// ```
/// use nepdate::{Lang, Month};
///
/// assert_eq!(Month::Shrawan.name(Lang::default()), "Shrawan");
/// assert_eq!(Month::Shrawan.name(Lang::Ne), "साउन");
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Lang {
    /// English names, ASCII digits.
    #[default]
    En,
    /// Nepali names in Devanagari, Devanagari digits.
    Ne,
}

/// A BS month. The discriminant is the month number, 1 to 12.
///
/// ```
/// use nepdate::Month;
///
/// assert_eq!(Month::Ashad.number(), 3);
/// assert_eq!(Month::try_from(3), Ok(Month::Ashad));
/// ```
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Month {
    /// Month 1.
    Baishakh = 1,
    /// Month 2.
    Jestha,
    /// Month 3.
    Ashad,
    /// Month 4.
    Shrawan,
    /// Month 5.
    Bhadra,
    /// Month 6.
    Ashoj,
    /// Month 7.
    Kartik,
    /// Month 8.
    Mangsir,
    /// Month 9.
    Poush,
    /// Month 10.
    Magh,
    /// Month 11.
    Falgun,
    /// Month 12.
    Chaitra,
}

const MONTH_EN: [&str; 12] = [
    "Baishakh", "Jestha", "Ashad", "Shrawan", "Bhadra", "Ashoj", "Kartik", "Mangsir", "Poush",
    "Magh", "Falgun", "Chaitra",
];
const MONTH_EN_SHORT: [&str; 12] = [
    "Bai", "Jes", "Asa", "Shr", "Bha", "Aso", "Kar", "Man", "Pou", "Mag", "Fal", "Cha",
];
const MONTH_NE: [&str; 12] = [
    "बैशाख",
    "जेठ",
    "असार",
    "साउन",
    "भदौ",
    "असोज",
    "कार्तिक",
    "मंसिर",
    "पुष",
    "माघ",
    "फागुन",
    "चैत",
];

impl Month {
    /// All twelve months, Baishakh first.
    ///
    /// ```
    /// assert_eq!(nepdate::Month::ALL.len(), 12);
    /// ```
    pub const ALL: [Month; 12] = [
        Month::Baishakh,
        Month::Jestha,
        Month::Ashad,
        Month::Shrawan,
        Month::Bhadra,
        Month::Ashoj,
        Month::Kartik,
        Month::Mangsir,
        Month::Poush,
        Month::Magh,
        Month::Falgun,
        Month::Chaitra,
    ];

    /// The month number, 1 to 12.
    ///
    /// ```
    /// assert_eq!(nepdate::Month::Chaitra.number(), 12);
    /// ```
    #[must_use]
    pub const fn number(self) -> u8 {
        self as u8
    }

    /// The month for a number 1 to 12, or `None`.
    pub(crate) const fn from_number(m: u8) -> Option<Month> {
        match m {
            1 => Some(Month::Baishakh),
            2 => Some(Month::Jestha),
            3 => Some(Month::Ashad),
            4 => Some(Month::Shrawan),
            5 => Some(Month::Bhadra),
            6 => Some(Month::Ashoj),
            7 => Some(Month::Kartik),
            8 => Some(Month::Mangsir),
            9 => Some(Month::Poush),
            10 => Some(Month::Magh),
            11 => Some(Month::Falgun),
            12 => Some(Month::Chaitra),
            _ => None,
        }
    }

    /// Index 0 to 11 into the name tables.
    const fn index(self) -> usize {
        self as usize - 1
    }

    /// The full name (FORMATTING §1).
    ///
    /// ```
    /// use nepdate::{Lang, Month};
    ///
    /// assert_eq!(Month::Kartik.name(Lang::En), "Kartik");
    /// assert_eq!(Month::Kartik.name(Lang::Ne), "कार्तिक");
    /// ```
    #[must_use]
    #[allow(clippy::indexing_slicing)] // index() is 0..=11 for every variant
    pub fn name(self, lang: Lang) -> &'static str {
        match lang {
            Lang::En => MONTH_EN[self.index()],
            Lang::Ne => MONTH_NE[self.index()],
        }
    }

    /// The short name: three letters in English (`Asa`, `Aso`, D-04), the full name in Nepali.
    ///
    /// ```
    /// use nepdate::{Lang, Month};
    ///
    /// assert_eq!(Month::Ashad.short_name(Lang::En), "Asa");
    /// assert_eq!(Month::Ashad.short_name(Lang::Ne), "असार");
    /// ```
    #[must_use]
    #[allow(clippy::indexing_slicing)] // index() is 0..=11 for every variant
    pub fn short_name(self, lang: Lang) -> &'static str {
        match lang {
            Lang::En => MONTH_EN_SHORT[self.index()],
            Lang::Ne => MONTH_NE[self.index()],
        }
    }

    /// The next month; Chaitra wraps to Baishakh.
    ///
    /// ```
    /// use nepdate::Month;
    ///
    /// assert_eq!(Month::Chaitra.next(), Month::Baishakh);
    /// ```
    #[must_use]
    #[allow(clippy::indexing_slicing)] // number() % 12 is 0..=11
    pub fn next(self) -> Month {
        Month::ALL[usize::from(self.number() % 12)]
    }

    /// The previous month; Baishakh wraps to Chaitra.
    ///
    /// ```
    /// use nepdate::Month;
    ///
    /// assert_eq!(Month::Baishakh.prev(), Month::Chaitra);
    /// ```
    #[must_use]
    #[allow(clippy::indexing_slicing)] // (number() + 10) % 12 is 0..=11
    pub fn prev(self) -> Month {
        Month::ALL[usize::from((self.number() + 10) % 12)]
    }
}

impl TryFrom<u8> for Month {
    type Error = Error;

    fn try_from(m: u8) -> Result<Self, Error> {
        Month::from_number(m).ok_or(Error::InvalidMonth(m))
    }
}

/// A day of the week. The discriminant counts from Sunday = 0, as in Nepal's week.
///
/// ```
/// use nepdate::{Lang, Weekday};
///
/// assert_eq!(Weekday::Saturday.index_from_sunday(), 6);
/// assert_eq!(Weekday::Saturday.name(Lang::Ne), "शनिवार");
/// ```
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Weekday {
    /// आइतवार.
    Sunday = 0,
    /// सोमवार.
    Monday,
    /// मङ्गलवार.
    Tuesday,
    /// बुधवार.
    Wednesday,
    /// बिहिवार.
    Thursday,
    /// शुक्रवार.
    Friday,
    /// शनिवार.
    Saturday,
}

const WEEKDAY_EN: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const WEEKDAY_EN_SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAY_NE: [&str; 7] = [
    "आइतवार",
    "सोमवार",
    "मङ्गलवार",
    "बुधवार",
    "बिहिवार",
    "शुक्रवार",
    "शनिवार",
];
const WEEKDAY_NE_SHORT: [&str; 7] = ["आइत", "सोम", "मङ्गल", "बुध", "बिहि", "शुक्र", "शनि"];

impl Weekday {
    /// All seven days, Sunday first.
    ///
    /// ```
    /// assert_eq!(nepdate::Weekday::ALL[0], nepdate::Weekday::Sunday);
    /// ```
    pub const ALL: [Weekday; 7] = [
        Weekday::Sunday,
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
    ];

    /// Days since Sunday, 0 to 6.
    ///
    /// ```
    /// assert_eq!(nepdate::Weekday::Monday.index_from_sunday(), 1);
    /// ```
    #[must_use]
    pub const fn index_from_sunday(self) -> u8 {
        self as u8
    }

    /// The full name (FORMATTING §1).
    ///
    /// ```
    /// use nepdate::{Lang, Weekday};
    ///
    /// assert_eq!(Weekday::Tuesday.name(Lang::En), "Tuesday");
    /// ```
    #[must_use]
    #[allow(clippy::indexing_slicing)] // the discriminant is 0..=6
    pub fn name(self, lang: Lang) -> &'static str {
        let i = usize::from(self.index_from_sunday());
        match lang {
            Lang::En => WEEKDAY_EN[i],
            Lang::Ne => WEEKDAY_NE[i],
        }
    }

    /// The short name, for a month grid header.
    ///
    /// ```
    /// use nepdate::{Lang, Weekday};
    ///
    /// assert_eq!(Weekday::Tuesday.short_name(Lang::En), "Tue");
    /// assert_eq!(Weekday::Tuesday.short_name(Lang::Ne), "मङ्गल");
    /// ```
    #[must_use]
    #[allow(clippy::indexing_slicing)] // the discriminant is 0..=6
    pub fn short_name(self, lang: Lang) -> &'static str {
        let i = usize::from(self.index_from_sunday());
        match lang {
            Lang::En => WEEKDAY_EN_SHORT[i],
            Lang::Ne => WEEKDAY_NE_SHORT[i],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nepali_ok(s: &str) {
        assert!(!s.is_empty());
        assert!(!s.bytes().any(|b| b.is_ascii()), "{s} contains ASCII");
    }

    #[test]
    fn nepali_names_have_no_ascii() {
        for m in Month::ALL {
            nepali_ok(m.name(Lang::Ne));
            nepali_ok(m.short_name(Lang::Ne));
        }
        for w in Weekday::ALL {
            nepali_ok(w.name(Lang::Ne));
            nepali_ok(w.short_name(Lang::Ne));
        }
    }

    #[test]
    fn month_numbers_round_trip() {
        for (i, m) in Month::ALL.into_iter().enumerate() {
            let n = u8::try_from(i + 1).unwrap_or(0);
            assert_eq!(m.number(), n);
            assert_eq!(Month::try_from(n), Ok(m));
            assert_eq!(m.next().prev(), m);
        }
        assert_eq!(Month::try_from(0), Err(Error::InvalidMonth(0)));
        assert_eq!(Month::try_from(13), Err(Error::InvalidMonth(13)));
    }

    #[test]
    fn english_names() {
        assert_eq!(Month::Ashoj.short_name(Lang::En), "Aso");
        assert_eq!(Month::Baishakh.name(Lang::En), "Baishakh");
        assert_eq!(Weekday::Sunday.name(Lang::En), "Sunday");
    }
}
