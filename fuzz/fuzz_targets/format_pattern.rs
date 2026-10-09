//! Any pattern formats any date without a panic, and `write_to` writes what `Display` writes.
#![no_main]

use libfuzzer_sys::fuzz_target;
use nepdate::{Lang, NepaliDate};

fuzz_target!(|input: (&str, u32, bool)| {
    let (pattern, serial, nepali) = input;
    // Any serial onto the supported range, 0 to 109_211 (ALGORITHM §2).
    let days = i32::try_from(serial % 109_212).unwrap_or(0);
    let Ok(date) = NepaliDate::MIN.checked_add_days(days) else {
        return;
    };
    let lang = if nepali { Lang::Ne } else { Lang::En };
    let formatted = date.format_pattern(pattern, lang);
    let mut out = String::new();
    formatted.write_to(&mut out).unwrap();
    assert_eq!(out, formatted.to_string());
});
