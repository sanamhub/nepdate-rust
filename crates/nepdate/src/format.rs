//! Formatter values (FORMATTING §2 to §5, ADR-0004 §5).
//!
//! Each formatter is a small `Copy` value that remembers what to write. `write_to` writes
//! `&'static str` pieces (names, separators, one digit at a time) or slices of the caller's
//! pattern with `out.write_str`, so nothing allocates, no bytes are built and then checked as
//! UTF-8, and a caller with its own buffer gets static dispatch. `Display::fmt` is one
//! call to `write_to`.

use core::fmt;

use crate::{Lang, NepaliDate};

/// The order of year, month and day in [`NepaliDate::short`]. Same order as C# `DateFormats`.
///
/// ```
/// use nepdate::{DateOrder, Lang, NepaliDate, Separator};
///
/// let d = NepaliDate::new(2080, 5, 15)?;
/// assert_eq!(d.short(DateOrder::Dmy, Separator::Dash, true, Lang::En).to_string(), "15-05-2080");
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DateOrder {
    /// Year, month, day.
    #[default]
    Ymd,
    /// Year, day, month.
    Ydm,
    /// Month, year, day.
    Myd,
    /// Month, day, year.
    Mdy,
    /// Day, year, month.
    Dym,
    /// Day, month, year.
    Dmy,
}

/// The character between the numbers of [`NepaliDate::short`].
///
/// ```
/// assert_eq!(nepdate::Separator::Backslash.as_char(), '\\');
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Separator {
    /// `/`
    #[default]
    Slash,
    /// `\`
    Backslash,
    /// `.`
    Dot,
    /// `_`
    Underscore,
    /// `-`
    Dash,
    /// A space.
    Space,
}

impl Separator {
    /// The separator as a `char`.
    ///
    /// ```
    /// assert_eq!(nepdate::Separator::Dash.as_char(), '-');
    /// ```
    #[must_use]
    pub const fn as_char(self) -> char {
        match self {
            Separator::Slash => '/',
            Separator::Backslash => '\\',
            Separator::Dot => '.',
            Separator::Underscore => '_',
            Separator::Dash => '-',
            Separator::Space => ' ',
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Separator::Slash => "/",
            Separator::Backslash => "\\",
            Separator::Dot => ".",
            Separator::Underscore => "_",
            Separator::Dash => "-",
            Separator::Space => " ",
        }
    }
}

/// Options for [`NepaliDate::long`] (FORMATTING §4). The default matches C#
/// `ToLongDateString()`: no weekday, with the year, day padded to 2 digits.
///
/// ```
/// use nepdate::{Lang, LongDate, NepaliDate};
///
/// let d = NepaliDate::new(2079, 2, 6)?;
/// let opts = LongDate { weekday: true, year: false, pad: false };
/// assert_eq!(d.long(opts, Lang::En).to_string(), "Friday, Jestha 6");
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LongDate {
    /// Start with the weekday name and a comma.
    pub weekday: bool,
    /// End with a comma and the 4-digit year.
    pub year: bool,
    /// Write the day with 2 digits.
    pub pad: bool,
}

impl Default for LongDate {
    fn default() -> Self {
        Self {
            weekday: false,
            year: true,
            pad: true,
        }
    }
}

/// A number written in a [`Lang`], from [`Lang::digits`].
///
/// ```
/// let mut out = String::new();
/// nepdate::Lang::Ne.digits(5, 2).write_to(&mut out)?;
/// assert_eq!(out, "०५");
/// # Ok::<(), core::fmt::Error>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Digits {
    n: u32,
    min_width: u8,
    lang: Lang,
}

const EN_DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
// 3 bytes each in UTF-8. Written as whole `&str` pieces, so nothing is encoded at run time.
static NE_DIGITS: [&str; 10] = ["०", "१", "२", "३", "४", "५", "६", "७", "८", "९"];

impl Digits {
    pub(crate) const fn new(n: u32, min_width: u8, lang: Lang) -> Self {
        Self { n, min_width, lang }
    }

    /// Writes the digits to `out`, one `&'static str` piece per digit.
    ///
    /// # Errors
    ///
    /// Whatever `out.write_str` returns.
    ///
    /// ```
    /// let mut out = String::new();
    /// nepdate::Lang::En.digits(42, 4).write_to(&mut out)?;
    /// assert_eq!(out, "0042");
    /// # Ok::<(), core::fmt::Error>(())
    /// ```
    // SAFETY-OF-INDEX: `count` stays at most 10 (u32::MAX has 10 digits), and every digit is
    // `% 10`, so both index expressions are in bounds.
    #[allow(clippy::indexing_slicing, clippy::cast_possible_truncation)]
    pub fn write_to<W: fmt::Write + ?Sized>(&self, out: &mut W) -> fmt::Result {
        let table = match self.lang {
            Lang::En => &EN_DIGITS,
            Lang::Ne => &NE_DIGITS,
        };
        // Least significant digit first.
        let mut digits = [0u8; 10];
        let mut count = 0;
        let mut n = self.n;
        loop {
            digits[count] = (n % 10) as u8;
            count += 1;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        for _ in count..usize::from(self.min_width) {
            out.write_str(table[0])?;
        }
        for &d in digits[..count].iter().rev() {
            out.write_str(table[usize::from(d)])?;
        }
        Ok(())
    }
}

impl fmt::Display for Digits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(f)
    }
}

/// `year`, `month` and `day` of `date` as digits in `lang`.
fn year<W: fmt::Write + ?Sized>(
    out: &mut W,
    date: NepaliDate,
    width: u8,
    lang: Lang,
) -> fmt::Result {
    lang.digits(u32::from(date.year()), width).write_to(out)
}

fn month<W: fmt::Write + ?Sized>(
    out: &mut W,
    date: NepaliDate,
    width: u8,
    lang: Lang,
) -> fmt::Result {
    lang.digits(u32::from(date.month_number()), width)
        .write_to(out)
}

fn day<W: fmt::Write + ?Sized>(
    out: &mut W,
    date: NepaliDate,
    width: u8,
    lang: Lang,
) -> fmt::Result {
    lang.digits(u32::from(date.day()), width).write_to(out)
}

/// A date as three numbers, from [`NepaliDate::short`] (FORMATTING §3).
///
/// ```
/// use nepdate::{DateOrder, Lang, NepaliDate, Separator};
///
/// let d = NepaliDate::new(2080, 5, 15)?;
/// let mut out = String::new();
/// d.short(DateOrder::Ymd, Separator::Slash, true, Lang::Ne).write_to(&mut out)?;
/// assert_eq!(out, "२०८०/०५/१५");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ShortFormat {
    date: NepaliDate,
    order: DateOrder,
    sep: Separator,
    pad: bool,
    lang: Lang,
}

impl ShortFormat {
    /// Writes the date to `out` without allocating.
    ///
    /// # Errors
    ///
    /// Whatever `out.write_str` returns.
    ///
    /// ```
    /// use nepdate::{DateOrder, Lang, NepaliDate, Separator};
    ///
    /// let mut out = String::new();
    /// let d = NepaliDate::new(2080, 5, 15)?;
    /// d.short(DateOrder::Mdy, Separator::Dot, false, Lang::En).write_to(&mut out)?;
    /// assert_eq!(out, "5.15.2080");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn write_to<W: fmt::Write + ?Sized>(&self, out: &mut W) -> fmt::Result {
        let (y, md) = if self.pad { (4, 2) } else { (1, 1) };
        let (d, lang, sep) = (self.date, self.lang, self.sep.as_str());
        let parts: [Part; 3] = match self.order {
            DateOrder::Ymd => [Part::Y, Part::M, Part::D],
            DateOrder::Ydm => [Part::Y, Part::D, Part::M],
            DateOrder::Myd => [Part::M, Part::Y, Part::D],
            DateOrder::Mdy => [Part::M, Part::D, Part::Y],
            DateOrder::Dym => [Part::D, Part::Y, Part::M],
            DateOrder::Dmy => [Part::D, Part::M, Part::Y],
        };
        for (i, part) in parts.into_iter().enumerate() {
            if i > 0 {
                out.write_str(sep)?;
            }
            match part {
                Part::Y => year(out, d, y, lang)?,
                Part::M => month(out, d, md, lang)?,
                Part::D => day(out, d, md, lang)?,
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Part {
    Y,
    M,
    D,
}

impl fmt::Display for ShortFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(f)
    }
}

/// A date with the month name, from [`NepaliDate::long`] (FORMATTING §4).
///
/// ```
/// use nepdate::{Lang, LongDate, NepaliDate};
///
/// let d = NepaliDate::new(2081, 4, 15)?;
/// assert_eq!(d.long(LongDate::default(), Lang::Ne).to_string(), "साउन १५, २०८१");
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct LongFormat {
    date: NepaliDate,
    opts: LongDate,
    lang: Lang,
}

impl LongFormat {
    /// Writes the date to `out` without allocating.
    ///
    /// # Errors
    ///
    /// Whatever `out.write_str` returns.
    ///
    /// ```
    /// use nepdate::{Lang, LongDate, NepaliDate};
    ///
    /// let mut out = String::new();
    /// let opts = LongDate { weekday: true, ..LongDate::default() };
    /// NepaliDate::new(2081, 4, 15)?.long(opts, Lang::En).write_to(&mut out)?;
    /// assert_eq!(out, "Tuesday, Shrawan 15, 2081");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn write_to<W: fmt::Write + ?Sized>(&self, out: &mut W) -> fmt::Result {
        let (d, lang) = (self.date, self.lang);
        if self.opts.weekday {
            out.write_str(d.weekday().name(lang))?;
            out.write_str(", ")?;
        }
        out.write_str(d.month().name(lang))?;
        out.write_str(" ")?;
        day(out, d, if self.opts.pad { 2 } else { 1 }, lang)?;
        if self.opts.year {
            out.write_str(", ")?;
            year(out, d, 4, lang)?;
        }
        Ok(())
    }
}

impl fmt::Display for LongFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(f)
    }
}

/// A date written by a pattern such as `yyyy-MM-dd`, from [`NepaliDate::format_pattern`]
/// (FORMATTING §5).
///
/// ```
/// use nepdate::{Lang, NepaliDate};
///
/// let d = NepaliDate::new(2081, 4, 15)?;
/// assert_eq!(d.format_pattern("dd.MM.yy", Lang::En).to_string(), "15.04.81");
/// assert_eq!(d.format_pattern("MMMM d", Lang::Ne).to_string(), "साउन १५");
/// # Ok::<(), nepdate::Error>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct PatternFormat<'a> {
    date: NepaliDate,
    pattern: &'a str,
    lang: Lang,
}

impl PatternFormat<'_> {
    /// Writes the date to `out` without allocating. Literal text is written as slices of the
    /// pattern.
    ///
    /// # Errors
    ///
    /// Whatever `out.write_str` returns.
    ///
    /// ```
    /// use nepdate::{Lang, NepaliDate};
    ///
    /// let mut out = String::new();
    /// NepaliDate::new(2081, 4, 15)?.format_pattern("yyyy'BS'", Lang::En).write_to(&mut out)?;
    /// assert_eq!(out, "2081BS");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn write_to<W: fmt::Write + ?Sized>(&self, out: &mut W) -> fmt::Result {
        let (d, lang) = (self.date, self.lang);
        match self.pattern {
            "" | "G" | "g" | "d" => {
                return d
                    .short(DateOrder::Ymd, Separator::Slash, true, lang)
                    .write_to(out);
            }
            "D" => return d.long(LongDate::default(), lang).write_to(out),
            "s" => {
                return d
                    .short(DateOrder::Ymd, Separator::Dash, true, lang)
                    .write_to(out);
            }
            _ => {}
        }
        write_custom(out, d, self.pattern, lang)
    }
}

impl fmt::Display for PatternFormat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(f)
    }
}

/// The custom pattern scanner of FORMATTING §5, one pass over the bytes.
///
/// Every index where a slice starts or ends is either 0, the pattern's length, or next to an
/// ASCII byte (`\`, `'`, `y`, `M`, `d`), so it is a char boundary. `str::get` is still used, so a
/// mistake writes nothing instead of panicking.
fn write_custom<W: fmt::Write + ?Sized>(
    out: &mut W,
    d: NepaliDate,
    pattern: &str,
    lang: Lang,
) -> fmt::Result {
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let at = |i: usize| bytes.get(i).copied();
    let run_of = |start: usize, b: u8| {
        let mut end = start;
        while at(end) == Some(b) {
            end += 1;
        }
        end - start
    };
    let mut i = 0;
    while let Some(b) = at(i) {
        match b {
            b'\\' if i + 1 < len => {
                // The escaped character may be several bytes long: copy it whole.
                let mut end = i + 2;
                while !pattern.is_char_boundary(end) {
                    end += 1;
                }
                out.write_str(pattern.get(i + 1..end).unwrap_or(""))?;
                i = end;
            }
            b'\'' => {
                let start = i + 1;
                let mut end = start;
                while at(end).is_some_and(|c| c != b'\'') {
                    end += 1;
                }
                out.write_str(pattern.get(start..end).unwrap_or(""))?;
                // Skip the closing quote, if there is one.
                i = if end < len { end + 1 } else { end };
            }
            b'y' => {
                let run = run_of(i, b'y');
                if run >= 4 {
                    year(out, d, 4, lang)?;
                } else {
                    lang.digits(u32::from(d.year() % 100), 2).write_to(out)?;
                }
                i += run;
            }
            b'M' => {
                let run = run_of(i, b'M');
                match run {
                    1 => month(out, d, 1, lang)?,
                    2 => month(out, d, 2, lang)?,
                    3 => out.write_str(d.month().short_name(lang))?,
                    _ => out.write_str(d.month().name(lang))?,
                }
                i += run;
            }
            b'd' => {
                let run = run_of(i, b'd');
                day(out, d, if run == 2 { 2 } else { 1 }, lang)?;
                i += run;
            }
            _ => {
                // A literal run up to the next byte that means something.
                let mut end = i + 1;
                while at(end).is_some_and(|c| !matches!(c, b'\\' | b'\'' | b'y' | b'M' | b'd')) {
                    end += 1;
                }
                out.write_str(pattern.get(i..end).unwrap_or(""))?;
                i = end;
            }
        }
    }
    Ok(())
}

impl NepaliDate {
    /// The date as three numbers in the given order, with a separator (FORMATTING §3).
    /// `pad` gives a 4-digit year and 2-digit month and day.
    ///
    /// ```
    /// use nepdate::{DateOrder, Lang, NepaliDate, Separator};
    ///
    /// let d = NepaliDate::new(2080, 5, 15)?;
    /// assert_eq!(d.short(DateOrder::Ymd, Separator::Dash, true, Lang::En).to_string(), "2080-05-15");
    /// assert_eq!(d.short(DateOrder::Dmy, Separator::Dash, false, Lang::En).to_string(), "15-5-2080");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn short(
        self,
        order: DateOrder,
        sep: Separator,
        pad: bool,
        lang: Lang,
    ) -> ShortFormat {
        ShortFormat {
            date: self,
            order,
            sep,
            pad,
            lang,
        }
    }

    /// The date with the month name: `[<Weekday>, ]<Month> <day>[, <year>]` (FORMATTING §4).
    ///
    /// ```
    /// use nepdate::{Lang, LongDate, NepaliDate};
    ///
    /// let d = NepaliDate::new(2081, 4, 15)?;
    /// assert_eq!(d.long(LongDate::default(), Lang::En).to_string(), "Shrawan 15, 2081");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn long(self, opts: LongDate, lang: Lang) -> LongFormat {
        LongFormat {
            date: self,
            opts,
            lang,
        }
    }

    /// The date written by `pattern` (FORMATTING §5): `G`, `g`, `d`, `D`, `s` or a custom
    /// pattern of `y`, `M` and `d` runs, quoted text and `\` escapes. Digits and names follow
    /// `lang` (D-15); literal text is copied unchanged.
    ///
    /// ```
    /// use nepdate::{Lang, NepaliDate};
    ///
    /// let d = NepaliDate::new(2081, 4, 15)?;
    /// assert_eq!(d.format_pattern("s", Lang::En).to_string(), "2081-04-15");
    /// assert_eq!(d.format_pattern("MMM yyyy", Lang::En).to_string(), "Shr 2081");
    /// assert_eq!(d.format_pattern("yyyy\\M\\M-\\d\\d", Lang::En).to_string(), "2081MM-dd");
    /// # Ok::<(), nepdate::Error>(())
    /// ```
    #[must_use]
    pub const fn format_pattern(self, pattern: &str, lang: Lang) -> PatternFormat<'_> {
        PatternFormat {
            date: self,
            pattern,
            lang,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed buffer, so these tests also run in the `--no-default-features` build.
    struct Buf {
        bytes: [u8; 128],
        len: usize,
    }

    impl Buf {
        fn new() -> Self {
            Self {
                bytes: [0; 128],
                len: 0,
            }
        }

        fn written(&self) -> &[u8] {
            self.bytes.get(..self.len).unwrap_or(&[])
        }
    }

    impl fmt::Write for Buf {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let end = self.len + s.len();
            let dest = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
            dest.copy_from_slice(s.as_bytes());
            self.len = end;
            Ok(())
        }
    }

    fn bs(y: u16, m: u8, d: u8) -> NepaliDate {
        NepaliDate::new(y, m, d).unwrap_or(NepaliDate::MIN)
    }

    fn check(write: impl Fn(&mut Buf) -> fmt::Result, expected: &str) {
        let mut buf = Buf::new();
        assert_eq!(write(&mut buf), Ok(()));
        assert_eq!(buf.written(), expected.as_bytes(), "{expected}");
    }

    #[test]
    fn digits() {
        let cases = [
            (Lang::En, 0, 0, "0"),
            (Lang::En, 0, 3, "000"),
            (Lang::En, u32::MAX, 0, "4294967295"),
            (Lang::En, 123, 2, "123"),
            (Lang::Ne, 1_234_567_890, 0, "१२३४५६७८९०"),
            (Lang::Ne, 83, 2, "८३"),
            (Lang::Ne, 0, 2, "००"),
        ];
        for (lang, n, width, expected) in cases {
            check(|b| lang.digits(n, width).write_to(b), expected);
        }
    }

    #[test]
    fn short_examples() {
        let d = bs(2080, 5, 15);
        check(
            |b| {
                d.short(DateOrder::Ymd, Separator::Dash, true, Lang::En)
                    .write_to(b)
            },
            "2080-05-15",
        );
        check(
            |b| {
                d.short(DateOrder::Dmy, Separator::Dash, false, Lang::En)
                    .write_to(b)
            },
            "15-5-2080",
        );
        check(
            |b| {
                d.short(DateOrder::Ymd, Separator::Slash, true, Lang::Ne)
                    .write_to(b)
            },
            "२०८०/०५/१५",
        );
        let orders = [
            (DateOrder::Ydm, "2080/15/05"),
            (DateOrder::Myd, "05/2080/15"),
            (DateOrder::Mdy, "05/15/2080"),
            (DateOrder::Dym, "15/2080/05"),
        ];
        for (order, expected) in orders {
            check(
                |b| d.short(order, Separator::Slash, true, Lang::En).write_to(b),
                expected,
            );
        }
        let seps = [
            (Separator::Backslash, "2080\\05\\15"),
            (Separator::Dot, "2080.05.15"),
            (Separator::Underscore, "2080_05_15"),
            (Separator::Space, "2080 05 15"),
        ];
        for (sep, expected) in seps {
            check(
                |b| d.short(DateOrder::Ymd, sep, true, Lang::En).write_to(b),
                expected,
            );
        }
    }

    #[test]
    fn long_examples() {
        let weekday = LongDate {
            weekday: true,
            ..LongDate::default()
        };
        let short = LongDate {
            weekday: true,
            year: false,
            pad: false,
        };
        let cases = [
            (
                bs(2081, 4, 15),
                LongDate::default(),
                "Shrawan 15, 2081",
                "साउन १५, २०८१",
            ),
            (
                bs(2081, 4, 15),
                weekday,
                "Tuesday, Shrawan 15, 2081",
                "मङ्गलवार, साउन १५, २०८१",
            ),
            (
                NepaliDate::MIN,
                LongDate::default(),
                "Baishakh 01, 1901",
                "बैशाख ०१, १९०१",
            ),
            (bs(2079, 2, 6), short, "Friday, Jestha 6", "शुक्रवार, जेठ ६"),
        ];
        for (d, opts, en, ne) in cases {
            check(|b| d.long(opts, Lang::En).write_to(b), en);
            check(|b| d.long(opts, Lang::Ne).write_to(b), ne);
        }
    }

    #[test]
    fn pattern_examples() {
        let d = bs(2081, 4, 15);
        let cases = [
            ("yyyy-MM-dd", "2081-04-15"),
            ("M/d/yyyy", "4/15/2081"),
            ("yyyy'BS'", "2081BS"),
            ("yyyy\\M\\M-\\d\\d", "2081MM-dd"),
            ("MMM yyyy", "Shr 2081"),
            ("dd.MM.yy", "15.04.81"),
            ("", "2081/04/15"),
            ("G", "2081/04/15"),
            ("g", "2081/04/15"),
            ("d", "2081/04/15"),
            ("D", "Shrawan 15, 2081"),
            ("s", "2081-04-15"),
            ("ddd", "15"),
            ("y", "81"),
            ("yyyyy", "2081"),
            ("MMMMM", "Shrawan"),
            ("'unterminated", "unterminated"),
            ("yyyy\\", "2081\\"),
            ("\\साउन", "साउन"),
            ("Gd", "G15"),
            ("साल yyyy", "साल 2081"),
        ];
        for (pattern, expected) in cases {
            check(
                |b| d.format_pattern(pattern, Lang::En).write_to(b),
                expected,
            );
        }
        check(
            |b| d.format_pattern("s", Lang::Ne).write_to(b),
            "२०८१-०४-१५",
        );
        check(|b| d.format_pattern("MMM", Lang::Ne).write_to(b), "साउन");
        check(|b| d.format_pattern("'y' yy", Lang::Ne).write_to(b), "y ८१");
    }

    #[test]
    fn display_writes_through_write_to() {
        use core::fmt::Write;
        let d = bs(2081, 4, 15);
        let mut buf = Buf::new();
        assert_eq!(write!(buf, "{d}"), Ok(()));
        assert_eq!(buf.written(), b"2081/04/15");
    }

    #[test]
    fn full_buffer_is_an_error_not_a_panic() {
        // 140 literal bytes, more than the 128-byte buffer holds.
        const LONG: &str = "0123456789012345678901234567890123456789012345678901234567890123456789                            0123456789012345678901234567890123456789012345678901234567890123456789";
        let mut buf = Buf::new();
        let d = bs(2081, 4, 15);
        assert_eq!(
            d.format_pattern(LONG, Lang::En).write_to(&mut buf),
            Err(fmt::Error)
        );
    }
}
