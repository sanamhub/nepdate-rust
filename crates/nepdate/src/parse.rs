//! Strict (`FromStr`) and lenient (`NepaliDate::parse_lenient`) parsing (PARSING §1 to §3).
//!
//! Both read `s.as_bytes()` in one pass and keep their state in fixed-size values, so neither
//! allocates. A Devanagari digit is the 3-byte UTF-8 sequence `E0 A5 A6` to `E0 A5 AF` and the
//! danda `।` is `E0 A5 A4`; every other non-ASCII byte belongs to a word.

use core::str::FromStr;

use crate::{Error, NepaliDate, ParseErrorKind};

/// The digit at `i` and its length in bytes: an ASCII digit, or a Devanagari digit (D-05).
fn digit_at(b: &[u8], i: usize) -> Option<(u32, usize)> {
    match *b.get(i)? {
        c @ b'0'..=b'9' => Some((u32::from(c - b'0'), 1)),
        0xE0 if b.get(i + 1) == Some(&0xA5) => match b.get(i + 2) {
            Some(&c @ 0xA6..=0xAF) => Some((u32::from(c - 0xA6), 3)),
            _ => None,
        },
        _ => None,
    }
}

/// The length of the separator at `i` (PARSING §1), with or without `.`.
fn separator_at(b: &[u8], i: usize, dot: bool) -> Option<usize> {
    match *b.get(i)? {
        b'-' | b'/' | b'_' | b'\\' | b' ' | b'|' => Some(1),
        b'.' if dot => Some(1),
        // The danda `।`.
        0xE0 if b.get(i + 1) == Some(&0xA5) && b.get(i + 2) == Some(&0xA4) => Some(3),
        _ => None,
    }
}

/// Validates year, month and day with `NepaliDate::new` (ALGORITHM §3). Each number has at most
/// 4 digits; a month or day above 255 is passed on as 255, which fails the same check with the
/// same error kind.
fn validate(year: u32, month: u32, day: u32) -> Result<NepaliDate, Error> {
    let year = u16::try_from(year).map_err(|_| Error::OutOfRange)?;
    let month = u8::try_from(month).unwrap_or(u8::MAX);
    let day = u8::try_from(day).unwrap_or(u8::MAX);
    NepaliDate::new(year, month, day)
}

fn parse_error(kind: ParseErrorKind) -> Error {
    Error::Parse(kind)
}

/// PARSING §1.
fn parse_strict(s: &str) -> Result<NepaliDate, Error> {
    let b = s.as_bytes();
    let mut groups = [0u32; 3];
    // Groups started so far, and digits in the current group (0 after a separator).
    let mut count = 0;
    let mut digits = 0;
    let mut i = 0;
    while i < b.len() {
        if let Some((value, len)) = digit_at(b, i) {
            if digits == 0 {
                if count == 3 {
                    return Err(parse_error(ParseErrorKind::WrongGroupCount));
                }
                count += 1;
            }
            digits += 1;
            if digits > 4 {
                return Err(parse_error(ParseErrorKind::NumberTooLong));
            }
            if let Some(g) = groups.get_mut(count - 1) {
                *g = *g * 10 + value;
            }
            i += len;
        } else if let Some(len) = separator_at(b, i, true) {
            digits = 0;
            i += len;
        } else {
            return Err(parse_error(ParseErrorKind::InvalidCharacter));
        }
    }
    if count < 3 {
        return Err(parse_error(ParseErrorKind::WrongGroupCount));
    }
    let [year, month, day] = groups;
    validate(year, month, day)
}

impl FromStr for NepaliDate {
    type Err = Error;

    /// Strict parsing (PARSING §1): year, month and day in that order, in ASCII or Devanagari
    /// digits (D-05), separated by any of `- / . _ \ space । |`.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] with [`ParseErrorKind::InvalidCharacter`],
    /// [`ParseErrorKind::WrongGroupCount`] or [`ParseErrorKind::NumberTooLong`], or the errors of
    /// [`NepaliDate::new`] for a date that doesn't exist. A month or day of 256 to 9999 doesn't
    /// fit in a `u8`, so it is checked as 255: [`Error::InvalidMonth`] and the `day` of
    /// [`Error::InvalidDay`] then hold 255, not the number in the text.
    ///
    /// ```
    /// use nepdate::{Error, NepaliDate, ParseErrorKind};
    ///
    /// let d: NepaliDate = "२०८०/०५/१५".parse()?;
    /// assert_eq!(d, NepaliDate::new(2080, 5, 15)?);
    /// assert_eq!("2080/05".parse::<NepaliDate>(), Err(Error::Parse(ParseErrorKind::WrongGroupCount)));
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    fn from_str(s: &str) -> Result<Self, Error> {
        parse_strict(s)
    }
}

/// Month names accepted by the lenient parser, in the order of PARSING §3 (duplicates there
/// appear once). Latin names compare without regard
/// to ASCII case; Devanagari compares exactly.
static MONTH_ALIASES: [(&str, u8); 80] = [
    ("baishakh", 1),
    ("baisakh", 1),
    ("baishak", 1),
    ("vaishakh", 1),
    ("vaisakh", 1),
    ("bai", 1),
    ("बैशाख", 1),
    ("वैशाख", 1),
    ("बैसाख", 1),
    ("jestha", 2),
    ("jeth", 2),
    ("jyestha", 2),
    ("jes", 2),
    ("जेठ", 2),
    ("जेष्ठ", 2),
    ("ज्येष्ठ", 2),
    ("ashad", 3),
    ("ashadh", 3),
    ("asar", 3),
    ("asadh", 3),
    ("asa", 3),
    ("असार", 3),
    ("आषाढ", 3),
    ("असाढ", 3),
    ("shrawan", 4),
    ("shravan", 4),
    ("srawan", 4),
    ("sawan", 4),
    ("saun", 4),
    ("shr", 4),
    ("साउन", 4),
    ("श्रावण", 4),
    ("सावन", 4),
    ("bhadra", 5),
    ("bhadau", 5),
    ("bha", 5),
    ("भदौ", 5),
    ("भाद्र", 5),
    ("भाद्रपद", 5),
    ("ashoj", 6),
    ("asoj", 6),
    ("ashwin", 6),
    ("aso", 6),
    ("असोज", 6),
    ("आश्विन", 6),
    ("असौज", 6),
    ("kartik", 7),
    ("kattik", 7),
    ("kar", 7),
    ("कार्तिक", 7),
    ("कात्तिक", 7),
    ("mangsir", 8),
    ("mansir", 8),
    ("margashirsha", 8),
    ("man", 8),
    ("मंसिर", 8),
    ("मङ्सिर", 8),
    ("मार्गशीर्ष", 8),
    ("poush", 9),
    ("paush", 9),
    ("push", 9),
    ("pus", 9),
    ("pou", 9),
    ("पुष", 9),
    ("पौष", 9),
    ("पुस", 9),
    ("magh", 10),
    ("mag", 10),
    ("माघ", 10),
    ("falgun", 11),
    ("phalgun", 11),
    ("fagun", 11),
    ("fal", 11),
    ("फागुन", 11),
    ("फाल्गुन", 11),
    ("chaitra", 12),
    ("chait", 12),
    ("cha", 12),
    ("चैत", 12),
    ("चैत्र", 12),
];

/// Era markers and the words `गते` and `मिति`, dropped by the lenient parser (PARSING §2).
static IGNORED_TOKENS: [&str; 12] = [
    "bs",
    "b.s.",
    "b.s",
    "vs",
    "v.s.",
    "v.s",
    "बि.सं.",
    "वि.सं.",
    "बि.सं",
    "वि.सं",
    "गते",
    "मिति",
];

/// What the lenient tokeniser has seen so far. Fixed size: three numbers are kept and any more
/// are only counted.
#[derive(Default)]
struct Tokens {
    numbers: [(u32, u8); 3],
    number_count: usize,
    month: Option<u8>,
    month_count: usize,
    unknown_word: bool,
    too_long: bool,
}

impl Tokens {
    fn push_number(&mut self, b: &[u8], start: usize, end: usize) {
        let (mut value, mut digits, mut i) = (0u32, 0u8, start);
        while i < end {
            let Some((d, len)) = digit_at(b, i) else {
                break;
            };
            digits = digits.saturating_add(1);
            // Only up to 4 digits are kept; a longer token is an error anyway.
            if digits <= 4 {
                value = value * 10 + d;
            }
            i += len;
        }
        if digits == 0 {
            return;
        }
        if digits > 4 {
            self.too_long = true;
        }
        if let Some(slot) = self.numbers.get_mut(self.number_count) {
            *slot = (value, digits);
        }
        self.number_count += 1;
    }

    fn push_word(&mut self, word: &str) {
        if word.is_empty() || IGNORED_TOKENS.iter().any(|t| t.eq_ignore_ascii_case(word)) {
            return;
        }
        match MONTH_ALIASES
            .iter()
            .find(|(alias, _)| alias.eq_ignore_ascii_case(word))
        {
            Some(&(_, m)) => {
                self.month = Some(m);
                self.month_count += 1;
            }
            None => self.unknown_word = true,
        }
    }
}

/// Splits `s` into number and word tokens (PARSING §2 step 2).
fn tokenize(s: &str) -> Tokens {
    let b = s.as_bytes();
    let mut tokens = Tokens::default();
    let is_sep = |i: usize| -> Option<usize> {
        if b.get(i) == Some(&b',') {
            Some(1)
        } else {
            separator_at(b, i, false)
        }
    };
    let mut i = 0;
    while i < b.len() {
        if let Some(len) = is_sep(i) {
            i += len;
            continue;
        }
        // A chunk runs to the next separator. `.` is not a separator here, so `B.S.` stays whole.
        let start = i;
        let mut only_digits_and_dots = true;
        while i < b.len() && is_sep(i).is_none() {
            if let Some((_, len)) = digit_at(b, i) {
                i += len;
            } else {
                if b.get(i) != Some(&b'.') {
                    only_digits_and_dots = false;
                }
                i += 1;
            }
        }
        split_chunk(s, start, i, only_digits_and_dots, &mut tokens);
    }
    tokens
}

/// One chunk between separators (PARSING §2 step 2). Made only of digits and dots: numbers split
/// on the dots. Anything else is one word, dots and digits included, so `Shr.15` and `2080.B.S.`
/// are words that match no month and no era.
fn split_chunk(s: &str, start: usize, end: usize, only_digits_and_dots: bool, t: &mut Tokens) {
    if !only_digits_and_dots {
        // `start` and `end` sit next to a separator or at the ends of `s`, so they are char
        // boundaries; `get` keeps a mistake from panicking.
        if let Some(word) = s.get(start..end) {
            t.push_word(word);
        }
        return;
    }
    let b = s.as_bytes();
    let mut i = start;
    while i < end {
        if b.get(i) == Some(&b'.') {
            i += 1;
            continue;
        }
        let run = i;
        while i < end && b.get(i) != Some(&b'.') {
            i += digit_at(b, i).map_or(1, |(_, len)| len);
        }
        t.push_number(b, run, i);
    }
}

/// PARSING §2 steps 2 to 6, after strict parsing failed.
fn parse_lenient_tokens(s: &str) -> Result<NepaliDate, Error> {
    let t = tokenize(s);
    if t.too_long {
        return Err(parse_error(ParseErrorKind::NumberTooLong));
    }
    if t.unknown_word || t.month_count > 1 {
        return Err(parse_error(ParseErrorKind::Unrecognized));
    }
    let [(n0, w0), (n1, w1), (n2, w2)] = t.numbers;
    match (t.month, t.number_count) {
        (Some(month), 2) => {
            let (year, day, day_width) = match (w0 == 4, w1 == 4) {
                (true, false) => (n0, n1, w1),
                (false, true) => (n1, n0, w0),
                _ => return Err(parse_error(ParseErrorKind::Ambiguous)),
            };
            if day_width > 2 {
                return Err(parse_error(ParseErrorKind::Ambiguous));
            }
            validate(year, u32::from(month), day)
        }
        (None, 3) => {
            if w0 == 4 {
                validate(n0, n1, n2)
            } else if w2 == 4 {
                validate(n2, n1, n0)
            } else {
                Err(parse_error(ParseErrorKind::Ambiguous))
            }
        }
        _ => Err(parse_error(ParseErrorKind::Unrecognized)),
    }
}

impl NepaliDate {
    /// Lenient parsing for text typed by people (PARSING §2): everything strict parsing accepts,
    /// plus day-month-year order, month names in English or Nepali, `,` as a separator, and the
    /// era markers `BS`, `VS`, `बि.सं.` and the words `गते`, `मिति`. It never guesses: a 2-digit
    /// year or an order it can't tell gives [`ParseErrorKind::Ambiguous`] (D-06).
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] with [`ParseErrorKind::NumberTooLong`], [`ParseErrorKind::Ambiguous`] or
    /// [`ParseErrorKind::Unrecognized`], or the errors of [`NepaliDate::new`] for a date that
    /// doesn't exist. As in strict parsing, a month or day above 255 is reported as 255.
    ///
    /// ```
    /// use nepdate::{Error, NepaliDate, ParseErrorKind};
    ///
    /// let d = NepaliDate::new(2081, 4, 15)?;
    /// assert_eq!(NepaliDate::parse_lenient("15 Shrawan 2081")?, d);
    /// assert_eq!(NepaliDate::parse_lenient("१५ साउन २०८१")?, d);
    /// assert_eq!(NepaliDate::parse_lenient("15/04/2081 B.S.")?, d);
    /// assert_eq!(
    ///     NepaliDate::parse_lenient("15 saun 81"),
    ///     Err(Error::Parse(ParseErrorKind::Ambiguous))
    /// );
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use = "parsing may fail"]
    pub fn parse_lenient(s: &str) -> Result<Self, Error> {
        match parse_strict(s) {
            Ok(d) => Ok(d),
            Err(_) => parse_lenient_tokens(s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(k: ParseErrorKind) -> Result<NepaliDate, Error> {
        Err(Error::Parse(k))
    }

    fn bs(y: u16, m: u8, d: u8) -> Result<NepaliDate, Error> {
        NepaliDate::new(y, m, d)
    }

    #[test]
    fn strict_examples() {
        for s in [
            "2080/05/15",
            "2080-5-15",
            "  2080 / 05 / 15  ",
            "/2080/05/15/",
            "2080।05।15",
            "२०८०/०५/१५",
            "2\u{0966}\u{096E}0/0\u{096B}/\u{0967}5",
        ] {
            assert_eq!(s.parse::<NepaliDate>(), bs(2080, 5, 15), "{s:?}");
        }
        assert_eq!(
            "".parse::<NepaliDate>(),
            kind(ParseErrorKind::WrongGroupCount)
        );
        assert_eq!(
            "2080/05/15/1".parse::<NepaliDate>(),
            kind(ParseErrorKind::WrongGroupCount)
        );
        assert_eq!(
            "12345/1/1".parse::<NepaliDate>(),
            kind(ParseErrorKind::NumberTooLong)
        );
        assert_eq!(
            "2080/05/1x".parse::<NepaliDate>(),
            kind(ParseErrorKind::InvalidCharacter)
        );
        assert_eq!(
            "2080,05,15".parse::<NepaliDate>(),
            kind(ParseErrorKind::InvalidCharacter)
        );
        // A Devanagari letter starts with the same first byte as a digit.
        assert_eq!(
            "2080/05/१क".parse::<NepaliDate>(),
            kind(ParseErrorKind::InvalidCharacter)
        );
        assert_eq!(
            "9999/9999/9999".parse::<NepaliDate>(),
            Err(Error::OutOfRange)
        );
        assert_eq!(
            "2080/9999/1".parse::<NepaliDate>(),
            Err(Error::InvalidMonth(255))
        );
        assert!(matches!(
            "2080/1/9999".parse::<NepaliDate>(),
            Err(Error::InvalidDay { day: 255, .. })
        ));
    }

    #[test]
    fn lenient_forms() {
        let d = bs(2080, 4, 15);
        for s in [
            "15 Shrawan 2080",
            "Shrawan 15, 2080",
            "2080 Shrawan 15",
            "15.04.2080",
            "2080.04.15 B.S.",
            "15 साउन, 2080 गते",
            "१५ श्रावण २०८०",
            "15 SHRAWAN 2080",
        ] {
            assert_eq!(NepaliDate::parse_lenient(s), d, "{s:?}");
        }
        assert_eq!(
            NepaliDate::parse_lenient(""),
            kind(ParseErrorKind::Unrecognized)
        );
        assert_eq!(
            NepaliDate::parse_lenient("   "),
            kind(ParseErrorKind::Unrecognized)
        );
        assert_eq!(
            NepaliDate::parse_lenient("80/05/15"),
            kind(ParseErrorKind::Ambiguous)
        );
        assert_eq!(
            NepaliDate::parse_lenient("15 shrawan 80"),
            kind(ParseErrorKind::Ambiguous)
        );
        assert_eq!(
            NepaliDate::parse_lenient("15 shrawan 080"),
            kind(ParseErrorKind::Ambiguous)
        );
        assert_eq!(
            NepaliDate::parse_lenient("2080 shrawan 2081"),
            kind(ParseErrorKind::Ambiguous)
        );
        assert_eq!(
            NepaliDate::parse_lenient("Shrawan Bhadra 2080"),
            kind(ParseErrorKind::Unrecognized)
        );
        assert_eq!(
            NepaliDate::parse_lenient("1 2 3 4 5"),
            kind(ParseErrorKind::Unrecognized)
        );
        assert_eq!(
            NepaliDate::parse_lenient("1 2 3 4 123456"),
            kind(ParseErrorKind::NumberTooLong)
        );
        for s in [
            "15th Shrawan 2080",
            "Shr.15 2080",
            "15.Shrawan.2080",
            "2080.B.S. Shrawan 15",
        ] {
            let parsed = NepaliDate::parse_lenient(s);
            assert_eq!(parsed, kind(ParseErrorKind::Unrecognized), "{s:?}");
        }
        assert_eq!(
            NepaliDate::parse_lenient("15/13/2080"),
            Err(Error::InvalidMonth(13))
        );
    }

    #[test]
    fn aliases_are_lower_case_or_devanagari() {
        for (alias, m) in MONTH_ALIASES {
            assert!((1..=12).contains(&m));
            assert!(!alias.bytes().any(|c| c.is_ascii_uppercase()), "{alias}");
        }
    }
}
