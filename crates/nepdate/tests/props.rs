//! Property tests (L3-05 arithmetic, L4-05 formatting and parsing; ADR-0014).
//!
//! proptest runs without its `std` feature: that feature pulls in `getrandom`, which needs an
//! assembler to build on the `windows-gnu` host. Without it the generator seed is fixed, so every
//! run checks the same cases; the cases are still random over the whole range. String strategies
//! from regular expressions need `std` too, so the robustness inputs are built from characters.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use nepdate::{DateOrder, Lang, LongDate, MonthOverflow, NepaliDate, Separator};
use proptest::prelude::*;

/// Any supported date: a serial from 0 to `109_211` turned into a date.
fn any_date() -> impl Strategy<Value = NepaliDate> {
    (0i32..=109_211).prop_map(|s| NepaliDate::MIN.checked_add_days(s).unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    #[test]
    fn add_days_is_undone_by_subtracting(d in any_date(), n in -120_000i32..=120_000) {
        if let Ok(e) = d.checked_add_days(n) {
            prop_assert_eq!(e.checked_add_days(-n), Ok(d));
            prop_assert_eq!(d.days_until(e), n);
        } else {
            let target = i64::from(NepaliDate::MIN.days_until(d)) + i64::from(n);
            prop_assert!(!(0..=109_211).contains(&target));
        }
    }

    #[test]
    fn bs_order_is_ad_order(a in any_date(), b in any_date()) {
        prop_assert_eq!(a.cmp(&b), a.to_gregorian().cmp(&b.to_gregorian()));
    }

    #[test]
    fn days_until_is_antisymmetric(a in any_date(), b in any_date()) {
        prop_assert_eq!(a.days_until(b), -b.days_until(a));
    }

    #[test]
    fn diff_rebuilds_the_later_date(a in any_date(), b in any_date()) {
        let diff = a.diff(b);
        let (from, to) = if diff.negative { (b, a) } else { (a, b) };
        let months = i32::from(diff.years) * 12 + i32::from(diff.months);
        let rebuilt = from
            .checked_add_months(months, MonthOverflow::Clamp)
            .and_then(|d| d.checked_add_days(i32::from(diff.days)));
        prop_assert_eq!(rebuilt, Ok(to));
        prop_assert_eq!(diff.total_days, a.days_until(b));
    }
}

fn any_separator() -> impl Strategy<Value = Separator> {
    prop::sample::select(vec![
        Separator::Slash,
        Separator::Backslash,
        Separator::Dot,
        Separator::Underscore,
        Separator::Dash,
        Separator::Space,
    ])
}

fn any_lang() -> impl Strategy<Value = Lang> {
    prop::sample::select(vec![Lang::En, Lang::Ne])
}

/// Any string of up to 40 chars from the whole of Unicode.
fn any_text() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..40).prop_map(|v| v.into_iter().collect())
}

/// Up to 20 chars that look like a date: digits of both systems, separators and a few letters.
fn date_like_text() -> impl Strategy<Value = String> {
    let chars: Vec<char> = "0123456789०१२३४५६७८९/ .-,।|BSsaun".chars().collect();
    prop::collection::vec(prop::sample::select(chars), 0..=20).prop_map(|v| v.into_iter().collect())
}

/// Every kind of pattern token at once.
const PATTERN: &str = r"dddd MMMM yy 'x' \M";

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    #[test]
    fn display_parses_back(d in any_date()) {
        prop_assert_eq!(d.to_string().parse::<NepaliDate>(), Ok(d));
    }

    #[test]
    fn short_ymd_parses_back(
        d in any_date(),
        sep in any_separator(),
        pad in any::<bool>(),
        lang in any_lang(),
    ) {
        let text = d.short(DateOrder::Ymd, sep, pad, lang).to_string();
        prop_assert_eq!(text.parse::<NepaliDate>(), Ok(d));
    }

    #[test]
    fn long_english_parses_back_leniently(d in any_date()) {
        let text = d.long(LongDate::default(), Lang::En).to_string();
        prop_assert_eq!(NepaliDate::parse_lenient(&text), Ok(d));
    }

    #[test]
    fn write_to_equals_display(d in any_date(), lang in any_lang(), sep in any_separator()) {
        fn via_write_to(write: impl Fn(&mut String) -> core::fmt::Result) -> String {
            let mut out = String::new();
            write(&mut out).unwrap();
            out
        }
        let short = d.short(DateOrder::Dmy, sep, false, lang);
        let long = d.long(LongDate { weekday: true, ..LongDate::default() }, lang);
        let pattern = d.format_pattern(PATTERN, lang);
        let digits = lang.digits(u32::from(d.day()), 3);
        let label = d.fiscal_year().label(lang);
        prop_assert_eq!(via_write_to(|o| short.write_to(o)), short.to_string());
        prop_assert_eq!(via_write_to(|o| long.write_to(o)), long.to_string());
        prop_assert_eq!(via_write_to(|o| pattern.write_to(o)), pattern.to_string());
        prop_assert_eq!(via_write_to(|o| digits.write_to(o)), digits.to_string());
        prop_assert_eq!(via_write_to(|o| label.write_to(o)), label.to_string());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    #[test]
    fn parsers_never_panic_on_any_text(s in any_text()) {
        // Reaching the end without a panic is the property; the result is either value.
        let _ = s.parse::<NepaliDate>();
        let _ = NepaliDate::parse_lenient(&s);
    }

    #[test]
    fn parsers_never_panic_on_date_like_text(s in date_like_text()) {
        let strict = s.parse::<NepaliDate>();
        let lenient = NepaliDate::parse_lenient(&s);
        // Lenient parsing accepts everything strict parsing accepts.
        if let Ok(d) = strict {
            prop_assert_eq!(lenient, Ok(d));
        }
    }
}
