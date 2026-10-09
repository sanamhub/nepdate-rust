# Learn 03: `const fn`, integer types, slices, tests (phase L2)

## 1. `const fn`: code that runs at compile time

```rust
static MONTH_INFO: [u32; 3589] = build_month_info();   // computed by the compiler
const fn build_month_info() -> [u32; 3589] { /* while loops only */ }
```

A `const fn` can be evaluated during compilation, so `MONTH_INFO` is baked into the
binary with zero startup cost. The restriction: no trait-based code in const context. `for`
loops use the `Iterator` trait, so they aren't allowed; `while` loops are.
(C# has nothing similar; the closest is a source generator.)

## 2. `const` vs `static`

- `const X: T = ...;` is copied/inlined at every use (like a C# `const`).
- `static X: T = ...;` is one fixed memory location (like `static readonly`).
- Large tables used at runtime → `static`. Tables that `const fn`s must read → `const`.

## 3. Integer types and casts

`u8, u16, u32, u64` (unsigned) and `i8…i64` (signed). No implicit widening:
`let a: u32 = my_u16.into();` (lossless) or `my_i32 as u32` (**unchecked**: wraps or
truncates silently). Prefer `u32::from(x)` / `u8::try_from(x)?`. In debug builds,
overflow panics (`255u8 + 1`); in release it wraps. That's why arithmetic uses
`checked_add` or a wider type (`i64`).

## 4. Arrays and slices

`[u32; 300]` is a fixed-size array (on the stack or in the binary). `&[u32]` is a
**slice**, a view (pointer + length) into an array: `&MONTH_INFO[..12]`.
`slice.partition_point(|&x| x <= s)` does a binary search for the first element where
the predicate becomes false. `from_serial` doesn't need one: it estimates the month index with
one multiplication and corrects it by at most one (ADR-0004 §4).
`arr[i]` panics if out of range; `arr.get(i)` returns `Option`.

## 5. Three kinds of tests

| Kind | Where | Run |
|---|---|---|
| Unit | `#[cfg(test)] mod tests { #[test] fn x() {...} }` in the same file; can test private fns | `cargo test` |
| Integration | `crates/nepdate/tests/*.rs`; sees only the public API, like a user | `cargo test` |
| Doc tests | code blocks in `///` comments | `cargo test --doc` |

```rust
/// ```
/// let d = nepdate::NepaliDate::new(2081, 1, 1)?;
/// assert_eq!(d.to_gregorian().to_string(), "2024-04-13");
/// # Ok::<(), nepdate::Error>(())
/// ```
```

Lines starting with `# ` are compiled but hidden in the rendered docs.
`cargo nextest run` is a faster runner for unit + integration tests (doc tests still
need `cargo test --doc`).
