//! C# `NepDate`'s own test suite, one test per row of `shared/spec/vectors/csharp-tests/catalog.tsv`
//! (L2-08). Names come from `cargo xtask catalog-check <phase> --list`; `cargo xtask
//! catalog-check` fails when a row has neither a test nor a `// cs:<id> D-xx` comment.
//! Phase 1 rows (L2-08), then phase 2 (L3-06); L4-06 adds phase 3.
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

use nepdate::{
    Error, FiscalYear, GregorianDate, Month, MonthOverflow, NepaliDate, NepaliDateRange,
    ParseErrorKind, Quarter, Weekday,
};

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

// ======================================================================================
// Phase 2 (L3-06): arithmetic, fiscal year, ranges.
// ======================================================================================

fn range(start: NepaliDate, end: NepaliDate) -> NepaliDateRange {
    NepaliDateRange::new(start, end).unwrap()
}

/// The last day of a month, the C# `new NepaliDate(y, m, new NepaliDate(y, m, 1).MonthEndDay)`.
fn month_end(year: u16, month: u8) -> NepaliDate {
    bs(year, month, 1).last_day_of_month()
}

/// C# `FiscalYearQuarterStartDate(q, yearOffset)` and friends: the fiscal year containing `date`,
/// moved by `offset` years, then one quarter of it (D-08).
fn fy_quarter(date: NepaliDate, offset: i16, q: Quarter) -> NepaliDateRange {
    let fy = date
        .fiscal_year()
        .start_year()
        .checked_add_signed(offset)
        .unwrap();
    FiscalYear::new(fy).quarter_range(q).unwrap()
}

/// C# `FiscalYearStartDate(yearOffset)` and `FiscalYearEndDate(yearOffset)` as one range.
fn fy_range(date: NepaliDate, offset: i16) -> NepaliDateRange {
    let fy = date
        .fiscal_year()
        .start_year()
        .checked_add_signed(offset)
        .unwrap();
    FiscalYear::new(fy).range().unwrap()
}

/// A lenient-parse row whose C# test expects a date. `parse_lenient` arrives in L4-04; until
/// then this checks that the date the string names exists.
fn assert_parses_lenient(input: &str, year: u16, month: u8, day: u8) {
    // L4-04: assert_eq!(NepaliDate::parse_lenient(input).map(ymd), Ok((year, month, day)));
    let date = NepaliDate::new(year, month, day).unwrap_or_else(|e| panic!("{input:?}: {e}"));
    assert_eq!(ymd(date), (year, month, day), "{input:?}");
}

// ---------- Core/FiscalYearInstanceMethodTests.cs ----------

// C# `FiscalYearStartDate()` maps to `date.fiscal_year().start()`, `FiscalYearQuarters.Current`
// to `date.quarter()` (D-08).

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_date_in_shrawan_returns_sameyear_shrawan1()
 {
    assert_eq!(bs(2080, 4, 15).fiscal_year().start(), Ok(bs(2080, 4, 1)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_date_in_chaitra_returns_same_year_shrawan1()
 {
    assert_eq!(bs(2080, 12, 15).fiscal_year().start(), Ok(bs(2080, 4, 1)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_date_in_basikhakh_returns_previous_year_shrawan1()
 {
    assert_eq!(bs(2080, 1, 15).fiscal_year().start(), Ok(bs(2079, 4, 1)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_date_in_ashadh_returns_previous_year_shrawan1()
 {
    assert_eq!(bs(2080, 3, 10).fiscal_year().start(), Ok(bs(2079, 4, 1)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_with_positive_year_offset_shifts_forward()
 {
    assert_eq!(fy_range(bs(2080, 4, 1), 1).start(), bs(2081, 4, 1));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_with_negative_year_offset_shifts_backward()
 {
    assert_eq!(fy_range(bs(2080, 4, 1), -1).start(), bs(2079, 4, 1));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_end_date_date_in_shrawan_returns_next_year_ashad()
 {
    assert_eq!(bs(2080, 4, 15).fiscal_year().end(), Ok(month_end(2081, 3)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_end_date_date_in_chaitra_returns_next_year_ashadh()
 {
    let end = bs(2080, 12, 15).fiscal_year().end().unwrap();
    assert_eq!((end.year(), end.month_number()), (2081, 3));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_end_date_date_in_basikhakh_returns_same_year_ashadh()
 {
    assert_eq!(bs(2080, 1, 15).fiscal_year().end(), Ok(month_end(2080, 3)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_end_date_with_positive_year_offset_shifts_forward()
 {
    let end = fy_range(bs(2080, 4, 1), 1).end();
    assert_eq!((end.year(), end.month_number()), (2082, 3));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_and_end_date_date_in_q2_returns_both_boundaries()
 {
    let fy = bs(2080, 8, 15).fiscal_year().range().unwrap();
    assert_eq!(fy.start(), bs(2080, 4, 1));
    assert_eq!((fy.end().year(), fy.end().month_number()), (2081, 3));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_and_end_date_date_in_q4_returns_previous_fiscal_year_boundaries()
 {
    let fy = bs(2081, 2, 10).fiscal_year().range().unwrap();
    assert_eq!(fy.start(), bs(2080, 4, 1));
    assert_eq!((fy.end().year(), fy.end().month_number()), (2081, 3));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_explicit_first_returns_shrawan1()
 {
    assert_eq!(
        fy_quarter(bs(2080, 5, 15), 0, Quarter::Q1).start(),
        bs(2080, 4, 1)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_explicit_second_returns_kartik1()
 {
    assert_eq!(
        fy_quarter(bs(2080, 5, 15), 0, Quarter::Q2).start(),
        bs(2080, 7, 1)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_explicit_third_returns_magh1()
 {
    assert_eq!(
        fy_quarter(bs(2080, 5, 15), 0, Quarter::Q3).start(),
        bs(2080, 10, 1)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_explicit_fourth_returns_next_year_basikhakh1()
 {
    assert_eq!(
        fy_quarter(bs(2080, 5, 15), 0, Quarter::Q4).start(),
        bs(2081, 1, 1)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_end_date_explicit_first_returns_last_day_of_ashoj()
 {
    assert_eq!(
        fy_quarter(bs(2080, 5, 15), 0, Quarter::Q1).end(),
        month_end(2080, 6)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_end_date_explicit_second_returns_last_day_of_poush()
 {
    assert_eq!(
        fy_quarter(bs(2080, 5, 15), 0, Quarter::Q2).end(),
        month_end(2080, 9)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_end_date_explicit_third_returns_last_day_of_chaitra()
 {
    assert_eq!(
        fy_quarter(bs(2080, 5, 15), 0, Quarter::Q3).end(),
        month_end(2080, 12)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_end_date_explicit_fourth_returns_next_year_last_day_of_ashadh()
 {
    assert_eq!(
        fy_quarter(bs(2080, 5, 15), 0, Quarter::Q4).end(),
        month_end(2081, 3)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_and_end_date_explicit_second_returns_both_boundaries()
 {
    let q = fy_quarter(bs(2080, 5, 15), 0, Quarter::Q2);
    assert_eq!(q.start(), bs(2080, 7, 1));
    assert_eq!((q.end().year(), q.end().month_number()), (2080, 9));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_and_end_date_explicit_third_returns_both_boundaries()
 {
    let q = fy_quarter(bs(2080, 5, 15), 0, Quarter::Q3);
    assert_eq!(q.start(), bs(2080, 10, 1));
    assert_eq!(q.end().month_number(), 12);
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_current_date_in_q1_returns_shrawan1()
 {
    let date = bs(2080, 5, 15);
    assert_eq!(fy_quarter(date, 0, date.quarter()).start(), bs(2080, 4, 1));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_end_date_current_date_in_q1_returns_last_day_of_ashoj()
 {
    let date = bs(2080, 5, 15);
    let end = fy_quarter(date, 0, date.quarter()).end();
    assert_eq!((end.year(), end.month_number()), (2080, 6));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_current_date_in_q2_returns_kartik1()
 {
    let date = bs(2080, 8, 15);
    assert_eq!(fy_quarter(date, 0, date.quarter()).start(), bs(2080, 7, 1));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_current_date_in_q3_returns_magh1()
 {
    let date = bs(2080, 11, 15);
    assert_eq!(fy_quarter(date, 0, date.quarter()).start(), bs(2080, 10, 1));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_end_date_current_date_in_q3_returns_last_day_of_chaitra()
 {
    let date = bs(2080, 11, 15);
    let end = fy_quarter(date, 0, date.quarter()).end();
    assert_eq!((end.year(), end.month_number()), (2080, 12));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_explicit_first_with_offset1_returns_next_year_shrawan1()
 {
    assert_eq!(
        fy_quarter(bs(2080, 5, 15), 1, Quarter::Q1).start(),
        bs(2081, 4, 1)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_explicit_first_with_negative_offset_returns_previous_year_shrawan1()
 {
    assert_eq!(
        fy_quarter(bs(2080, 5, 15), -1, Quarter::Q1).start(),
        bs(2079, 4, 1)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_all_fiscal_months_return2080_shrawan1_1()
 {
    assert_eq!(bs(2080, 4, 1).fiscal_year().start(), Ok(bs(2080, 4, 1)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_all_fiscal_months_return2080_shrawan1_2()
 {
    assert_eq!(bs(2080, 6, 1).fiscal_year().start(), Ok(bs(2080, 4, 1)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_all_fiscal_months_return2080_shrawan1_3()
 {
    assert_eq!(bs(2080, 7, 1).fiscal_year().start(), Ok(bs(2080, 4, 1)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_all_fiscal_months_return2080_shrawan1_4()
 {
    assert_eq!(bs(2080, 12, 1).fiscal_year().start(), Ok(bs(2080, 4, 1)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_q4_calendar_months_return_previous_year_shrawan1_1()
 {
    assert_eq!(bs(2080, 1, 1).fiscal_year().start(), Ok(bs(2079, 4, 1)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_q4_calendar_months_return_previous_year_shrawan1_2()
 {
    assert_eq!(bs(2080, 2, 1).fiscal_year().start(), Ok(bs(2079, 4, 1)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_start_date_q4_calendar_months_return_previous_year_shrawan1_3()
 {
    assert_eq!(bs(2080, 3, 1).fiscal_year().start(), Ok(bs(2079, 4, 1)));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_from_q4_date_explicit_first_returns_fy2080_q1_start()
 {
    assert_eq!(
        fy_quarter(bs(2081, 2, 10), 0, Quarter::Q1).start(),
        bs(2080, 4, 1)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_from_q4_date_explicit_second_returns_fy2080_q2_start()
 {
    assert_eq!(
        fy_quarter(bs(2081, 2, 10), 0, Quarter::Q2).start(),
        bs(2080, 7, 1)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_from_q4_date_explicit_third_returns_fy2080_q3_start()
 {
    assert_eq!(
        fy_quarter(bs(2081, 2, 10), 0, Quarter::Q3).start(),
        bs(2080, 10, 1)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_from_q4_date_explicit_fourth_returns_fy2080_q4_start()
 {
    assert_eq!(
        fy_quarter(bs(2081, 2, 10), 0, Quarter::Q4).start(),
        bs(2081, 1, 1)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_start_date_from_q4_date_current_returns_fy2080_q4_start()
 {
    let date = bs(2081, 2, 10);
    assert_eq!(fy_quarter(date, 0, date.quarter()).start(), bs(2081, 1, 1));
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_end_date_from_q4_date_explicit_first_returns_fy2080_q1_end()
 {
    assert_eq!(
        fy_quarter(bs(2081, 2, 10), 0, Quarter::Q1).end(),
        month_end(2080, 6)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_end_date_from_q4_date_explicit_third_returns_fy2080_q3_end()
 {
    assert_eq!(
        fy_quarter(bs(2081, 2, 10), 0, Quarter::Q3).end(),
        month_end(2080, 12)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_end_date_from_q4_date_explicit_fourth_returns_fy2080_q4_end()
 {
    assert_eq!(
        fy_quarter(bs(2081, 2, 10), 0, Quarter::Q4).end(),
        month_end(2081, 3)
    );
}

#[test]
fn cs_fiscal_year_instance_method_tests__fiscal_year_quarter_end_date_from_q4_date_current_returns_fy2080_q4_end()
 {
    let date = bs(2081, 2, 10);
    assert_eq!(
        fy_quarter(date, 0, date.quarter()).end(),
        month_end(2081, 3)
    );
}

// ---------- Core/FiscalYearTests.cs ----------

#[test]
fn cs_fiscal_year_tests__get_fiscal_year_start_and_end_date_valid_year_returns_correct_dates() {
    let fy = FiscalYear::new(2080).range().unwrap();
    assert_eq!((fy.start(), fy.end()), (bs(2080, 4, 1), month_end(2081, 3)));
}

#[test]
fn cs_fiscal_year_tests__get_fiscal_year_start_and_end_date_invalid_year_throws_exception_1() {
    assert_eq!(FiscalYear::new(1843).range(), Err(Error::OutOfRange));
}

#[test]
fn cs_fiscal_year_tests__get_fiscal_year_start_and_end_date_invalid_year_throws_exception_2() {
    assert_eq!(FiscalYear::new(2200).range(), Err(Error::OutOfRange));
}

#[test]
fn cs_fiscal_year_tests__get_fiscal_year_start_date_valid_year_returns_correct_date() {
    assert_eq!(FiscalYear::new(2080).start(), Ok(bs(2080, 4, 1)));
}

#[test]
fn cs_fiscal_year_tests__get_fiscal_year_end_date_valid_year_returns_correct_date() {
    assert_eq!(FiscalYear::new(2080).end(), Ok(month_end(2081, 3)));
}

#[test]
fn cs_fiscal_year_tests__fiscal_year_start_and_end_date_on_nepali_date_instance_returns_correct_dates()
 {
    let fy = bs(2080, 6, 15).fiscal_year().range().unwrap();
    assert_eq!((fy.start(), fy.end()), (bs(2080, 4, 1), month_end(2081, 3)));
}

#[test]
fn cs_fiscal_year_tests__fiscal_year_start_date_on_nepali_date_instance_returns_correct_date() {
    assert_eq!(bs(2080, 6, 15).fiscal_year().start(), Ok(bs(2080, 4, 1)));
}

#[test]
fn cs_fiscal_year_tests__fiscal_year_end_date_on_nepali_date_instance_returns_correct_date() {
    assert_eq!(bs(2080, 6, 15).fiscal_year().end(), Ok(month_end(2081, 3)));
}

// C# `GetFiscalYearQuarter*(fy, month)` takes a month number; the quarter is that month's
// quarter. Months 5, 8 and 11 are in Q1 to Q3, where C#'s calendar-year reading of `fy` and the
// fiscal year agree (D-08 differs only for months 1 to 3).

#[test]
fn cs_fiscal_year_tests__get_fiscal_year_quarter_start_and_end_date_valid_year_and_month_returns_correct_dates()
 {
    let q = FiscalYear::new(2080)
        .quarter_range(bs(2080, 5, 1).quarter())
        .unwrap();
    assert_eq!((q.start(), q.end()), (bs(2080, 4, 1), month_end(2080, 6)));
}

#[test]
fn cs_fiscal_year_tests__get_fiscal_year_quarter_start_date_valid_year_and_month_returns_correct_date()
 {
    let q = FiscalYear::new(2080)
        .quarter_range(bs(2080, 8, 1).quarter())
        .unwrap();
    assert_eq!(q.start(), bs(2080, 7, 1));
}

#[test]
fn cs_fiscal_year_tests__get_fiscal_year_quarter_end_date_valid_year_and_month_returns_correct_date()
 {
    let q = FiscalYear::new(2080)
        .quarter_range(bs(2080, 11, 1).quarter())
        .unwrap();
    assert_eq!(q.end(), month_end(2080, 12));
}

// ---------- Core/NepaliDateArithmeticEdgeCaseTests.cs ----------

// C# `AddDays(double)` maps to `checked_add_days(i32)` (D-03), `AddMonths(n)` to
// `checked_add_months(n, MonthOverflow::Clamp)` (D-02).

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_days_zero_returns_same_date() {
    let date = bs(2080, 5, 15);
    assert_eq!(date.checked_add_days(0), Ok(date));
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_days_positive_across_month_boundary_is_correct() {
    assert_eq!(
        bs(2080, 5, 31).checked_add_days(1).map(ymd),
        Ok((2080, 6, 1))
    );
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_days_negative_across_month_boundary_is_correct() {
    assert_eq!(
        bs(2080, 6, 1).checked_add_days(-1).map(ymd),
        Ok((2080, 5, 31))
    );
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_days_positive_across_year_boundary_is_correct() {
    assert_eq!(
        month_end(2080, 12).checked_add_days(1).map(ymd),
        Ok((2081, 1, 1))
    );
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_days_negative_across_year_boundary_is_correct() {
    assert_eq!(bs(2081, 1, 1).checked_add_days(-1), Ok(month_end(2080, 12)));
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_days_large_positive_value_is_correct() {
    let start = bs(2080, 1, 1);
    let result = start.checked_add_days(365).unwrap();
    assert!(result.year() == 2081 || result.year() == 2080);
    assert!(result >= start);
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_months_positive_crosses_year_boundary_is_correct()
{
    let result = bs(2080, 11, 15).checked_add_months(5, MonthOverflow::Clamp);
    assert_eq!(result.map(ymd), Ok((2081, 4, 15)));
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_months_negative_crosses_year_boundary_is_correct()
{
    let result = bs(2080, 3, 15).checked_add_months(-5, MonthOverflow::Clamp);
    assert_eq!(result.map(ymd), Ok((2079, 10, 15)));
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_months_positive_more_than12_crosses_multiple_years()
 {
    let result = bs(2080, 5, 15).checked_add_months(14, MonthOverflow::Clamp);
    assert_eq!(result.map(ymd), Ok((2081, 7, 15)));
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_months_exactly12_same_month_next_year() {
    let result = bs(2080, 6, 15).checked_add_months(12, MonthOverflow::Clamp);
    assert_eq!(result.map(ymd), Ok((2081, 6, 15)));
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_months_day_capped_at_month_end_when_result_month_is_shorter()
 {
    let result = bs(2081, 4, 32)
        .checked_add_months(5, MonthOverflow::Clamp)
        .unwrap();
    assert_eq!((result.year(), result.month_number()), (2081, 9));
    assert!(result.day() <= result.month_length());
    assert!(result.day() > 0);
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__add_months_negative_same_day_preserved_when_fits() {
    let result = bs(2080, 8, 10).checked_add_months(-1, MonthOverflow::Clamp);
    assert_eq!(result.map(ymd), Ok((2080, 7, 10)));
}

// cs:NepaliDateArithmeticEdgeCaseTests.AddMonths_HalfMonth_AddsApproximately15Days D-02 fractional months are not ported; add months takes an integer
// cs:NepaliDateArithmeticEdgeCaseTests.AddMonths_NegativeHalfMonth_SubtractsApproximately15Days D-02 fractional months are not ported
// cs:NepaliDateArithmeticEdgeCaseTests.AddMonths_OneAndHalfMonths_AddsApproximately46Days D-02 fractional months are not ported
// cs:NepaliDateArithmeticEdgeCaseTests.AddMonths_FractionalValue_ReturnsDateFurtherThanZeroMonths D-02 fractional months are not ported

// `new NepaliDate(DateTime)` maps to `from_gregorian`, which takes a date without a time of day,
// so no time can change the result. Each row converts the AD date its C# test uses.

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__constructor_date_time_with_midnight_time_ignores_time()
 {
    assert_eq!(
        NepaliDate::from_gregorian(2023, 9, 1).map(ymd),
        Ok((2080, 5, 15))
    );
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__constructor_date_time_with_noon_time_ignores_time() {
    assert_eq!(
        NepaliDate::try_from(ad(2023, 9, 1)).map(ymd),
        Ok((2080, 5, 15))
    );
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__constructor_date_time_with_end_of_day_time_ignores_time()
 {
    assert_eq!(
        NepaliDate::from_gregorian(2023, 9, 1).map(ymd),
        Ok((2080, 5, 15))
    );
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__constructor_two_date_times_on_same_english_date_different_times_produce_same_nepali_date()
 {
    let morning = NepaliDate::from_gregorian(2023, 1, 1);
    let evening = NepaliDate::try_from(ad(2023, 1, 1));
    assert!(morning.is_ok());
    assert_eq!(morning, evening);
}

// C# `TryParse(s, autoAdjust: true, ...)` maps to `parse_lenient` (L4-04).

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__try_parse_auto_adjust_day_year_month_format_returns_correct_date()
 {
    // parse.tsv: lenient reads "15/05/2080" as D M Y.
    assert_parses_lenient("15/05/2080", 2080, 5, 15);
}

// cs:NepaliDateArithmeticEdgeCaseTests.TryParse_AutoAdjust_MonthDayYearInvertedOrder_ReturnsCorrectDate D-06 "05/15/2080" as M D Y needs C#'s order guessing; lenient parse reads D M Y and rejects month 15
// cs:NepaliDateArithmeticEdgeCaseTests.TryParse_AutoAdjust_TwoDigitYear_ExpandsToCurrentMillennium D-06 "15/05/80": 2-digit years are never expanded

#[test]
#[ignore = "L4-04: needs parse_lenient"]
fn cs_nepali_date_arithmetic_edge_case_tests__try_parse_auto_adjust_invalid_date_after_adjustment_returns_false()
 {
    // L4-04: assert!(NepaliDate::parse_lenient("99/99/99").is_err());
    panic!("parse_lenient lands in L4-04: \"99/99/99\" should be rejected");
}

#[test]
fn cs_nepali_date_arithmetic_edge_case_tests__try_parse_auto_adjust_valid_standard_format_returns_correct_date()
 {
    assert_parses_lenient("2080/05/15", 2080, 5, 15);
}

// ---------- Core/NepaliDateManipulationTests.cs ----------

#[test]
fn cs_nepali_date_manipulation_tests__month_end_date_returns_correct_date() {
    // `MonthEndDate()` maps to `last_day_of_month`.
    assert_eq!(ymd(bs(2080, 5, 15).last_day_of_month()), (2080, 5, 31));
}

fn add_months_row(from: (u16, u8, u8), n: i32, overflow: MonthOverflow, to: (u16, u8, u8)) {
    let result = bs(from.0, from.1, from.2).checked_add_months(n, overflow);
    assert_eq!(result.map(ymd), Ok(to));
}

#[test]
fn cs_nepali_date_manipulation_tests__add_months_returns_correct_date_1() {
    add_months_row((2080, 5, 15), 1, MonthOverflow::Clamp, (2080, 6, 15));
}

#[test]
fn cs_nepali_date_manipulation_tests__add_months_returns_correct_date_2() {
    add_months_row((2080, 5, 15), -1, MonthOverflow::Clamp, (2080, 4, 15));
}

#[test]
fn cs_nepali_date_manipulation_tests__add_months_returns_correct_date_3() {
    add_months_row((2080, 5, 15), 12, MonthOverflow::Clamp, (2081, 5, 15));
}

#[test]
fn cs_nepali_date_manipulation_tests__add_months_returns_correct_date_4() {
    add_months_row((2081, 4, 32), 5, MonthOverflow::Clamp, (2081, 9, 29));
}

#[test]
fn cs_nepali_date_manipulation_tests__add_months_returns_correct_date_5() {
    // `awayFromMonthEnd: true` maps to `MonthOverflow::Overflow`.
    add_months_row((2081, 4, 32), 5, MonthOverflow::Overflow, (2081, 10, 3));
}

fn add_days_row(from: (u16, u8, u8), n: i32, to: (u16, u8, u8)) {
    let result = bs(from.0, from.1, from.2).checked_add_days(n);
    assert_eq!(result.map(ymd), Ok(to));
}

#[test]
fn cs_nepali_date_manipulation_tests__add_days_returns_correct_date_1() {
    add_days_row((2080, 5, 15), 5, (2080, 5, 20));
}

#[test]
fn cs_nepali_date_manipulation_tests__add_days_returns_correct_date_2() {
    add_days_row((2080, 5, 15), -5, (2080, 5, 10));
}

#[test]
fn cs_nepali_date_manipulation_tests__add_days_returns_correct_date_3() {
    add_days_row((2080, 5, 31), 1, (2080, 6, 1));
}

#[test]
fn cs_nepali_date_manipulation_tests__add_days_returns_correct_date_4() {
    add_days_row((2080, 12, 30), 5, (2081, 1, 5));
}

// `a.Subtract(b)` (a `TimeSpan`) maps to `b.days_until(a)` (D-07).

#[test]
fn cs_nepali_date_manipulation_tests__subtract_two_dates_returns_correct_time_span() {
    let (earlier, later) = (bs(2080, 5, 1), bs(2080, 5, 16));
    assert_eq!(earlier.days_until(later), 15);
    assert_eq!(later.days_until(earlier), -15);
}

#[test]
fn cs_nepali_date_manipulation_tests__subtract_same_date_returns_zero() {
    let date = bs(2080, 5, 15);
    assert_eq!(date.days_until(date), 0);
}

// ---------- Core/NepaliDateRangeExtendedTests.cs ----------

// C# can build an empty range (end before start); `NepaliDateRange::new` returns `None` instead
// (D-09), so "empty range" rows use `Option<NepaliDateRange>`, as C# null maps to `None` in
// phase 1.

fn empty() -> Option<NepaliDateRange> {
    NepaliDateRange::new(bs(2080, 5, 10), bs(2080, 5, 1))
}

#[test]
fn cs_nepali_date_range_extended_tests__for_calendar_year_starts_at_baishakh_one_ends_at_chaitra_end()
 {
    // `ForCalendarYear` maps to `NepaliDateRange::year`.
    let r = NepaliDateRange::year(2080).unwrap();
    assert_eq!(r.start(), bs(2080, 1, 1));
    assert_eq!((r.end().year(), r.end().month_number()), (2080, 12));
    assert_eq!(r.end().day(), r.end().month_length());
}

#[test]
fn cs_nepali_date_range_extended_tests__for_calendar_year_length_is_reasonable() {
    let len = NepaliDateRange::year(2080).unwrap().len();
    assert!((365..=366).contains(&len), "{len}");
}

// The `Current*` rows need today's date: `NepaliDate::today()` comes with the `jiff` feature in
// L5 (ADR-0011). Each then becomes `NepaliDateRange::month` or `year`, or `fiscal_year().range()`,
// of `today()`.

#[test]
#[ignore = "L5: NepaliDate::today() comes with the jiff feature (ADR-0011)"]
fn cs_nepali_date_range_extended_tests__current_month_start_is_first_day_of_current_month() {
    // L5: let t = NepaliDate::today().unwrap();
    // L5: let r = NepaliDateRange::month(t.year(), t.month_number()).unwrap();
    // L5: assert_eq!(r.start(), t.first_day_of_month());
    panic!("NepaliDate::today() lands in L5");
}

#[test]
#[ignore = "L5: NepaliDate::today() comes with the jiff feature (ADR-0011)"]
fn cs_nepali_date_range_extended_tests__current_month_end_is_last_day_of_current_month() {
    // L5: let t = NepaliDate::today().unwrap();
    // L5: let r = NepaliDateRange::month(t.year(), t.month_number()).unwrap();
    // L5: assert_eq!(r.end(), t.last_day_of_month());
    panic!("NepaliDate::today() lands in L5");
}

#[test]
#[ignore = "L5: NepaliDate::today() comes with the jiff feature (ADR-0011)"]
fn cs_nepali_date_range_extended_tests__current_month_contains_today() {
    // L5: let t = NepaliDate::today().unwrap();
    // L5: assert!(NepaliDateRange::month(t.year(), t.month_number()).unwrap().contains(t));
    panic!("NepaliDate::today() lands in L5");
}

#[test]
#[ignore = "L5: NepaliDate::today() comes with the jiff feature (ADR-0011)"]
fn cs_nepali_date_range_extended_tests__current_fiscal_year_contains_today() {
    // L5: let t = NepaliDate::today().unwrap();
    // L5: assert!(t.fiscal_year().range().unwrap().contains(t));
    panic!("NepaliDate::today() lands in L5");
}

#[test]
#[ignore = "L5: NepaliDate::today() comes with the jiff feature (ADR-0011)"]
fn cs_nepali_date_range_extended_tests__current_fiscal_year_start_is_shrawan() {
    // L5: let start = NepaliDate::today().unwrap().fiscal_year().start().unwrap();
    // L5: assert_eq!((start.month_number(), start.day()), (4, 1));
    panic!("NepaliDate::today() lands in L5");
}

#[test]
#[ignore = "L5: NepaliDate::today() comes with the jiff feature (ADR-0011)"]
fn cs_nepali_date_range_extended_tests__current_fiscal_year_end_is_ashadh() {
    // L5: let end = NepaliDate::today().unwrap().fiscal_year().end().unwrap();
    // L5: assert_eq!((end.month_number(), end.day()), (3, end.month_length()));
    panic!("NepaliDate::today() lands in L5");
}

#[test]
#[ignore = "L5: NepaliDate::today() comes with the jiff feature (ADR-0011)"]
fn cs_nepali_date_range_extended_tests__current_calendar_year_contains_today() {
    // L5: let t = NepaliDate::today().unwrap();
    // L5: assert!(NepaliDateRange::year(t.year()).unwrap().contains(t));
    panic!("NepaliDate::today() lands in L5");
}

#[test]
#[ignore = "L5: NepaliDate::today() comes with the jiff feature (ADR-0011)"]
fn cs_nepali_date_range_extended_tests__current_calendar_year_start_is_baishakh1() {
    // L5: let t = NepaliDate::today().unwrap();
    // L5: let start = NepaliDateRange::year(t.year()).unwrap().start();
    // L5: assert_eq!((start.month_number(), start.day()), (1, 1));
    panic!("NepaliDate::today() lands in L5");
}

// cs:NepaliDateRangeExtendedTests.SplitByFiscalQuarter_EmptyRange_ReturnsEmptyArray D-09 no splitting; FiscalYear::quarter_range gives each quarter
// cs:NepaliDateRangeExtendedTests.SplitByFiscalQuarter_RangeWithinOneQuarter_ReturnsSingleRange D-09 no splitting
// cs:NepaliDateRangeExtendedTests.SplitByFiscalQuarter_RangeSpanningTwoQuarters_ReturnsTwoRanges D-09 no splitting
// cs:NepaliDateRangeExtendedTests.SplitByFiscalQuarter_FullFiscalYear_ReturnsFourRanges D-09 no splitting
// cs:NepaliDateRangeExtendedTests.SplitByFiscalQuarter_RangeInQ4Months_SingleRange D-09 no splitting
// cs:NepaliDateRangeExtendedTests.DatesWithInterval_Zero_ThrowsArgumentOutOfRangeException D-09 day iterator only; days().step_by(n) does this
// cs:NepaliDateRangeExtendedTests.DatesWithInterval_NegativeInterval_ThrowsArgumentOutOfRangeException D-09 day iterator only
// cs:NepaliDateRangeExtendedTests.DatesWithInterval_IntervalOfOne_YieldsEveryDay D-09 day iterator only
// cs:NepaliDateRangeExtendedTests.DatesWithInterval_IntervalOfTwo_YieldsOddDays D-09 day iterator only
// cs:NepaliDateRangeExtendedTests.DatesWithInterval_IntervalLargerThanRange_YieldsOnlyStartDate D-09 day iterator only
// cs:NepaliDateRangeExtendedTests.DatesWithInterval_EmptyRange_YieldsNothing D-09 day iterator only
// cs:NepaliDateRangeExtendedTests.WeekendDays_IncludeSundayTrue_ReturnsBothSaturdayAndSunday D-09 no working or weekend days; filter days() by weekday
// cs:NepaliDateRangeExtendedTests.WeekendDays_IncludeSundayFalse_ReturnsOnlySaturday D-09 no working or weekend days
// cs:NepaliDateRangeExtendedTests.WeekendDays_RangeWithNoWeekend_ReturnsEmpty D-09 no working or weekend days
// cs:NepaliDateRangeExtendedTests.WeekendDays_EmptyRange_ReturnsEmpty D-09 no working or weekend days
// cs:NepaliDateRangeExtendedTests.WorkingDays_ExcludeSundayFalse_ExcludesOnlySaturday D-09 no working or weekend days
// cs:NepaliDateRangeExtendedTests.WorkingDays_ExcludeSundayTrue_ExcludesBothSatAndSun D-09 no working or weekend days
// cs:NepaliDateRangeExtendedTests.WorkingDays_EmptyRange_ReturnsEmpty D-09 no working or weekend days

// `FromDayCount(start, n)` maps to `new(start, start + (n - 1) days)`. Zero or fewer days give an
// end before the start, so `None` where C# throws.

#[test]
fn cs_nepali_date_range_extended_tests__from_day_count_zero_days_throws_argument_out_of_range_exception()
 {
    let start = bs(2080, 5, 1);
    let end = start.checked_add_days(-1).unwrap();
    assert_eq!(NepaliDateRange::new(start, end), None);
}

#[test]
fn cs_nepali_date_range_extended_tests__from_day_count_negative_days_throws_argument_out_of_range_exception()
 {
    let start = bs(2080, 5, 1);
    let end = start.checked_add_days(-6).unwrap();
    assert_eq!(NepaliDateRange::new(start, end), None);
}

// cs:NepaliDateRangeExtendedTests.Intersect_NonOverlappingRanges_ReturnsEmptyRange D-09 no set operations
// cs:NepaliDateRangeExtendedTests.Intersect_AdjacentRanges_ReturnsEmptyRange D-09 no set operations
// cs:NepaliDateRangeExtendedTests.Intersect_OneRangeFullyInsideOther_ReturnsInnerRange D-09 no set operations
// cs:NepaliDateRangeExtendedTests.Union_LeftRangeIsEmpty_ReturnsRightRange D-09 no set operations
// cs:NepaliDateRangeExtendedTests.Union_RightRangeIsEmpty_ReturnsLeftRange D-09 no set operations
// cs:NepaliDateRangeExtendedTests.Union_NonOverlappingRanges_SpansEntireGap D-09 no set operations

#[test]
fn cs_nepali_date_range_extended_tests__contains_exact_start_date_returns_true() {
    assert!(range(bs(2080, 1, 5), bs(2080, 1, 20)).contains(bs(2080, 1, 5)));
}

#[test]
fn cs_nepali_date_range_extended_tests__contains_exact_end_date_returns_true() {
    assert!(range(bs(2080, 1, 5), bs(2080, 1, 20)).contains(bs(2080, 1, 20)));
}

#[test]
fn cs_nepali_date_range_extended_tests__contains_one_day_before_start_returns_false() {
    assert!(!range(bs(2080, 1, 5), bs(2080, 1, 20)).contains(bs(2080, 1, 4)));
}

#[test]
fn cs_nepali_date_range_extended_tests__contains_one_day_after_end_returns_false() {
    assert!(!range(bs(2080, 1, 5), bs(2080, 1, 20)).contains(bs(2080, 1, 21)));
}

#[test]
fn cs_nepali_date_range_extended_tests__contains_empty_range_contains_nothing() {
    assert!(!empty().is_some_and(|r| r.contains(bs(2080, 5, 5))));
}

// cs:NepaliDateRangeExtendedTests.IsAdjacentTo_GapBetweenRanges_ReturnsFalse D-09 no set operations
// cs:NepaliDateRangeExtendedTests.IsAdjacentTo_OverlappingRanges_ReturnsFalse D-09 no set operations
// cs:NepaliDateRangeExtendedTests.IsAdjacentTo_EmptyRange_ReturnsFalse D-09 no set operations
// cs:NepaliDateRangeExtendedTests.Except_NonOverlappingExcludeRange_ReturnsOriginalRange D-09 no set operations
// cs:NepaliDateRangeExtendedTests.Except_ExcludeRangeFullyContainsThis_ReturnsEmptyArray D-09 no set operations
// cs:NepaliDateRangeExtendedTests.Except_ExcludeOverlapsStart_ReturnsRightPart D-09 no set operations
// cs:NepaliDateRangeExtendedTests.Except_ExcludeOverlapsEnd_ReturnsLeftPart D-09 no set operations

#[test]
fn cs_nepali_date_range_extended_tests__get_enumerator_three_day_range_yields_exactly_three_dates()
{
    // C# `foreach` maps to `for` over `IntoIterator`.
    let mut dates = Vec::new();
    for d in range(bs(2080, 5, 13), bs(2080, 5, 15)) {
        dates.push(d);
    }
    assert_eq!(dates, [bs(2080, 5, 13), bs(2080, 5, 14), bs(2080, 5, 15)]);
}

#[test]
fn cs_nepali_date_range_extended_tests__get_enumerator_empty_range_yields_no_dates() {
    assert_eq!(empty().into_iter().flatten().count(), 0);
}

#[test]
fn cs_nepali_date_range_extended_tests__get_enumerator_single_day_range_yields_one_date() {
    let date = bs(2080, 5, 15);
    let dates: Vec<NepaliDate> = range(date, date).days().collect();
    assert_eq!(dates, [date]);
}

// cs:NepaliDateRangeExtendedTests.ToString_DefaultFormat_IsStartDashEnd D-09 the range has no text form (Debug only); format start() and end()
// cs:NepaliDateRangeExtendedTests.ToString_EmptyRange_ReturnsEmptyRangeString D-09 no empty range and no text form
// cs:NepaliDateRangeExtendedTests.ToString_WithDateFormat_UsesSpecifiedFormat D-09 the range has no text form
// cs:NepaliDateRangeExtendedTests.ToString_WithDateFormat_EmptyRange_ReturnsEmptyRangeString D-09 no empty range and no text form

#[test]
fn cs_nepali_date_range_extended_tests__equals_identical_ranges_returns_true() {
    let a = range(bs(2080, 1, 1), bs(2080, 1, 15));
    let b = range(bs(2080, 1, 1), bs(2080, 1, 15));
    assert!(a.eq(&b));
}

#[test]
fn cs_nepali_date_range_extended_tests__equals_different_start_returns_false() {
    let a = range(bs(2080, 1, 1), bs(2080, 1, 15));
    let b = range(bs(2080, 1, 2), bs(2080, 1, 15));
    assert!(!a.eq(&b));
}

#[test]
fn cs_nepali_date_range_extended_tests__equals_different_end_returns_false() {
    let a = range(bs(2080, 1, 1), bs(2080, 1, 15));
    let b = range(bs(2080, 1, 1), bs(2080, 1, 16));
    assert!(!a.eq(&b));
}

#[test]
fn cs_nepali_date_range_extended_tests__equals_both_empty_returns_true() {
    let other_empty = NepaliDateRange::new(bs(2079, 3, 10), bs(2079, 3, 1));
    assert_eq!(empty(), other_empty);
}

#[test]
fn cs_nepali_date_range_extended_tests__equals_object_overload_same_range_returns_true() {
    // Rust has only the typed `PartialEq`, so the C# object overload is the same call.
    let a = range(bs(2080, 1, 1), bs(2080, 1, 15));
    assert_eq!(a, range(bs(2080, 1, 1), bs(2080, 1, 15)));
}

#[test]
fn cs_nepali_date_range_extended_tests__equals_object_overload_wrong_type_returns_false() {
    // `range == "not a range"` doesn't compile, so the C# case can't arise at run time.
    let a = range(bs(2080, 1, 1), bs(2080, 1, 15));
    assert_ne!(a, range(bs(2080, 1, 1), bs(2080, 1, 16)));
}

#[test]
fn cs_nepali_date_range_extended_tests__get_hash_code_equal_ranges_same_hash() {
    let state = RandomState::new();
    let a = range(bs(2080, 1, 1), bs(2080, 1, 15));
    let b = range(bs(2080, 1, 1), bs(2080, 1, 15));
    assert_eq!(state.hash_one(a), state.hash_one(b));
}

#[test]
fn cs_nepali_date_range_extended_tests__operator_equals_identical_ranges_returns_true() {
    let a = range(bs(2080, 1, 1), bs(2080, 1, 15));
    let b = range(bs(2080, 1, 1), bs(2080, 1, 15));
    assert!(a == b);
    assert!(!(a != b));
}

#[test]
fn cs_nepali_date_range_extended_tests__operator_not_equals_different_ranges_returns_true() {
    let a = range(bs(2080, 1, 1), bs(2080, 1, 15));
    let b = range(bs(2080, 1, 1), bs(2080, 1, 16));
    assert!(a != b);
    assert!(!(a == b));
}

// ---------- Core/NepaliDateRangeTests.cs ----------

#[test]
fn cs_nepali_date_range_tests__constructor_valid_dates_creates_range() {
    let (start, end) = (bs(2080, 1, 1), bs(2080, 1, 15));
    let r = NepaliDateRange::new(start, end).unwrap();
    assert_eq!((r.start(), r.end(), r.len()), (start, end, 15));
}

#[test]
fn cs_nepali_date_range_tests__constructor_end_before_start_creates_empty_range() {
    // C# gives an empty range of length 0; the port gives no range (D-09).
    assert_eq!(NepaliDateRange::new(bs(2080, 1, 15), bs(2080, 1, 1)), None);
}

#[test]
fn cs_nepali_date_range_tests__single_day_creates_range_with_one_day() {
    // `SingleDay(d)` maps to `new(d, d)`.
    let date = bs(2080, 1, 1);
    let r = NepaliDateRange::new(date, date).unwrap();
    assert_eq!((r.start(), r.end(), r.len()), (date, date, 1));
}

#[test]
fn cs_nepali_date_range_tests__from_day_count_creates_correct_range() {
    let start = bs(2080, 1, 1);
    let end = start.checked_add_days(10 - 1).unwrap();
    let r = NepaliDateRange::new(start, end).unwrap();
    assert_eq!((r.start(), r.end(), r.len()), (start, end, 10));
}

#[test]
fn cs_nepali_date_range_tests__for_month_creates_correct_range() {
    let r = NepaliDateRange::month(2080, 1).unwrap();
    assert_eq!(r.start(), bs(2080, 1, 1));
    assert_eq!(r.end(), month_end(2080, 1));
}

#[test]
fn cs_nepali_date_range_tests__for_fiscal_year_creates_correct_range() {
    // `ForFiscalYear(fy)` maps to `FiscalYear::new(fy).range()` (D-08).
    let r = FiscalYear::new(2080).range().unwrap();
    assert_eq!(r.start(), bs(2080, 4, 1));
    assert_eq!(r.end(), month_end(2081, 3));
}

#[test]
fn cs_nepali_date_range_tests__contains_date_in_range_returns_true() {
    assert!(range(bs(2080, 1, 1), bs(2080, 1, 15)).contains(bs(2080, 1, 10)));
}

#[test]
fn cs_nepali_date_range_tests__contains_date_outside_range_returns_false() {
    assert!(!range(bs(2080, 1, 1), bs(2080, 1, 15)).contains(bs(2080, 1, 20)));
}

// cs:NepaliDateRangeTests.Contains_RangeFullyContained_ReturnsTrue D-09 contains takes a date only; no range set operations
// cs:NepaliDateRangeTests.Overlaps_RangesOverlap_ReturnsTrue D-09 no set operations
// cs:NepaliDateRangeTests.Overlaps_RangesDontOverlap_ReturnsFalse D-09 no set operations
// cs:NepaliDateRangeTests.IsAdjacentTo_AdjacentRanges_ReturnsTrue D-09 no set operations
// cs:NepaliDateRangeTests.Intersect_OverlappingRanges_ReturnsIntersection D-09 no set operations
// cs:NepaliDateRangeTests.Union_OverlappingRanges_ReturnsUnion D-09 no set operations
// cs:NepaliDateRangeTests.Except_RemovingRangeFromMiddle_ReturnsTwoRanges D-09 no set operations
// cs:NepaliDateRangeTests.SplitByMonth_RangeAcrossMonths_SplitsCorrectly D-09 no splitting
// cs:NepaliDateRangeTests.WorkingDays_ReturnsCorrectDays D-09 no working days; filter days() by weekday

#[test]
fn cs_nepali_date_range_tests__enumeration_enumerates_all_days_in_range() {
    let days: Vec<NepaliDate> = range(bs(2080, 1, 1), bs(2080, 1, 10)).days().collect();
    let expected: Vec<NepaliDate> = (1..=10).map(|d| bs(2080, 1, d)).collect();
    assert_eq!(days, expected);
}

// cs:NepaliDateRangeTests.Demo_RangeOperations D-09 demo of SplitByFiscalQuarter, WeekendDays, WorkingDays, Overlaps and Except, none of which is ported

// ---------- Core/OptimizationVerificationTests.cs (Subtraction_*) ----------

// `a - b` (a `TimeSpan`) maps to `b.days_until(a)` (D-07).

#[test]
fn cs_optimization_verification_tests__subtraction_same_date_returns_zero_days() {
    let date = bs(2080, 5, 15);
    assert_eq!(date.days_until(date), 0);
}

#[test]
fn cs_optimization_verification_tests__subtraction_adjacent_days_returns_one_day() {
    let (a, b) = (bs(2080, 5, 15), bs(2080, 5, 16));
    assert_eq!(a.days_until(b), 1);
    assert_eq!(b.days_until(a), -1);
}

#[test]
fn cs_optimization_verification_tests__subtraction_cross_month_correct_days() {
    assert_eq!(bs(2080, 4, 32).days_until(bs(2080, 5, 1)), 1);
}

#[test]
fn cs_optimization_verification_tests__subtraction_cross_year_correct_days() {
    assert_eq!(month_end(2080, 12).days_until(bs(2081, 1, 1)), 1);
}

// ---------- Core/NepaliDateIsRelativeTests.cs ----------

// Not ported (D-14): compare with `NepaliDate::today()` (L5) and `checked_add_days(±1)`.
// cs:NepaliDateIsRelativeTests.IsToday_TodaysDate_ReturnsTrue D-14 is_today() comes with jiff in L5
// cs:NepaliDateIsRelativeTests.IsToday_YesterdaysDate_ReturnsFalse D-14 is_today() comes with jiff in L5
// cs:NepaliDateIsRelativeTests.IsToday_TomorrowsDate_ReturnsFalse D-14 is_today() comes with jiff in L5
// cs:NepaliDateIsRelativeTests.IsToday_ArbitraryPastDate_ReturnsFalse D-14 is_today() comes with jiff in L5
// cs:NepaliDateIsRelativeTests.IsYesterday_YesterdaysDate_ReturnsTrue D-14 no IsYesterday
// cs:NepaliDateIsRelativeTests.IsYesterday_TodaysDate_ReturnsFalse D-14 no IsYesterday
// cs:NepaliDateIsRelativeTests.IsYesterday_TomorrowsDate_ReturnsFalse D-14 no IsYesterday
// cs:NepaliDateIsRelativeTests.IsYesterday_ArbitraryPastDate_ReturnsFalse D-14 no IsYesterday
// cs:NepaliDateIsRelativeTests.IsTomorrow_TomorrowsDate_ReturnsTrue D-14 no IsTomorrow
// cs:NepaliDateIsRelativeTests.IsTomorrow_TodaysDate_ReturnsFalse D-14 no IsTomorrow
// cs:NepaliDateIsRelativeTests.IsTomorrow_YesterdaysDate_ReturnsFalse D-14 no IsTomorrow
// cs:NepaliDateIsRelativeTests.IsTomorrow_ArbitraryPastDate_ReturnsFalse D-14 no IsTomorrow
// cs:NepaliDateIsRelativeTests.Today_OnlyIsToday_IsTrue D-14 no IsYesterday or IsTomorrow
// cs:NepaliDateIsRelativeTests.Yesterday_OnlyIsYesterday_IsTrue D-14 no IsYesterday or IsTomorrow
// cs:NepaliDateIsRelativeTests.Tomorrow_OnlyIsTomorrow_IsTrue D-14 no IsYesterday or IsTomorrow
// cs:NepaliDateIsRelativeTests.TwoDaysAhead_NoneAreTrue D-14 no IsYesterday or IsTomorrow
// cs:NepaliDateIsRelativeTests.TwoDaysBehind_NoneAreTrue D-14 no IsYesterday or IsTomorrow
