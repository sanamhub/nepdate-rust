//! Shared helpers for the integration tests: read files from the repository's `shared/` copy,
//! which isn't published with the crate (L2-04).

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names
)]

use nepdate::NepaliDate;

/// Contents of a file under `shared/`.
pub(crate) fn read_shared(path: &str) -> String {
    let full = format!("{}/../../shared/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("reading {full}: {e}"))
}

/// Data rows of a delimited file, header skipped, each split on `sep`.
pub(crate) fn rows(path: &str, sep: char) -> Vec<Vec<String>> {
    read_shared(path)
        .lines()
        .skip(1)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split(sep).map(str::to_owned).collect())
        .collect()
}

/// `YYYY-MM-DD` as three numbers.
pub(crate) fn ymd(s: &str) -> (i32, u8, u8) {
    let mut parts = s.split('-');
    let y = parts.next().unwrap().parse().unwrap();
    let m = parts.next().unwrap().parse().unwrap();
    let d = parts.next().unwrap().parse().unwrap();
    (y, m, d)
}

/// A BS date written `YYYY-MM-DD`.
pub(crate) fn bs(s: &str) -> NepaliDate {
    let (y, m, d) = ymd(s);
    NepaliDate::new(u16::try_from(y).unwrap(), m, d).unwrap_or_else(|e| panic!("{s}: {e}"))
}
