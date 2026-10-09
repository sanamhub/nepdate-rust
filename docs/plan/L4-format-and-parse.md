# L4: Formatting and parsing

**Goal:** all output formats (English and Nepali) and both parsers, allocation-free,
matching `shared/spec/FORMATTING.md` / `shared/spec/PARSING.md` and the golden vectors.

Read first: `shared/spec/FORMATTING.md`, `shared/spec/PARSING.md`, ADR-0009 D-04, D-05, D-06, D-15.
Human reads: `docs/learn/05-display-fromstr-strings.md`.

## Tasks

### L4-01 Digit localisation (`src/names.rs`)
`Lang::digits(n, min_width) -> Digits` (`docs/API.md`). `Digits::write_to` writes `n` zero-padded
to `min_width`; for `Lang::Ne` each digit is a piece of `static NE_DIGITS: [&str; 10]`
(`"०"` to `"९"`, 3 bytes each), written with `out.write_str`. No `char` encoding at run time.

### L4-02 Formatter values (`src/format.rs`)
The public `Copy + Debug` types `ShortFormat`, `LongFormat`, `PatternFormat<'a>` and
`FiscalLabel` (with `Digits` from L4-01), exactly as in `docs/API.md`. Each has
`pub fn write_to<W: fmt::Write + ?Sized>(&self, out: &mut W) -> fmt::Result`, and `Display::fmt` is
`self.write_to(f)`. `write_to` writes only `&'static str` pieces (names, separators, digits) with
`out.write_str`: no `write!` per digit, no byte buffer plus `str::from_utf8`, no `String`, no
`unsafe` (ADR-0004 §5 and its B5 measurements: 30 ns this way against 120 to 175 ns through
`fmt::Formatter` or `from_utf8`). Implement FORMATTING §3, §4, §5 exactly. The pattern scanner
works on `pattern.as_bytes()`; a byte that is not an ASCII pattern letter starts a literal run that
is written as one `&str` slice of the pattern (slice at the run's byte boundaries, which are always
char boundaries because pattern letters are ASCII).

AC:
- Unit tests for every example in FORMATTING.md.
- For every formatter and a sample of dates: `write_to` into a `String` equals `to_string()`.
- A test in the `--no-default-features` build formats into a hand-written
  `struct Buf { bytes: [u8; 128], len: usize }` implementing `fmt::Write`.

### L4-03 Strict parser: `FromStr` (`src/parse.rs`)
PARSING §1, over `s.as_bytes()`. ASCII digits `b'0'..=b'9'`; a Devanagari digit is the 3-byte
sequence `E0 A5 A6` to `E0 A5 AF` (D-05). Accumulate each group into a `u32` with a digit counter
(more than 4 digits gives `NumberTooLong`). One pass; no `chars()`, no second validation step.

### L4-04 Lenient parser: `NepaliDate::parse_lenient` (`src/parse.rs`)
PARSING §2–3, **without allocation**:
- Tokenise over the bytes into `&str` slices. A token is either digits or a "word".
- Keep at most 3 numeric tokens and 1 month token in fixed-size arrays or `Option`s; more
  tokens return the error at once.
- Month lookup: a `static MONTH_ALIASES: [(&str, u8); N]`. Latin comparison uses
  `eq_ignore_ascii_case`; Devanagari compares exactly.
- Era markers and `गते`/`मिति`: a `static IGNORED_TOKENS: [&str; N]`, also compared with
  `eq_ignore_ascii_case`.

### L4-05 Vector, parity and property tests
- `tests/parse_vectors.rs`: every row of `shared/spec/vectors/parse.tsv`, both columns; error kind
  names must match (`ERR:InvalidDay` ⇔ `Err(Error::InvalidDay{..})`,
  `ERR:Ambiguous` ⇔ `Err(Error::Parse(ParseErrorKind::Ambiguous))`, …).
- `tests/csharp_parity.rs`: `dates.tsv` columns `unicode`, `long_en`, `long_en_weekday`,
  `long_ne`, `long_ne_weekday`, `dmy_dash_nopad`; `format-pattern.tsv`, skipping D-04 rows
  with a filter commented `// D-04 (ADR-0009): MMM for Ashad/Ashoj differs on purpose`.
  `parse.tsv` (C#) `strict` column: must match for all rows **except** Devanagari-digit
  inputs (D-05), with a commented filter.
- Proptests in `tests/props.rs`: `s.parse::<NepaliDate>() == Ok(d)` where `s = d.to_string()`,
  and for `short(Ymd, any sep, any pad, any lang)`; `parse_lenient` of `long(default, En)`
  round-trips.
- Robustness proptest (PORT-PLAN phase 3): for any `String` (proptest `".*"` and
  `"[0-9०-९/ .-]{0,20}"`), `s.parse::<NepaliDate>()` and `NepaliDate::parse_lenient(&s)` return
  `Ok` or `Err(Error)` and never panic. 10,000 cases each.

### L4-06 C# catalogue rows, phase 3
`crates/nepdate/tests/csharp_catalog.rs`: one `cs_` test per catalogue row with `phase` = 3
(`cargo xtask catalog-check 3 --list`; files in `docs/plan/00-ROADMAP.md` "C# test catalogue").
Add `cargo xtask catalog-check 3` to `cargo xtask ci`.
AC: `cargo xtask catalog-check 3` exits 0, or the PR says H5 is not synced yet and lists the rows
it implemented from the C# files.

### L4-07 Zero allocation (B5, B6) and fuzz targets
- Add B5 (`long(LongDate { weekday: true, ..LongDate::default() }, Lang::Ne).write_to(&mut buf)`
  into a reused stack buffer) and B6 (`s.parse::<NepaliDate>()` over the 64 `YYYY/MM/DD` strings)
  to `crates/nepdate/tests/no_alloc.rs` (L2-09). Also `parse_lenient` and `format_pattern`.
- `fuzz/` (cargo-fuzz layout, not a workspace member; add it to `[workspace] exclude`):
  targets `parse_strict` and `parse_lenient` (arbitrary `&str`: never panic; if `Ok(d)`,
  `d.to_string().parse() == Ok(d)`), and `format_pattern` (arbitrary pattern string and serial:
  never panic; `write_to` output equals `to_string()`). Running is L6-02b.

AC: `cargo test -p nepdate --test no_alloc` passes with the new rows; `cargo +nightly fuzz build`
in `fuzz/` succeeds on Linux or WSL (if no nightly is available, say so in the PR; L6-02b runs it).

## Definition of Done
Global DoD + milestone M2 exit.

## Human review checklist
- [ ] Find `LongFormat::write_to`. Why is it better than returning `String`? (learn/05 §2)
- [ ] Run `cargo test -p nepdate --no-default-features`. Did the no-alloc test run?
- [ ] Try `"१५ साउन २०८१".parse_lenient()` in a scratch test. What happens with `"15 saun 81"`, and why? (PARSING §2 step 3)

## Checkpoint

The implementation agent stops here for review. The reviewer checks:

- [ ] `cargo xtask ci` passes; the summary line is in the PR.
- [ ] The formatter types and their signatures equal `docs/API.md`; each `Display::fmt` is one
      call to `write_to`.
- [ ] `src/format.rs` and `src/names.rs` contain no `String`, `format!`, `to_string`, `Vec`,
      `str::from_utf8` or `unsafe`, and no `write!` inside a per-digit loop.
- [ ] `src/parse.rs` reads `as_bytes()`, not `chars()`, and contains no allocation.
- [ ] `tests/parse_vectors.rs` covers every row; the D-04 and D-05 filters carry their comment.
- [ ] `cargo test -p nepdate --test no_alloc` covers B1 to B6 and passes.
- [ ] `cargo xtask catalog-check 3` exits 0, or the PR says H5 is not synced yet.
- [ ] `fuzz/` has the three targets and is excluded from the workspace.
- [ ] `send_sync.rs` lists the new public types.
