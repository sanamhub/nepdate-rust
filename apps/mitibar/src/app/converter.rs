//! Converter input to result text or error (WR-07, W3-01). Pure: no Slint, no OS calls. BS input
//! uses the library's lenient parser. AD input is the strict `YYYY-MM-DD` / `YYYY/MM/DD` form of
//! PARSING §1, with ASCII or Devanagari digits.

use nepdate::{DateOrder, Error, Lang, LongDate, NepaliDate, Separator};

/// Which way the converter reads its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// BS input, AD result.
    BsToAd,
    /// AD input, BS result.
    AdToBs,
}

/// The converter result area (WR-07 AC1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConvertOutcome {
    /// Not enough input to say anything yet (WR-07 AC2).
    Empty,
    /// The short and long forms of the converted date.
    Ok {
        /// `YYYY-MM-DD` for AD, `YYYY/MM/DD` for BS.
        short: String,
        /// Month name form, for example `April 13, 2024`.
        long: String,
    },
    /// A translated error message (WR-07 AC3).
    Error(String),
}

/// Converts `input` in `dir` (WR-07 AC1 to AC3).
///
/// An error appears only once the input has at least 8 characters or 2 separators; before that
/// the result area is empty, so typing does not flash red (WR-07 AC2).
#[must_use]
pub fn convert(input: &str, dir: Direction, lang: Lang) -> ConvertOutcome {
    let input = input.trim();
    if input.is_empty() {
        return ConvertOutcome::Empty;
    }
    if !enough_to_judge(input) {
        return ConvertOutcome::Empty;
    }
    match dir {
        Direction::BsToAd => match NepaliDate::parse_lenient(input) {
            Ok(date) => {
                let ad: jiff::civil::Date = date.into();
                ConvertOutcome::Ok {
                    short: ad.to_string(),
                    long: ad.strftime("%B %-d, %Y").to_string(),
                }
            }
            Err(Error::OutOfRange) => ConvertOutcome::Error(range_message(lang).to_owned()),
            Err(_) => ConvertOutcome::Error(invalid_message(lang).to_owned()),
        },
        Direction::AdToBs => match parse_ad(input) {
            Ok((year, month, day)) => match NepaliDate::from_gregorian(year, month, day) {
                Ok(date) => ConvertOutcome::Ok {
                    short: date
                        .short(DateOrder::Ymd, Separator::Slash, true, lang)
                        .to_string(),
                    long: date
                        .long(
                            LongDate {
                                weekday: false,
                                year: true,
                                pad: false,
                            },
                            lang,
                        )
                        .to_string(),
                },
                Err(Error::OutOfRange) => ConvertOutcome::Error(range_message(lang).to_owned()),
                Err(_) => ConvertOutcome::Error(invalid_message(lang).to_owned()),
            },
            Err(()) => ConvertOutcome::Error(invalid_message(lang).to_owned()),
        },
    }
}

/// Days between two BS dates (WR-07 AC4). `a` and `b` are lenient BS dates; the result is signed
/// from `a` to `b` and uses `NepaliDate::diff` for the breakdown.
#[must_use]
pub fn days_between(a: &str, b: &str, lang: Lang) -> ConvertOutcome {
    let a = a.trim();
    let b = b.trim();
    if a.is_empty() || b.is_empty() {
        return ConvertOutcome::Empty;
    }
    if !enough_to_judge(a) && !enough_to_judge(b) {
        return ConvertOutcome::Empty;
    }
    let (start, end) = match (parse_days_input(a, lang), parse_days_input(b, lang)) {
        (Ok(start), Ok(end)) => (start, end),
        (Err(outcome), _) | (_, Err(outcome)) => return outcome,
    };
    let diff = start.diff(end);
    let total = diff.total_days;
    let sign = if total < 0 { "-" } else { "" };
    let total_text = format!(
        "{sign}{} {}",
        lang.digits(total.unsigned_abs(), 1),
        unit(lang, Unit::Day, total.unsigned_abs())
    );
    let mut parts = Vec::new();
    if diff.years > 0 {
        parts.push(format!(
            "{} {}",
            lang.digits(u32::from(diff.years), 1),
            unit(lang, Unit::Year, u32::from(diff.years))
        ));
    }
    if diff.months > 0 {
        parts.push(format!(
            "{} {}",
            lang.digits(u32::from(diff.months), 1),
            unit(lang, Unit::Month, u32::from(diff.months))
        ));
    }
    if diff.days > 0 || parts.is_empty() {
        parts.push(format!(
            "{} {}",
            lang.digits(u32::from(diff.days), 1),
            unit(lang, Unit::Day, u32::from(diff.days))
        ));
    }
    ConvertOutcome::Ok {
        short: total_text,
        long: format!("{sign}{}", parts.join(", ")),
    }
}

/// The BS side of `days_between`, with the same error gating as `convert`.
fn parse_days_input(input: &str, lang: Lang) -> Result<NepaliDate, ConvertOutcome> {
    match NepaliDate::parse_lenient(input) {
        Ok(date) => Ok(date),
        Err(Error::OutOfRange) => Err(ConvertOutcome::Error(range_message(lang).to_owned())),
        Err(_) if enough_to_judge(input) => {
            Err(ConvertOutcome::Error(invalid_message(lang).to_owned()))
        }
        Err(_) => Err(ConvertOutcome::Empty),
    }
}

/// WR-07 AC2: 8 characters or 2 separators.
fn enough_to_judge(input: &str) -> bool {
    let separators = input
        .chars()
        .filter(|c| matches!(c, '/' | '-' | '.' | '\\' | '_'))
        .count();
    input.chars().count() >= 8 || separators >= 2
}

/// Parses the strict AD form `YYYY-MM-DD` or `YYYY/MM/DD` (PARSING §1), with ASCII or
/// Devanagari digits. One separator must be used throughout.
fn parse_ad(input: &str) -> Result<(i32, u8, u8), ()> {
    let mut ascii = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '0'..='9' | '/' | '-' => ascii.push(c),
            // Devanagari digits U+0966 to U+096F; the mapping is ten entries, so no library
            // change is needed (W3-01).
            '\u{0966}'..='\u{096F}' => {
                let digit = u32::from(c) - 0x0966;
                ascii.push(char::from_digit(digit, 10).ok_or(())?);
            }
            _ => return Err(()),
        }
    }
    let separator = if ascii.contains('/') {
        if ascii.contains('-') {
            return Err(());
        }
        '/'
    } else {
        if !ascii.contains('-') {
            return Err(());
        }
        '-'
    };
    let parts: Vec<&str> = ascii.split(separator).collect();
    let [year, month, day] = parts.as_slice() else {
        return Err(());
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return Err(());
    }
    let year: i32 = year.parse().map_err(|_| ())?;
    let month: u8 = month.parse().map_err(|_| ())?;
    let day: u8 = day.parse().map_err(|_| ())?;
    Ok((year, month, day))
}

/// The range sentence of WR-07 AC3, in the current language. The English text is the exact
/// requirement; the Nepali digits follow `Lang::digits` and the AD year stays Latin.
fn range_message(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Supported range: 1901–2199 BS (1844–2143 AD)",
        Lang::Ne => "समर्थित दायरा: १९०१–२१९९ वि.सं. (1844–2143 ई.सं.)",
    }
}

/// The fallback error when the text is neither a date nor out of range.
fn invalid_message(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Invalid date",
        Lang::Ne => "अमान्य मिति",
    }
}

/// A unit for the days-between breakdown.
#[derive(Clone, Copy)]
enum Unit {
    Year,
    Month,
    Day,
}

/// The singular or plural word for a unit. English needs the plural; Nepali does not change the
/// noun after a number.
fn unit(lang: Lang, unit: Unit, n: u32) -> &'static str {
    match (lang, unit) {
        (Lang::En, Unit::Year) if n == 1 => "year",
        (Lang::En, Unit::Year) => "years",
        (Lang::En, Unit::Month) if n == 1 => "month",
        (Lang::En, Unit::Month) => "months",
        (Lang::En, Unit::Day) if n == 1 => "day",
        (Lang::En, Unit::Day) => "days",
        (Lang::Ne, Unit::Year) => "वर्ष",
        (Lang::Ne, Unit::Month) => "महिना",
        (Lang::Ne, Unit::Day) => "दिन",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wr07_ac1_bs_to_ad() {
        assert_eq!(
            convert("2081/01/01", Direction::BsToAd, Lang::En),
            ConvertOutcome::Ok {
                short: "2024-04-13".to_owned(),
                long: "April 13, 2024".to_owned()
            }
        );
        assert_eq!(
            convert("15 Shrawan 2081", Direction::BsToAd, Lang::En),
            ConvertOutcome::Ok {
                short: "2024-07-30".to_owned(),
                long: "July 30, 2024".to_owned()
            }
        );
    }

    #[test]
    fn wr07_ac1_ad_to_bs() {
        assert_eq!(
            convert("2024-04-13", Direction::AdToBs, Lang::En),
            ConvertOutcome::Ok {
                short: "2081/01/01".to_owned(),
                long: "Baishakh 1, 2081".to_owned()
            }
        );
        assert_eq!(
            convert("2024/04/13", Direction::AdToBs, Lang::En),
            ConvertOutcome::Ok {
                short: "2081/01/01".to_owned(),
                long: "Baishakh 1, 2081".to_owned()
            }
        );
    }

    #[test]
    fn wr07_ac1_devanagari_digits_are_accepted() {
        let outcome = convert("२०२४/०४/१३", Direction::AdToBs, Lang::Ne);
        assert_eq!(
            outcome,
            ConvertOutcome::Ok {
                short: "२०८१/०१/०१".to_owned(),
                long: "बैशाख १, २०८१".to_owned()
            }
        );
        let outcome = convert("२०८१/०१/०१", Direction::BsToAd, Lang::Ne);
        assert_eq!(
            outcome,
            ConvertOutcome::Ok {
                short: "2024-04-13".to_owned(),
                long: "April 13, 2024".to_owned()
            }
        );
    }

    #[test]
    fn wr07_ac2_error_only_after_8_chars_or_2_separators() {
        for input in ["", "2081", "2081/0", "2081/01", "abc", "abcdef"] {
            assert_eq!(
                convert(input, Direction::BsToAd, Lang::En),
                ConvertOutcome::Empty,
                "{input:?}"
            );
        }
        for input in ["2081/01/", "abcdefgh", "2024-04-"] {
            assert!(
                matches!(
                    convert(input, Direction::BsToAd, Lang::En),
                    ConvertOutcome::Error(_)
                ),
                "{input:?}"
            );
        }
    }

    #[test]
    fn wr07_ac3_range_message() {
        let message = "Supported range: 1901–2199 BS (1844–2143 AD)";
        assert_eq!(
            convert("1900/12/30", Direction::BsToAd, Lang::En),
            ConvertOutcome::Error(message.to_owned())
        );
        assert_eq!(
            convert("1844-04-10", Direction::AdToBs, Lang::En),
            ConvertOutcome::Error(message.to_owned())
        );
        assert_eq!(
            convert("2143-12-31", Direction::AdToBs, Lang::En),
            ConvertOutcome::Error(message.to_owned())
        );
    }

    #[test]
    fn wr07_ac1_invalid_ad_and_bs() {
        assert_eq!(
            convert("2024-13-01", Direction::AdToBs, Lang::En),
            ConvertOutcome::Error("Invalid date".to_owned())
        );
        assert_eq!(
            convert("2024-04-31", Direction::AdToBs, Lang::En),
            ConvertOutcome::Error("Invalid date".to_owned())
        );
        // `parse_lenient` spills an impossible BS day into the next month (the library and C#
        // behaviour); W3-01 follows the plan and does not add a second validator.
        assert_eq!(
            convert("2081/02/32", Direction::BsToAd, Lang::En),
            ConvertOutcome::Ok {
                short: "2024-06-14".to_owned(),
                long: "June 14, 2024".to_owned()
            }
        );
    }

    #[test]
    fn wr07_ac4_days_between_signed_and_breakdown() {
        assert_eq!(
            days_between("2081/01/01", "2081/02/01", Lang::En),
            ConvertOutcome::Ok {
                short: "31 days".to_owned(),
                long: "1 month".to_owned()
            }
        );
        assert_eq!(
            days_between("2081/02/01", "2081/01/01", Lang::En),
            ConvertOutcome::Ok {
                short: "-31 days".to_owned(),
                long: "-1 month".to_owned()
            }
        );
        assert_eq!(
            days_between("2081/01/01", "2081/01/01", Lang::En),
            ConvertOutcome::Ok {
                short: "0 days".to_owned(),
                long: "0 days".to_owned()
            }
        );
    }

    #[test]
    fn wr07_ac4_days_between_nepali() {
        assert_eq!(
            days_between("2081/01/01", "2082/01/01", Lang::Ne),
            ConvertOutcome::Ok {
                short: "३६६ दिन".to_owned(),
                long: "१ वर्ष".to_owned()
            }
        );
    }

    #[test]
    fn days_between_waits_for_both_inputs() {
        assert_eq!(
            days_between("", "2081/01/01", Lang::En),
            ConvertOutcome::Empty
        );
        assert_eq!(
            days_between("2081", "2081/01/01", Lang::En),
            ConvertOutcome::Empty
        );
    }
}
