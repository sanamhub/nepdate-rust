//! C# `NepDate`'s own test suite, one test per row of `shared/spec/vectors/csharp-tests/catalog.tsv`
//! (L2-08). Names come from `cargo xtask catalog-check <phase> --list`; `cargo xtask
//! catalog-check` fails when a row has neither a test nor a `// cs:<id> D-xx` comment.
//! Phase 1 rows here; L3-06 and L4-06 add phases 2 and 3.
// `non_snake_case`: the L2-08 name rule turns the `.` in a row id into `__`.
// `nonminimal_bool` and `manual_assert_eq`: the `Operator_*` rows test each operator itself
// (`!(a < b)` calls `<`, not `>=`), so the expressions stay as C# wrote them.
#![allow(
    non_snake_case,
    clippy::nonminimal_bool,
    clippy::manual_assert_eq,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::hash::{BuildHasher, RandomState};

use nepdate::{Error, GregorianDate, Month, NepaliDate, ParseErrorKind, Weekday};

fn bs(year: u16, month: u8, day: u8) -> NepaliDate {
    NepaliDate::new(year, month, day).unwrap()
}

fn ad(year: i32, month: u8, day: u8) -> GregorianDate {
    GregorianDate::new(year, month, day).unwrap()
}

fn ymd(date: NepaliDate) -> (u16, u8, u8) {
    (date.year(), date.month_number(), date.day())
}

/// The next AD day, found with `GregorianDate::new` only, so the AD walks below don't depend on
/// the conversion they test.
fn next_ad_day(g: GregorianDate) -> GregorianDate {
    GregorianDate::new(g.year(), g.month(), g.day() + 1)
        .or_else(|_| GregorianDate::new(g.year(), g.month() + 1, 1))
        .or_else(|_| GregorianDate::new(g.year() + 1, 1, 1))
        .unwrap()
}

fn all_bs_days() -> impl Iterator<Item = NepaliDate> {
    std::iter::successors(Some(NepaliDate::MIN), |d| d.succ())
}

fn first_days_of_months() -> impl Iterator<Item = NepaliDate> {
    (1901..=2199).flat_map(|y| (1..=12).map(move |m| bs(y, m, 1)))
}

/// A string row whose C# test expects a date. `FromStr` arrives in L4-03; until then this checks
/// that the date the string names exists.
fn assert_parses(input: &str, year: u16, month: u8, day: u8) {
    // L4-03: assert_eq!(input.parse::<NepaliDate>().map(ymd), Ok((year, month, day)));
    let date = NepaliDate::new(year, month, day).unwrap_or_else(|e| panic!("{input:?}: {e}"));
    assert_eq!(ymd(date), (year, month, day), "{input:?}");
}

/// A string row whose C# test expects a syntax error. Nothing in phase 1 can check it, so these
/// tests are `#[ignore]` until L4-03 replaces this body with the assertion below.
fn assert_parse_rejects(input: &str, expected: ParseErrorKind) {
    // L4-03: assert_eq!(input.parse::<NepaliDate>(), Err(Error::Parse(expected)));
    panic!("FromStr lands in L4-03: {input:?} should give {expected:?}");
}

// ---------- Abilities/NepaliDateComparableTests.cs ----------

// C# `IComparable` (object) and `IComparable<NepaliDate>` both map to `Ord::cmp`.

#[test]
fn cs_nepali_date_comparable_tests__compare_to_object_earlier_returns_negative() {
    assert_eq!(bs(2081, 1, 1).cmp(&bs(2081, 6, 15)), Ordering::Less);
}

#[test]
fn cs_nepali_date_comparable_tests__compare_to_object_same_returns_zero() {
    assert_eq!(bs(2081, 1, 1).cmp(&bs(2081, 1, 1)), Ordering::Equal);
}

#[test]
fn cs_nepali_date_comparable_tests__compare_to_object_later_returns_positive() {
    assert_eq!(bs(2082, 1, 1).cmp(&bs(2081, 1, 1)), Ordering::Greater);
}

#[test]
fn cs_nepali_date_comparable_tests__compare_to_object_null_returns_positive() {
    // C# null maps to `Option::None`, which sorts before every `Some(date)`.
    assert_eq!(Some(bs(2081, 1, 1)).cmp(&None), Ordering::Greater);
}

#[test]
fn cs_nepali_date_comparable_tests__compare_to_wrong_type_throws_argument_exception() {
    // Comparing with a string is a compile error in Rust (`Ord` is only `NepaliDate` with
    // `NepaliDate`), so the C# `ArgumentException` can't happen. The typed compare still works.
    let date = bs(2081, 1, 1);
    assert_eq!(date.partial_cmp(&bs(2081, 1, 1)), Some(Ordering::Equal));
}

#[test]
fn cs_nepali_date_comparable_tests__compare_to_generic_earlier_returns_negative() {
    assert!(bs(2081, 1, 1) < bs(2081, 6, 15));
}

#[test]
fn cs_nepali_date_comparable_tests__compare_to_generic_same_returns_zero() {
    assert_eq!(bs(2081, 1, 1).cmp(&bs(2081, 1, 1)), Ordering::Equal);
}

#[test]
fn cs_nepali_date_comparable_tests__compare_to_generic_later_returns_positive() {
    assert!(bs(2082, 1, 1) > bs(2081, 1, 1));
}

#[test]
fn cs_nepali_date_comparable_tests__sorted_set_orders_correctly() {
    // C# `SortedSet` maps to `BTreeSet`.
    let set: BTreeSet<NepaliDate> = [bs(2082, 1, 1), bs(2081, 1, 1), bs(2081, 6, 15)].into();
    let ordered: Vec<NepaliDate> = set.into_iter().collect();
    assert_eq!(ordered, [bs(2081, 1, 1), bs(2081, 6, 15), bs(2082, 1, 1)]);
}

#[test]
fn cs_nepali_date_comparable_tests__array_sort_via_i_comparable_orders_correctly() {
    let mut dates = [bs(2082, 1, 1), bs(2081, 1, 1), bs(2081, 6, 15)];
    dates.sort();
    assert_eq!(dates, [bs(2081, 1, 1), bs(2081, 6, 15), bs(2082, 1, 1)]);
}

#[test]
fn cs_nepali_date_comparable_tests__list_sort_orders_correctly() {
    let mut list = vec![bs(2082, 1, 1), bs(2081, 1, 1), bs(2081, 6, 15)];
    list.sort();
    assert_eq!(list, [bs(2081, 1, 1), bs(2081, 6, 15), bs(2082, 1, 1)]);
}

// ---------- Core/DictionaryIntegrityTests.cs ----------

#[test]
fn cs_dictionary_integrity_tests__all_nepali_months_roundtrip_to_english_and_back_produces_original_date()
 {
    let failures = first_days_of_months()
        .filter(|d| NepaliDate::try_from(d.to_gregorian()) != Ok(*d))
        .count();
    assert_eq!(failures, 0);
}

#[test]
fn cs_dictionary_integrity_tests__all_nepali_days_roundtrip_to_english_and_back_produces_original_date()
 {
    let mut total = 0;
    let mut failures = 0;
    for date in all_bs_days() {
        total += 1;
        if NepaliDate::try_from(date.to_gregorian()) != Ok(date) {
            failures += 1;
        }
    }
    assert_eq!(failures, 0);
    assert!(total > 100_000, "expected 100k+ days, got {total}");
}

#[test]
fn cs_dictionary_integrity_tests__all_english_days_roundtrip_to_nepali_and_back_produces_original_date()
 {
    let end = NepaliDate::MAX.to_gregorian();
    let mut g = NepaliDate::MIN.to_gregorian();
    let mut total = 0;
    let mut failures = 0;
    loop {
        total += 1;
        let back = NepaliDate::try_from(g).map(NepaliDate::to_gregorian);
        if back != Ok(g) {
            failures += 1;
        }
        if g == end {
            break;
        }
        g = next_ad_day(g);
    }
    assert_eq!(failures, 0);
    assert!(total > 100_000, "expected 100k+ days, got {total}");
}

#[test]
fn cs_dictionary_integrity_tests__all_months_month_end_day_is_within_valid_range() {
    for date in first_days_of_months() {
        assert!((29..=32).contains(&date.month_length()), "{date}");
    }
}

#[test]
fn cs_dictionary_integrity_tests__consecutive_nepali_days_produce_consecutive_english_days() {
    let mut previous: Option<GregorianDate> = None;
    let mut violations = 0;
    for date in all_bs_days() {
        let g = date.to_gregorian();
        if previous.is_some_and(|p| next_ad_day(p) != g) {
            violations += 1;
        }
        previous = Some(g);
    }
    assert_eq!(violations, 0);
}

fn known_reference(nepali: (u16, u8, u8), english: (i32, u8, u8)) {
    let date = bs(nepali.0, nepali.1, nepali.2);
    let g = ad(english.0, english.1, english.2);
    assert_eq!(date.to_gregorian(), g);
    let reverse = NepaliDate::from_gregorian(english.0, english.1, english.2).unwrap();
    assert_eq!(ymd(reverse), nepali);
}

#[test]
fn cs_dictionary_integrity_tests__known_reference_dates_convert_correctly_1() {
    known_reference((2080, 1, 1), (2023, 4, 14));
}

#[test]
fn cs_dictionary_integrity_tests__known_reference_dates_convert_correctly_2() {
    known_reference((2000, 1, 1), (1943, 4, 14));
}

#[test]
fn cs_dictionary_integrity_tests__known_reference_dates_convert_correctly_3() {
    known_reference((2081, 1, 1), (2024, 4, 13));
}

#[test]
fn cs_dictionary_integrity_tests__known_reference_dates_convert_correctly_4() {
    known_reference((1901, 1, 1), (1844, 4, 11));
}

#[test]
fn cs_dictionary_integrity_tests__boundary_dates_convert_correctly() {
    let min = NepaliDate::MIN;
    assert_eq!(ymd(min), (1901, 1, 1));
    assert_eq!(NepaliDate::try_from(min.to_gregorian()), Ok(min));

    let max = NepaliDate::MAX;
    assert_eq!((max.year(), max.month_number()), (2199, 12));
    assert_eq!(NepaliDate::try_from(max.to_gregorian()), Ok(max));
}

#[test]
fn cs_dictionary_integrity_tests__total_days_in_range_matches_english_date_span() {
    let total_nepali_days: u32 = first_days_of_months()
        .map(|d| u32::from(d.month_length()))
        .sum();

    // C# subtracts two DateTimes. `GregorianDate` has no day difference, so count the AD days.
    let end = NepaliDate::MAX.to_gregorian();
    let mut g = NepaliDate::MIN.to_gregorian();
    let mut english_span = 1;
    while g != end {
        g = next_ad_day(g);
        english_span += 1;
    }
    assert_eq!(english_span, total_nepali_days);
}

// ---------- Core/NepaliDateConstructionTests.cs ----------

#[test]
fn cs_nepali_date_construction_tests__constructor_valid_nepali_date_creates_instance() {
    assert_eq!(ymd(bs(2080, 5, 15)), (2080, 5, 15));
}

#[test]
fn cs_nepali_date_construction_tests__constructor_min_value_creates_instance() {
    assert_eq!(ymd(NepaliDate::MIN), (1901, 1, 1));
}

#[test]
fn cs_nepali_date_construction_tests__constructor_max_value_creates_instance() {
    let max = NepaliDate::MAX;
    assert_eq!((max.year(), max.month()), (2199, Month::Chaitra));
    assert!(max.day() > 0);
}

// C# throws `ArgumentOutOfRangeException`; `new` returns the matching `Error` variant.

#[test]
fn cs_nepali_date_construction_tests__constructor_invalid_nepali_date_throws_exception_1() {
    assert_eq!(NepaliDate::new(2080, 0, 1), Err(Error::InvalidMonth(0)));
}

#[test]
fn cs_nepali_date_construction_tests__constructor_invalid_nepali_date_throws_exception_2() {
    assert_eq!(NepaliDate::new(2080, 13, 1), Err(Error::InvalidMonth(13)));
}

#[test]
fn cs_nepali_date_construction_tests__constructor_invalid_nepali_date_throws_exception_3() {
    assert!(matches!(
        NepaliDate::new(2080, 1, 0),
        Err(Error::InvalidDay { day: 0, .. })
    ));
}

#[test]
fn cs_nepali_date_construction_tests__constructor_invalid_nepali_date_throws_exception_4() {
    assert!(matches!(
        NepaliDate::new(2080, 1, 33),
        Err(Error::InvalidDay { day: 33, .. })
    ));
}

#[test]
fn cs_nepali_date_construction_tests__constructor_valid_string_formats_creates_instance_1() {
    assert_parses("2080/05/15", 2080, 5, 15);
}

#[test]
fn cs_nepali_date_construction_tests__constructor_valid_string_formats_creates_instance_2() {
    assert_parses("2080-05-15", 2080, 5, 15);
}

#[test]
fn cs_nepali_date_construction_tests__constructor_valid_string_formats_creates_instance_3() {
    assert_parses("2080.05.15", 2080, 5, 15);
}

#[test]
fn cs_nepali_date_construction_tests__constructor_valid_string_formats_creates_instance_4() {
    assert_parses("2080_05_15", 2080, 5, 15);
}

#[test]
fn cs_nepali_date_construction_tests__constructor_valid_string_formats_creates_instance_5() {
    assert_parses("2080\\05\\15", 2080, 5, 15);
}

#[test]
fn cs_nepali_date_construction_tests__constructor_valid_string_formats_creates_instance_6() {
    assert_parses("2080 05 15", 2080, 5, 15);
}

#[test]
fn cs_nepali_date_construction_tests__constructor_auto_adjusted_formats_creates_instance_1() {
    // C# `autoAdjust: true` maps to `parse_lenient` (L4-04), which reads a trailing 4-digit
    // year as D M Y, the same answer here.
    assert_parses("15/05/2080", 2080, 5, 15);
}

// cs:NepaliDateConstructionTests.Constructor_AutoAdjustedFormats_CreatesInstance#2 D-06 "05/15/2080" as M D Y needs C#'s order guessing; lenient parse reads D M Y and rejects month 15
#[test]
fn cs_nepali_date_construction_tests__constructor_auto_adjusted_formats_creates_instance_3() {
    // `autoAdjust: false` is the plain strict parse.
    assert_parses("2080/05/15", 2080, 5, 15);
}

// cs:NepaliDateConstructionTests.Constructor_AutoAdjustedFormats_CreatesInstance#4 D-06 "80/05/15": 2-digit years are never expanded (parse.tsv: lenient gives Ambiguous)
// cs:NepaliDateConstructionTests.Constructor_AutoAdjust_MonthOverflowWithBoundaryDay_SwapsAsDocumented#1 D-06 "2080/13/12": no month and day swapping, month 13 is InvalidMonth
// cs:NepaliDateConstructionTests.Constructor_AutoAdjust_MonthOverflowWithBoundaryDay_SwapsAsDocumented#2 D-06 "2080/15/11": no month and day swapping, month 15 is InvalidMonth
// cs:NepaliDateConstructionTests.TryParse_AutoAdjust_MonthOverflowWithBoundaryDay_SwapsAsDocumented D-06 "2080/13/12": no month and day swapping, month 13 is InvalidMonth

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_nepali_date_construction_tests__constructor_invalid_string_formats_throws_exception_1() {
    assert_parse_rejects("", ParseErrorKind::WrongGroupCount);
}

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_nepali_date_construction_tests__constructor_invalid_string_formats_throws_exception_2() {
    assert_parse_rejects("invalid", ParseErrorKind::InvalidCharacter);
}

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_nepali_date_construction_tests__constructor_invalid_string_formats_throws_exception_3() {
    assert_parse_rejects("2080/5", ParseErrorKind::WrongGroupCount);
}

#[test]
fn cs_nepali_date_construction_tests__constructor_valid_format_invalid_date_throws_exception_1() {
    // L4-03: parse "2080/5/40" instead; strict parse validates with `new` (PARSING §1 step 4).
    assert!(matches!(
        NepaliDate::new(2080, 5, 40),
        Err(Error::InvalidDay { day: 40, .. })
    ));
}

#[test]
fn cs_nepali_date_construction_tests__constructor_english_date_converts_correctly() {
    // `new NepaliDate(DateTime)` maps to `from_gregorian`.
    assert_eq!(
        NepaliDate::from_gregorian(2023, 8, 30).map(ymd),
        Ok((2080, 5, 13))
    );
}

#[test]
fn cs_nepali_date_construction_tests__try_parse_valid_string_returns_true_with_correct_date() {
    // C# `TryParse` maps to `str::parse`, which returns a `Result` (ADR-0007).
    assert_parses("2080/05/15", 2080, 5, 15);
}

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_nepali_date_construction_tests__try_parse_invalid_string_returns_false_with_default_1() {
    assert_parse_rejects("", ParseErrorKind::WrongGroupCount);
}

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_nepali_date_construction_tests__try_parse_invalid_string_returns_false_with_default_2() {
    assert_parse_rejects("invalid", ParseErrorKind::InvalidCharacter);
}

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_nepali_date_construction_tests__try_parse_invalid_string_returns_false_with_default_3() {
    assert_parse_rejects("2080/5", ParseErrorKind::WrongGroupCount);
}

// ---------- Core/NepaliDatePropertiesTests.cs ----------

#[test]
fn cs_nepali_date_properties_tests__english_date_conversion_returns_correct_date() {
    // `EnglishDate` maps to `to_gregorian`.
    assert_eq!(bs(2080, 5, 15).to_gregorian(), ad(2023, 9, 1));
}

#[test]
fn cs_nepali_date_properties_tests__day_of_week_returns_correct_value() {
    assert_eq!(bs(2080, 5, 15).weekday(), Weekday::Friday);
}

#[test]
fn cs_nepali_date_properties_tests__month_end_day_returns_correct_value() {
    // `MonthEndDay` maps to `month_length`.
    assert_eq!(bs(2080, 5, 15).month_length(), 31);
}

#[test]
fn cs_nepali_date_properties_tests__month_name_returns_correct_enum() {
    // `MonthName` (enum `NepaliMonths`) maps to `month()` (enum `Month`).
    assert_eq!(bs(2080, 5, 15).month(), Month::Bhadra);
}

#[test]
#[ignore = "L5: NepaliDate::today() comes with the jiff feature (ADR-0011)"]
fn cs_nepali_date_properties_tests__today_returns_current_nepali_date() {
    // L5: assert_eq!(NepaliDate::today(), NepaliDate::try_from(jiff::Zoned::now().date()));
    panic!("NepaliDate::today() lands in L5");
}

#[test]
fn cs_nepali_date_properties_tests__day_of_year_first_day_of_year_returns1() {
    assert_eq!(bs(2080, 1, 1).day_of_year(), 1);
}

#[test]
fn cs_nepali_date_properties_tests__day_of_year_first_day_of_month4_equals_sum_of_months1_to3_plus1()
 {
    let expected: u16 = (1..=3)
        .map(|m| u16::from(bs(2080, m, 1).month_length()))
        .sum::<u16>()
        + 1;
    assert_eq!(bs(2080, 4, 1).day_of_year(), expected);
}

#[test]
fn cs_nepali_date_properties_tests__equals_null_object_returns_false() {
    // C# null maps to `Option::None`.
    assert!(!Some(bs(2080, 5, 15)).eq(&None));
}

#[test]
fn cs_nepali_date_properties_tests__equals_different_type_returns_false() {
    // `date == "2080/05/15"` and `date == 42` don't compile: `NepaliDate` is `PartialEq` only
    // with itself, so the C# "another type is never equal" case can't arise at run time.
    assert_ne!(bs(2080, 5, 15), bs(2080, 5, 16));
}

// ---------- Core/OptimizationVerificationTests.cs ----------

#[test]
fn cs_optimization_verification_tests__constructor_all_separators_parse_correctly_1() {
    assert_parses("2080/05/15", 2080, 5, 15);
}

#[test]
fn cs_optimization_verification_tests__constructor_all_separators_parse_correctly_2() {
    assert_parses("2080-05-15", 2080, 5, 15);
}

#[test]
fn cs_optimization_verification_tests__constructor_all_separators_parse_correctly_3() {
    assert_parses("2080.05.15", 2080, 5, 15);
}

#[test]
fn cs_optimization_verification_tests__constructor_all_separators_parse_correctly_4() {
    assert_parses("2080_05_15", 2080, 5, 15);
}

#[test]
fn cs_optimization_verification_tests__constructor_all_separators_parse_correctly_5() {
    assert_parses("2080\\05\\15", 2080, 5, 15);
}

#[test]
fn cs_optimization_verification_tests__constructor_all_separators_parse_correctly_6() {
    assert_parses("2080 05 15", 2080, 5, 15);
}

#[test]
fn cs_optimization_verification_tests__constructor_all_separators_parse_correctly_7() {
    assert_parses("2080।05।15", 2080, 5, 15);
}

#[test]
fn cs_optimization_verification_tests__constructor_all_separators_parse_correctly_8() {
    assert_parses("2080|05|15", 2080, 5, 15);
}

// C# only asks for "any exception"; the kinds come from `shared/spec/vectors/parse.tsv`.

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_optimization_verification_tests__constructor_invalid_strings_throws_exception_1() {
    assert_parse_rejects("", ParseErrorKind::WrongGroupCount);
}

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_optimization_verification_tests__constructor_invalid_strings_throws_exception_2() {
    assert_parse_rejects("invalid", ParseErrorKind::InvalidCharacter);
}

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_optimization_verification_tests__constructor_invalid_strings_throws_exception_3() {
    assert_parse_rejects("2080/05", ParseErrorKind::WrongGroupCount);
}

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_optimization_verification_tests__constructor_invalid_strings_throws_exception_4() {
    assert_parse_rejects("2080", ParseErrorKind::WrongGroupCount);
}

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_optimization_verification_tests__constructor_invalid_strings_throws_exception_5() {
    assert_parse_rejects("2080/05/15/99", ParseErrorKind::WrongGroupCount);
}

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_optimization_verification_tests__constructor_invalid_strings_throws_exception_6() {
    assert_parse_rejects("abc/def/ghi", ParseErrorKind::InvalidCharacter);
}

#[test]
#[ignore = "L4-03: needs FromStr"]
fn cs_optimization_verification_tests__constructor_null_string_throws_exception() {
    // A `&str` can't be null; the nearest input is the empty string.
    assert_parse_rejects("", ParseErrorKind::WrongGroupCount);
}

#[test]
fn cs_optimization_verification_tests__constructor_without_leading_zeros_parses_correctly_1() {
    assert_parses("2080/5/15", 2080, 5, 15);
}

#[test]
fn cs_optimization_verification_tests__constructor_without_leading_zeros_parses_correctly_2() {
    assert_parses("2080/05/5", 2080, 5, 5);
}

#[test]
fn cs_optimization_verification_tests__constructor_without_leading_zeros_parses_correctly_3() {
    assert_parses("2080/1/1", 2080, 1, 1);
}

#[test]
fn cs_optimization_verification_tests__operator_equality_same_dates_returns_true() {
    let (a, b) = (bs(2080, 5, 15), bs(2080, 5, 15));
    assert!(a == b);
    assert!(!(a != b));
}

#[test]
fn cs_optimization_verification_tests__operator_inequality_different_dates_returns_true() {
    let (a, b) = (bs(2080, 5, 15), bs(2080, 5, 16));
    assert!(a != b);
    assert!(!(a == b));
}

#[test]
fn cs_optimization_verification_tests__operator_less_than_earlier_date_returns_true() {
    let (earlier, later) = (bs(2080, 5, 15), bs(2080, 5, 16));
    let same_as_earlier = earlier;
    assert!(earlier < later);
    assert!(!(later < earlier));
    assert!(!(earlier < same_as_earlier));
}

#[test]
fn cs_optimization_verification_tests__operator_less_than_or_equal_same_and_earlier_returns_true() {
    let (earlier, later) = (bs(2080, 5, 15), bs(2080, 5, 16));
    let same_as_earlier = earlier;
    assert!(earlier <= later);
    assert!(earlier <= same_as_earlier);
    assert!(!(later <= earlier));
}

#[test]
fn cs_optimization_verification_tests__operator_greater_than_later_date_returns_true() {
    let (earlier, later) = (bs(2080, 5, 15), bs(2080, 5, 16));
    let same_as_later = later;
    assert!(later > earlier);
    assert!(!(earlier > later));
    assert!(!(later > same_as_later));
}

#[test]
fn cs_optimization_verification_tests__operator_greater_than_or_equal_same_and_later_returns_true()
{
    let (earlier, later) = (bs(2080, 5, 15), bs(2080, 5, 16));
    let same_as_later = later;
    assert!(later >= earlier);
    assert!(later >= same_as_later);
    assert!(!(earlier >= later));
}

#[test]
fn cs_optimization_verification_tests__operator_min_max_boundary_correct_ordering() {
    assert!(NepaliDate::MIN < NepaliDate::MAX);
    assert!(NepaliDate::MAX > NepaliDate::MIN);
    assert!(NepaliDate::MIN <= NepaliDate::MAX);
    assert!(NepaliDate::MAX >= NepaliDate::MIN);
}

#[test]
fn cs_optimization_verification_tests__compare_to_earlier_date_returns_negative() {
    assert_eq!(bs(2080, 5, 15).cmp(&bs(2080, 5, 16)), Ordering::Less);
}

#[test]
fn cs_optimization_verification_tests__compare_to_same_date_returns_zero() {
    let date = bs(2080, 5, 15);
    assert_eq!(date.cmp(&date), Ordering::Equal);
}

#[test]
fn cs_optimization_verification_tests__compare_to_later_date_returns_positive() {
    assert_eq!(bs(2080, 5, 16).cmp(&bs(2080, 5, 15)), Ordering::Greater);
}

#[test]
fn cs_optimization_verification_tests__compare_to_different_years_correct_ordering() {
    assert_eq!(bs(2080, 1, 1).cmp(&bs(2081, 1, 1)), Ordering::Less);
}

#[test]
fn cs_optimization_verification_tests__compare_to_different_months_correct_ordering() {
    assert_eq!(bs(2080, 1, 1).cmp(&bs(2080, 2, 1)), Ordering::Less);
}

#[test]
fn cs_optimization_verification_tests__equals_same_dates_returns_true() {
    // C# also checks `Equals(object)`; Rust has only the typed `PartialEq`.
    assert!(bs(2080, 5, 15).eq(&bs(2080, 5, 15)));
}

#[test]
fn cs_optimization_verification_tests__equals_different_dates_returns_false() {
    assert!(!bs(2080, 5, 15).eq(&bs(2080, 5, 16)));
}

#[test]
fn cs_optimization_verification_tests__get_hash_code_equal_dates_same_hash_code() {
    // `GetHashCode` maps to `Hash`; one `RandomState` gives one hash function.
    let state = RandomState::new();
    assert_eq!(
        state.hash_one(bs(2080, 5, 15)),
        state.hash_one(bs(2080, 5, 15))
    );
}

#[test]
fn cs_optimization_verification_tests__get_hash_code_stable_across_multiple_calls() {
    let state = RandomState::new();
    let date = bs(2080, 5, 15);
    assert_eq!(state.hash_one(date), state.hash_one(date));
}

// cs:OptimizationVerificationTests.IsLeapYear_ChecksEnglishYearCorrectly#1 D-12 IsLeapYear (Gregorian leap-ness of the AD year) is not ported
// cs:OptimizationVerificationTests.IsLeapYear_ChecksEnglishYearCorrectly#2 D-12 IsLeapYear is not ported
// cs:OptimizationVerificationTests.IsLeapYear_ChecksEnglishYearCorrectly#3 D-12 IsLeapYear is not ported

#[test]
fn cs_optimization_verification_tests__constructor_invalid_year_month_day_throws_exception_1() {
    assert_eq!(NepaliDate::new(2080, 0, 15), Err(Error::InvalidMonth(0)));
}

#[test]
fn cs_optimization_verification_tests__constructor_invalid_year_month_day_throws_exception_2() {
    assert_eq!(NepaliDate::new(2080, 13, 15), Err(Error::InvalidMonth(13)));
}

#[test]
fn cs_optimization_verification_tests__constructor_invalid_year_month_day_throws_exception_3() {
    assert!(matches!(
        NepaliDate::new(2080, 5, 0),
        Err(Error::InvalidDay { day: 0, .. })
    ));
}

#[test]
fn cs_optimization_verification_tests__constructor_invalid_year_month_day_throws_exception_4() {
    assert!(matches!(
        NepaliDate::new(2080, 5, 33),
        Err(Error::InvalidDay { day: 33, .. })
    ));
}

#[test]
fn cs_optimization_verification_tests__constructor_invalid_year_month_day_throws_exception_5() {
    assert_eq!(NepaliDate::new(1900, 1, 1), Err(Error::OutOfRange));
}

#[test]
fn cs_optimization_verification_tests__constructor_invalid_year_month_day_throws_exception_6() {
    assert_eq!(NepaliDate::new(2200, 1, 1), Err(Error::OutOfRange));
}

#[test]
fn cs_optimization_verification_tests__constructor_min_value_is_valid() {
    assert_eq!(ymd(NepaliDate::MIN), (1901, 1, 1));
}

#[test]
fn cs_optimization_verification_tests__constructor_max_value_is_valid() {
    let max = NepaliDate::MAX;
    assert_eq!((max.year(), max.month_number()), (2199, 12));
    assert!((29..=32).contains(&max.day()));
}

#[test]
fn cs_optimization_verification_tests__english_date_min_value_converts_and_roundtrips() {
    let min = NepaliDate::MIN;
    assert_eq!(NepaliDate::try_from(min.to_gregorian()), Ok(min));
}

#[test]
fn cs_optimization_verification_tests__english_date_max_value_converts_and_roundtrips() {
    let max = NepaliDate::MAX;
    assert_eq!(NepaliDate::try_from(max.to_gregorian()), Ok(max));
}

#[test]
fn cs_optimization_verification_tests__english_date_just_before_min_throws_exception() {
    // The day before is written out because `GregorianDate` has no day arithmetic.
    assert_eq!(NepaliDate::MIN.to_gregorian(), ad(1844, 4, 11));
    assert_eq!(
        NepaliDate::from_gregorian(1844, 4, 10),
        Err(Error::OutOfRange)
    );
}

#[test]
fn cs_optimization_verification_tests__english_date_just_after_max_throws_exception() {
    let after = next_ad_day(NepaliDate::MAX.to_gregorian());
    assert_eq!(NepaliDate::try_from(after), Err(Error::OutOfRange));
}

#[test]
fn cs_optimization_verification_tests__month_end_day_creating_date_with_exact_end_day_succeeds() {
    for year in 2070..=2090 {
        for month in 1..=12 {
            let end_day = bs(year, month, 1).month_length();
            let last_day = NepaliDate::new(year, month, end_day).unwrap();
            assert_eq!(last_day.day(), end_day);
            assert_eq!(last_day, bs(year, month, 1).last_day_of_month());
            assert!(NepaliDate::new(year, month, end_day + 1).is_err());
        }
    }
}

#[test]
fn cs_optimization_verification_tests__day_of_week_matches_english_date_day_of_week() {
    for year in (1910..=2190).step_by(30) {
        for month in (1..=12).step_by(3) {
            let date = bs(year, month, 15);
            assert_eq!(date.weekday(), date.to_gregorian().weekday(), "{date}");
        }
    }
}

// cs:OptimizationVerificationTests.IsDefault_DefaultStruct_ReturnsTrue D-01 no default NepaliDate exists (no Default impl), so there is nothing to call IsDefault on
// cs:OptimizationVerificationTests.IsDefault_ValidDate_ReturnsFalse D-01 IsDefault exists only because of the C# default value, which the port can't construct
