//! Proleptic Gregorian dates and days since 1970-01-01: ALGORITHM §5 (Howard Hinnant, public
//! domain), copied line by line with the spec's variable names. Floor division makes the
//! functions right for any year, not only the supported range.

/// Days since 1970-01-01 of a proleptic Gregorian date. The date must be valid.
pub(crate) const fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (m + 9) % 12; // March = 0 .. February = 11
    let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Proleptic Gregorian `(year, month, day)` of a day count since 1970-01-01.
pub(crate) const fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Gregorian leap year rule.
#[cfg_attr(not(test), expect(dead_code, reason = "GregorianDate arrives in L2"))]
pub(crate) const fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Days in a Gregorian month, 28..=31. `m` must be 1..=12.
#[cfg_attr(not(test), expect(dead_code, reason = "GregorianDate arrives in L2"))]
pub(crate) const fn greg_month_length(y: i64, m: u8) -> u8 {
    match m {
        2 if is_leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_days() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(days_from_civil(1844, 4, 11), -45_920);
        assert_eq!(days_from_civil(-1, 1, 1), -719_893);
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(-719_893), (-1, 1, 1));
    }

    #[test]
    fn round_trip_over_supported_span_and_beyond() {
        // AD 1800 to 2200 covers the supported range with margin on both sides.
        let mut z = days_from_civil(1800, 1, 1);
        let end = days_from_civil(2200, 12, 31);
        while z <= end {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
            z += 1;
        }
    }

    #[test]
    fn month_lengths() {
        assert_eq!(greg_month_length(2024, 2), 29);
        assert_eq!(greg_month_length(2023, 2), 28);
        assert_eq!(greg_month_length(1900, 2), 28);
        assert_eq!(greg_month_length(2000, 2), 29);
        assert_eq!(greg_month_length(2023, 4), 30);
        assert_eq!(greg_month_length(2023, 12), 31);
    }
}
