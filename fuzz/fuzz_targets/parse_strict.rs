//! Strict parsing never panics, and a date it returns prints and parses back to itself.
#![no_main]

use libfuzzer_sys::fuzz_target;
use nepdate::NepaliDate;

fuzz_target!(|s: &str| {
    if let Ok(d) = s.parse::<NepaliDate>() {
        assert_eq!(d.to_string().parse::<NepaliDate>(), Ok(d));
    }
});
