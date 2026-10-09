# Learn 07: Docs, benchmarks, publishing (phase L6)

## 1. Documentation is part of the code

- `//!` documents the enclosing module/crate; `///` documents the next item.
- `#![deny(missing_docs)]` makes an undocumented `pub` item a compile error.
- docs.rs builds your docs automatically on publish. `[package.metadata.docs.rs]`
  controls features.
- `#[doc = include_str!("../README.md")]` reuses the README as crate docs, and its code
  blocks become doctests, so the README can't go stale.

## 2. Benchmarks with criterion

```rust
fn bench(c: &mut Criterion) {
    c.bench_function("bs_to_ad", |b| b.iter(|| black_box(date).to_gregorian()));
}
```

`black_box` stops the optimiser from deleting work whose result is unused. criterion runs
many iterations and reports mean ± noise. Compare runs on the same machine only.

## 3. Semver in Rust

Before 1.0, `0.MINOR` bumps are breaking (`0.1 → 0.2`) and `0.1.x` patches are compatible.
Adding a variant to a `#[non_exhaustive]` enum isn't breaking. Removing a pub fn, changing a
signature, or adding a required trait method is. `cargo-semver-checks` detects these.

## 4. Publishing

`cargo publish --dry-run` builds the exact package that would be uploaded.
Published versions can never be deleted, only **yanked** (hidden from new resolution). So
the release is gated by a human approval (org §12).
