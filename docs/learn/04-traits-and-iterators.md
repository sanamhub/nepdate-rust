# Learn 04: Traits and iterators (phase L3)

## 1. Traits ≈ interfaces (with superpowers)

```rust
pub trait Tray { fn set_title(&self, text: &str); }
impl Tray for SlintTray { fn set_title(&self, text: &str) { /* ... */ } }
```

Differences from C#:
- You can implement *your* trait for *someone else's* type, or someone else's trait
  (`Display`, `Iterator`, `From`) for your type. You can't do both at once (the "orphan
  rule").
- Traits can have default method bodies (like C# default interface methods).
- Generic functions use traits as constraints: `fn f<T: Display>(x: T)` ≈ `where T : IFoo`.

## 2. Conversion traits

`From<A> for B` (infallible) and `TryFrom<A> for B` (returns `Result`). Implementing
`From<NepaliDate> for GregorianDate` gives you `.into()` for free.

## 3. `Iterator`: implement one method, get 70 for free

```rust
pub struct Days { front: u32, back: u32, done: bool }
impl Iterator for Days {
    type Item = NepaliDate;
    fn next(&mut self) -> Option<NepaliDate> { /* return Some(day) or None */ }
}
```

Only `next` is required. `map`, `filter`, `take`, `count`, `collect`, `rev` (with
`DoubleEndedIterator`) and many more come from default methods. Iterators are **lazy**
like LINQ, and compile to plain loops (no allocation, no virtual calls).

```rust
let saturdays = range.days().filter(|d| d.weekday() == Weekday::Saturday).count();
```

## 4. Marker-ish iterator traits

- `DoubleEndedIterator` → `next_back`, enables `.rev()`.
- `ExactSizeIterator` → `len()` is known.
- `FusedIterator` → promises `None` forever after the first `None`.

## 5. Property tests (`proptest`)

Instead of a few hand-picked examples, state a law and let the tool generate hundreds of
random inputs:

```rust
proptest! {
    #[test]
    fn add_days_roundtrip(s in 0i32..=109_211, n in -5000i32..5000) {
        let d = NepaliDate::MIN.checked_add_days(s).unwrap();   // any date in range
        if let Ok(e) = d.checked_add_days(n) { prop_assert_eq!(e.checked_add_days(-n), Ok(d)); }
    }
}
```

When a case fails, proptest shrinks it to the smallest failing input.
