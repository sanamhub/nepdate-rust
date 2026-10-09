# Learn 06: Cargo features, `cfg`, `no_std`, a second crate (phase L5)

## 1. `#[cfg(...)]` removes code at compile time

```rust
#[cfg(feature = "jiff")]
pub fn today() -> Result<NepaliDate, Error> { /* ... */ }   // doesn't exist without the feature

#[cfg(feature = "std")]
impl std::error::Error for Error {}
```

Like `#if` in C#, except the conditions come from Cargo features or the target
(`#[cfg(target_os = "windows")]`, `#[cfg(test)]`).

## 2. Features must be additive

Turning a feature **on** may only add API, never remove or change it. Cargo unifies
features: if two crates in a build ask for `nepdate` with different features, it's compiled
once with the union. So "feature X disables Y" designs break users.

## 3. Optional dependencies

```toml
[dependencies]
jiff = { version = "0.2", optional = true }
[features]
jiff = ["dep:jiff", "std"]       # enabling our "jiff" feature pulls in the crate + std
```

`cargo tree -p nepdate --no-default-features` proves the core has zero dependencies.

## 4. `no_std`

`#![no_std]` means only `core` is available: no heap (`Vec`, `String`), no files, no clock,
no threads. We keep the core `no_std` so it works in WASM (future JS/TS bindings) and
embedded. `extern crate std;` under `#[cfg(feature = "std")]` brings `std` back when
enabled.

## 5. Checking combinations

CI builds each important feature combination explicitly (L5-06). A classic bug is code
that compiles only because some *other* feature happened to be on.

## 6. Why the patro is a crate, not a feature

A feature removes code from the *build*, but not from the *download*: every `nepdate` user
would still fetch about 100 KB of event names. A separate crate `nepdate-patro` (≈ a second
NuGet package) is downloaded only by those who add it. It depends on `nepdate`, never the
other way round (ADR-0005, hub ADR-0004).

```toml
# crates/nepdate-patro/Cargo.toml
[dependencies]
nepdate = { path = "../nepdate", version = "~0.1.0", default-features = false }
```

`path` is used inside this repository; `version` is what crates.io users get. `~0.1.0` means
"0.1.x only", so the two crates always come from the same release.
