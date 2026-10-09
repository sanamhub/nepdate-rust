# Learn 05: Strings, `Display`, `FromStr`, lifetimes (phase L4)

## 1. `String` vs `&str`

- `String`: owned, growable, heap-allocated (≈ `StringBuilder` + `string`).
- `&str`: a borrowed view of UTF-8 bytes (≈ `ReadOnlySpan<char>`, but UTF-8).
- Functions **take** `&str` (accept anything), and **return** `String` only when they must
  create new text.

## 2. `impl Display` instead of returning `String`

```rust
pub fn long(self, opts: LongDate, lang: Lang) -> impl Display { LongDisplay { date: self, opts, lang } }

impl fmt::Display for LongDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.date.month().name(self.lang))?;
        write!(f, " {}", self.lang.digits(u32::from(self.date.day()), 2))
    }
}
```

The caller decides where the text goes: `println!("{}", d.long(..))` writes straight to
stdout, and `d.long(..).to_string()` makes a `String` only if they need one. So the library
never allocates and works in `no_std` (ADR-0005).

## 3. UTF-8 and Devanagari

`"साउन".len()` is **12** (bytes), not 4. Each Devanagari code point is 3 bytes in UTF-8.
Iterate with `.chars()` (Unicode scalar values) and never slice strings at arbitrary byte
indices (`&s[0..1]` would panic in the middle of a character). A "letter" as a human sees it
(e.g. कि) can be **several** chars (क + ि). That's why parsing works on whole tokens
and compares complete strings.

## 4. `FromStr`

```rust
impl FromStr for NepaliDate {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> { /* PARSING §1 */ }
}
let d: NepaliDate = "2081/04/15".parse()?;
```

## 5. Lifetimes, just enough

```rust
pub fn format_pattern<'a>(self, pattern: &'a str, lang: Lang) -> impl Display + 'a
```

`'a` says: "the returned value borrows `pattern`, so it can't outlive it". The compiler
checks that the caller keeps the pattern string alive while using the result. Most of the
time lifetimes are inferred. You only write them when a return value borrows from an
argument.
