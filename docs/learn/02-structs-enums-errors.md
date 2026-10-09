# Learn 02: Structs, enums, `Option`, `Result` (phase L1)

## 1. Structs and `derive`

```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NepaliDate { year: u16, month: u8, day: u8 }
```

- Fields are **private** by default (no `pub`), so outside code can't build an invalid date.
- `derive` asks the compiler to write trait impls: `Copy` (value semantics like a C#
  struct), `PartialEq/Eq` (`==`), `PartialOrd/Ord` (`<`, sorting; compares fields **in
  declaration order**, which is why `year` comes first), `Hash` (use as a `HashMap` key).
- Methods live in `impl NepaliDate { ... }`. `self` (by value) is fine because it's `Copy`
  and only 4 bytes.

## 2. Enums are much stronger than C# enums

```rust
pub enum Error {
    OutOfRange,                                   // no data
    InvalidMonth(u8),                             // carries a value
    InvalidDay { month_length: u8, day: u8 },     // carries named fields
}
match err {
    Error::OutOfRange => println!("range"),
    Error::InvalidMonth(m) => println!("month {m}"),
    Error::InvalidDay { day, .. } => println!("day {day}"),
}
```

`match` must be **exhaustive**: forget a variant and it won't compile.
`#[non_exhaustive]` on a public enum forces *users* to add a `_ =>` arm, so we can add
variants later without breaking them.

## 3. `Option<T>` and `Result<T, E>` (no null, no exceptions)

```rust
fn succ(self) -> Option<NepaliDate>          // Some(next) or None at MAX
fn new(y: u16, m: u8, d: u8) -> Result<NepaliDate, Error>
```

Both are ordinary enums: `enum Option<T> { Some(T), None }`,
`enum Result<T, E> { Ok(T), Err(E) }`.

## 4. Why `Result` instead of panic (ADR-0007)

A **panic** (`unwrap()` on `Err`, `panic!`, array index out of bounds) is like an
unhandled exception: it unwinds or aborts the thread. Libraries must not panic on bad
input. The caller decides what to do, so we return `Result`. Clippy lints
(`unwrap_used`, `indexing_slicing`) flag hidden panics in review.

## 5. The `?` operator

```rust
fn next_new_year(d: NepaliDate) -> Result<NepaliDate, Error> {
    let next = NepaliDate::new(d.year() + 1, 1, 1)?;  // on Err: return Err early
    Ok(next)
}
```

`?` = "if error, return it now; else unwrap the value". It's the Rust version of letting
an exception propagate, except that it shows up in the function signature.

## 6. Modules and visibility

`mod date;` in `lib.rs` loads `src/date.rs`. `pub` = public API; `pub(crate)` = C#
`internal`; nothing = private to the module. `pub use crate::date::NepaliDate;` in `lib.rs`
re-exports it so users write `nepdate::NepaliDate` instead of `nepdate::date::NepaliDate`.
