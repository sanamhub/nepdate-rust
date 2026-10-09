# L2: BS ↔ AD conversion

**Goal:** exact, allocation-free, table-driven conversion both ways across the whole range
(ADR-0004 §4), proven by vectors, an exhaustive test, C# parity on `dates.tsv` (conversion
columns), the PORT-PLAN phase 1 rows of the C# test catalogue, and a zero-allocation test.

Read first: `shared/spec/ALGORITHM.md` §4–7, ADR-0004, ADR-0009, ADR-0014,
`shared/spec/PARITY.md` §1, `shared/spec/PORT-PLAN.md` ("C# test catalogue", phase 1).
Human reads: `docs/learn/03-const-fn-and-tests.md`.

## Tasks

### L2-01 Serial ↔ BS (`src/calendar.rs`)

`NepaliDate::to_serial(self) -> u32` and `NepaliDate::from_serial(u32) -> Result<Self, Error>`
(`pub(crate)`), with the table steps of ADR-0004 §4:

```rust
const EST_MUL: u64 = 137_797;   // round(2^22 / (109_212 / 3_588)), ADR-0004 §4

pub(crate) fn to_serial(self) -> u32 {
    let i = month_index(self.year, self.month);          // (y - 1901) * 12 + m - 1
    (MONTH_INFO[i] >> 2) + u32::from(self.day) - 1
}

pub(crate) fn from_serial(s: u32) -> Result<Self, Error> {
    if s > MAX_SERIAL { return Err(Error::OutOfRange); }
    let mut i = ((u64::from(s) * EST_MUL) >> 22) as usize;  // never above 3587 for s <= MAX_SERIAL
    if MONTH_INFO[i] >> 2 > s { i -= 1; } else if MONTH_INFO[i + 1] >> 2 <= s { i += 1; }
    // year 1901 + i / 12, month i % 12 + 1, day s - (MONTH_INFO[i] >> 2) + 1
}
```

No loop and no binary search. Clippy's `indexing_slicing` and `cast_possible_truncation` fire
here: allow them on these two functions with a comment that names the range check that proves
each index.

AC:
- `to_serial(MIN) == 0`, `to_serial(MAX) == 109_211`, `from_serial(109_212)` → `OutOfRange`.
- Unit test `month_estimate_is_off_by_at_most_one`: for every serial, the estimate
  `(s * EST_MUL) >> 22` differs from the true month index (found with `partition_point`) by 0 or 1.
  Measured on 2026-10-08: the maximum is 1.

### L2-02 `GregorianDate` (`src/gregorian.rs`)

- `GregorianDate` per `docs/API.md`. `new` validates month and day with `civil::is_leap` and
  `civil::greg_month_length` (L1-02), for any `i32` year.
- `GregorianDate::weekday()` = `(days_from_civil(y, m, d) + 4).rem_euclid(7)` in `i64`
  (1970-01-01 was a Thursday). This is the only run-time use of ALGORITHM §5; it is not a hot path.

AC: `GregorianDate::new(2023, 2, 29)` → `InvalidGregorian`; `new(2024, 2, 29)` is Ok;
`new(1900, 2, 29)` → `InvalidGregorian`; `new(2000, 2, 29)` is Ok; weekday of 1970-01-01 is
Thursday and of -0001-01-01 is Friday (proleptic).

### L2-03 Conversion API (`src/date.rs`)

- `to_gregorian()`: unpack `BS_MONTH_AD[month_index]` (`year = v >> 9`, `month = (v >> 5) & 0xF`,
  `day = v & 0x1F`), add `day - 1`, and if that passes `greg_month_length(year, month)`, carry into
  the next AD month (December carries into January of the next year). One carry is enough: the
  offset `day - 1` is at most 31 and every AD month has at least 28 days, so the result can pass
  the end of the start's AD month but never the end of the next one. The exhaustive test proves it.
- `from_gregorian(y, m, d)`: Gregorian validity first (`InvalidGregorian`), then
  `y` in 1844..=2143 else `OutOfRange` (check before multiplying, so huge years can't overflow),
  `j = (y - 1844) * 12 + m - 4`, `j` in `0..3589` else `OutOfRange`,
  `s = AD_MONTH_START[j] + d - 1`, `s` in `0..=109_211` else `OutOfRange`, then `from_serial(s)`.
- `TryFrom<GregorianDate>`, `From<NepaliDate> for GregorianDate`.
- `weekday()` = ALGORITHM §7 (`(serial + 4) % 7`), `day_of_year()` (serial minus the serial of
  month 1 of that year, from `MONTH_INFO`), `succ()`, `pred()`, `days_until()`.

AC:
- Doctest example `NepaliDate::from_gregorian(2024, 4, 13)? == NepaliDate::new(2081, 1, 1)?`.
- `from_gregorian(1844, 4, 11)` = `MIN` and `from_gregorian(2143, 4, 15)` = `MAX`;
  `from_gregorian(1844, 4, 10)`, `from_gregorian(1844, 4, 1)` and `from_gregorian(2143, 4, 16)` →
  `OutOfRange`; `from_gregorian(2023, 2, 29)` → `InvalidGregorian`;
  `from_gregorian(i32::MAX, 1, 1)` and `from_gregorian(i32::MIN, 1, 1)` → `OutOfRange`, no panic.
- `MAX.succ() == None`, `MIN.pred() == None`, `MIN.days_until(MAX) == 109_211`.

### L2-04 Test data loader (`crates/nepdate/tests/common/mod.rs`)

Tests need the repo's `shared/spec/` files, which aren't published with the crate. Use
`concat!(env!("CARGO_MANIFEST_DIR"), "/../../shared/spec/vectors/...")` + `std::fs::read_to_string`.
Write a tiny TSV/CSV splitter (`line.split('\t')`), no csv crate. Skip the header line.
Integration tests are allowed to use `unwrap()`/`expect()` (add
`#![allow(clippy::unwrap_used, clippy::expect_used)]` at the top of test files).

### L2-05 Conformance test (`tests/vectors.rs`)

For every row of `shared/spec/vectors/month-boundaries.csv`: BS→AD, AD→BS, weekday name,
month_length. Report the failing row in the assertion message. Assert that exactly 7176 data
rows were read, so a truncated file can't pass.

### L2-06 Exhaustive test (`tests/exhaustive.rs`)

The test file contains `mod reference`: ALGORITHM §4 (`to_serial` with the month loop,
`from_serial` with the year search) and §5 (Hinnant) copied line by line, over the month lengths
read from `shared/data/calendar/bs-calendar.json` with a 20-line hand-written reader (no JSON
crate). For every serial `0..=109_211`:
- `from_serial → to_serial` is the identity;
- the library's `from_serial(s)` equals `reference::from_serial(s)`, and `to_gregorian` equals
  `reference::civil_from_days(s + EPOCH_UNIX_DAYS)`;
- `from_gregorian(to_gregorian(d)) == d`;
- consecutive serials give consecutive AD days.

### L2-07 C# parity: conversions (`tests/csharp_parity.rs`, first part)

From `csharp-golden/dates.tsv`: columns `ad`, `weekday`, `day_of_year`, `month_length`,
`to_string`. (Other columns are asserted in L3/L4. Leave `// L3:` / `// L4:` markers.)

### L2-08 C# test catalogue: tool and phase 1 rows

Needs hub task H5 (`shared/spec/vectors/csharp-tests/catalog.tsv`). If the file is not in
`shared/` yet, do the rest of L2, then stop and tell the human.

- `cargo xtask catalog-check <phase>` (`phase` = `1` to `4`, the PORT-PLAN phase): reads the
  catalogue, takes the rows whose `phase` column equals `<phase>`, and for each row looks in
  `crates/*/tests/*.rs` for either `fn cs_<name>(` or a comment `// cs:<id> D-xx`
  (`D-xx` must exist in `shared/spec/PARITY.md`). Exits 1 and lists the missing rows.
  `<name>` is the row id made into a Rust identifier: `.` becomes `__`, `#` becomes `_`,
  CamelCase becomes snake_case, anything else outside `[a-z0-9_]` becomes `_`. Example:
  `NepaliDateTests.AddDays_Positive#3` → `cs_nepali_date_tests__add_days_positive_3`.
  `cargo xtask catalog-check <phase> --list` prints the expected names, so the implementer copies
  them.
- `crates/nepdate/tests/csharp_catalog.rs`: one `#[test]` per phase-1 row, asserting what the
  row's `asserts` column says, with the Rust API. Where the C# test uses an API this port maps
  differently (ADR-0007, ADR-0009), test the mapped API and say so in a one-line comment.
- Add `cargo xtask catalog-check 1` to `cargo xtask ci`.

AC: `cargo xtask catalog-check 1` exits 0; deleting one `cs_` test makes it exit 1 and name the
row; xtask unit tests cover the name rule with the example above and a theory row.

### L2-09 Zero allocation and thread safety (B1, B2)

- `crates/nepdate/tests/no_alloc.rs` with `[[test]] name = "no_alloc" harness = false` in
  `crates/nepdate/Cargo.toml` (one thread, so nothing else allocates while it counts). A
  `#[global_allocator]` wraps `std::alloc::System` and adds every `alloc`/`realloc` size to an
  `AtomicUsize`. `main` builds the 64 dates of BENCHMARK §2 first, reads the counter, runs B1
  (`NepaliDate::new(..)?.to_gregorian()`) and B2 (`NepaliDate::from_gregorian(..)`) 1,000 times
  over them through `std::hint::black_box`, reads the counter again, and exits 1 if it moved.
  L3, L4 and L5 add their B-operations to this file (and to the patro crate's copy).
- `crates/nepdate/tests/send_sync.rs`: `fn assert_send_sync<T: Send + Sync>() {}` called for every
  public type of the crate; plus 8 `std::thread::scope` threads each converting all 109,212 serials
  and comparing with the single-threaded result.

AC: `cargo test -p nepdate --test no_alloc` passes; adding a `String` allocation to
`to_gregorian` on a scratch branch makes it fail.

## Definition of Done
Global DoD + milestone M1 exit (all test files above green).

## Human review checklist
- [ ] Put ALGORITHM §4–5 next to `tests/exhaustive.rs` `mod reference`: do the lines match one-to-one?
- [ ] Run `cargo test --release --test exhaustive -- --nocapture`. How long does it take? (well under 1 s in release, a few s in debug)
- [ ] In `from_serial`, why is one correction step enough? Read the `month_estimate_is_off_by_at_most_one` test.

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes; the summary line is in the PR.
- [ ] `from_serial`, `to_serial`, `to_gregorian` and `from_gregorian` contain no loop and no
      binary search (ADR-0004 §4).
- [ ] `tests/vectors.rs` asserts 7176 rows; `tests/exhaustive.rs` compares with `mod reference`
      for all 109,212 serials.
- [ ] `month_estimate_is_off_by_at_most_one` exists and passes.
- [ ] `cargo xtask catalog-check 1` exits 0, or the PR says H5 is not synced yet.
- [ ] `cargo test -p nepdate --test no_alloc` covers B1 and B2 and passes.
- [ ] `send_sync.rs` lists every public type in `docs/API.md` so far.
- [ ] No `unwrap`, `expect`, `panic!` or unchecked index without a comment in `src/`.
