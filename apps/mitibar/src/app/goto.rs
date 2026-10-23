//! The Go to date field (WR-14 AC3). Pure: no Slint, no OS calls. One field takes a BS date or an
//! AD date and the outcome is the BS date to open, or the same errors as the converter
//! (WR-07 AC2, AC3).

use nepdate::{Error, Lang, NepaliDate};

use crate::app::converter::{enough_to_judge, invalid_message, parse_ad, range_message};

/// The first and last AD year of the supported range (WR-07 AC3).
const FIRST_AD_YEAR: i32 = 1844;
const LAST_AD_YEAR: i32 = 2143;

/// What the field says about its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GotoOutcome {
    /// Not enough typed to judge yet, so no error shows (WR-07 AC2).
    Empty,
    /// The BS date to open.
    Date(NepaliDate),
    /// A translated message for the field.
    Error(String),
}

/// Reads `input` as a BS date, else as an AD date (WR-14 AC3).
///
/// BS goes first, with the library's lenient parser (numbers or month names, Devanagari digits).
/// An AD date is `YYYY-MM-DD` or `YYYY/MM/DD`, and only a year from 1844 to 2143 is read as AD.
/// A text that is a valid BS date is a BS date even when its year also falls in the AD range, so
/// `2026-10-22` opens BS 2026 Kartik 22, and `1850-05-05` opens AD 1850-05-05 because 1850 is not
/// a BS year. A text that is not a valid BS date but is a valid AD date, such as `2026-12-31` when
/// BS 2026 Chaitra has fewer than 31 days, is read as AD.
#[must_use]
pub fn parse(input: &str, lang: Lang) -> GotoOutcome {
    let input = input.trim();
    if input.is_empty() {
        return GotoOutcome::Empty;
    }
    let bs = NepaliDate::parse_lenient(input);
    if let Ok(date) = bs {
        return GotoOutcome::Date(date);
    }
    let mut out_of_range = bs == Err(Error::OutOfRange);
    if let Ok((year, month, day)) = parse_ad(input) {
        if (FIRST_AD_YEAR..=LAST_AD_YEAR).contains(&year) {
            match NepaliDate::from_gregorian(year, month, day) {
                Ok(date) => return GotoOutcome::Date(date),
                Err(Error::OutOfRange) => out_of_range = true,
                Err(_) => {}
            }
        } else {
            out_of_range = true;
        }
    }
    if !enough_to_judge(input) {
        return GotoOutcome::Empty;
    }
    GotoOutcome::Error(
        if out_of_range {
            range_message(lang)
        } else {
            invalid_message(lang)
        }
        .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: u16, month: u8, day: u8) -> NepaliDate {
        NepaliDate::new(year, month, day).expect("valid BS date")
    }

    #[test]
    fn wr14_ac3_a_bs_date_opens_that_date() {
        assert_eq!(
            parse("2083/07/05", Lang::En),
            GotoOutcome::Date(date(2083, 7, 5))
        );
        assert_eq!(
            parse("5 Kartik 2083", Lang::En),
            GotoOutcome::Date(date(2083, 7, 5)),
            "month names are accepted, as in the converter"
        );
        assert_eq!(
            parse("२०८३/०७/०५", Lang::Ne),
            GotoOutcome::Date(date(2083, 7, 5)),
            "Devanagari digits are accepted"
        );
        assert_eq!(
            parse("  2083-7-5  ", Lang::En),
            GotoOutcome::Date(date(2083, 7, 5)),
            "lenient: spaces and unpadded numbers"
        );
    }

    #[test]
    fn wr14_ac3_an_ad_date_opens_the_bs_date_it_falls_on() {
        // 2024-04-13 is BS 2081/01/01. Its year is also a BS year, so the rule reads it as BS;
        // the AD years that are not BS years are the unambiguous ones.
        let ad = NepaliDate::from_gregorian(1850, 5, 5).expect("AD 1850-05-05");
        assert_eq!(parse("1850-05-05", Lang::En), GotoOutcome::Date(ad));
        assert_eq!(parse("1850/05/05", Lang::En), GotoOutcome::Date(ad));
        assert_eq!(
            parse("१८५०-०५-०५", Lang::Ne),
            GotoOutcome::Date(ad),
            "Devanagari digits"
        );
        let first_ad = NepaliDate::MIN.to_gregorian();
        let text = format!(
            "{:04}-{:02}-{:02}",
            first_ad.year(),
            first_ad.month(),
            first_ad.day()
        );
        assert_eq!(parse(&text, Lang::En), GotoOutcome::Date(NepaliDate::MIN));
    }

    #[test]
    fn wr14_ac3_the_ambiguous_year_rule_reads_a_valid_bs_date_as_bs() {
        // 2026 is a BS year and an AD year. A valid BS date wins, so this is not 2026-10-22 AD.
        assert_eq!(
            parse("2026-10-22", Lang::En),
            GotoOutcome::Date(date(2026, 10, 22))
        );
        assert_eq!(
            parse("2081/01/01", Lang::En),
            GotoOutcome::Date(date(2081, 1, 1))
        );
    }

    #[test]
    fn wr14_ac3_a_text_that_is_not_a_bs_date_but_is_an_ad_date_is_read_as_ad() {
        // Find a year where Dec 31 does not exist in BS (its month is shorter than 31 days).
        let year = (1901..=2143)
            .find(|year| NepaliDate::new(*year, 12, 31).is_err())
            .expect("some BS year has a Chaitra shorter than 31 days");
        let text = format!("{year}-12-31");
        let expected = NepaliDate::from_gregorian(i32::from(year), 12, 31).expect("AD date");
        assert_eq!(parse(&text, Lang::En), GotoOutcome::Date(expected));
    }

    #[test]
    fn wr07_ac2_an_error_shows_only_with_enough_text() {
        assert_eq!(parse("", Lang::En), GotoOutcome::Empty);
        assert_eq!(parse("   ", Lang::En), GotoOutcome::Empty);
        assert_eq!(
            parse("20", Lang::En),
            GotoOutcome::Empty,
            "too short to judge"
        );
        assert_eq!(parse("2083/7", Lang::En), GotoOutcome::Empty);
        assert_eq!(
            parse("2083/13/45", Lang::En),
            GotoOutcome::Error("Invalid date".to_owned()),
            "two separators are enough"
        );
        assert_eq!(
            parse("not a date", Lang::En),
            GotoOutcome::Error("Invalid date".to_owned()),
            "eight characters are enough"
        );
        assert_eq!(
            parse("not a date", Lang::Ne),
            GotoOutcome::Error("अमान्य मिति".to_owned())
        );
    }

    #[test]
    fn wr07_ac3_a_date_outside_both_ranges_says_the_supported_range() {
        for text in ["1700-01-01", "9999/01/01", "2300/05/05", "1843-12-31"] {
            assert_eq!(
                parse(text, Lang::En),
                GotoOutcome::Error("Supported range: 1901–2199 BS (1844–2143 AD)".to_owned()),
                "{text}"
            );
        }
        assert_eq!(
            parse("1700-01-01", Lang::Ne),
            GotoOutcome::Error("समर्थित दायरा: १९०१–२१९९ वि.सं. (१८४४–२१४३ ई.सं.)".to_owned())
        );
    }

    #[test]
    fn wr14_ac3_both_edges_of_each_range_open() {
        assert_eq!(
            parse("1901/01/01", Lang::En),
            GotoOutcome::Date(NepaliDate::MIN)
        );
        assert_eq!(
            parse("2199/12/30", Lang::En),
            GotoOutcome::Date(NepaliDate::MAX)
        );
        assert!(matches!(
            parse("2143-12-31", Lang::En),
            GotoOutcome::Date(_)
        ));
    }
}
