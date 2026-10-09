# ADR-0005: `no_std`, zero-dependency core with opt-in Cargo features

- Status: Accepted
- Date: 2026-09-24 (revised 2026-10-07: the `meta` feature became the crate `nepdate-patro`,
  hub ADR-0004)
- Deciders: Sanam Pakuwal

## Context

The C# library bundles conversion, formatting, parsing, "smart" parsing, fiscal years,
ranges, bulk conversion, JSON/XML serializers, a TypeConverter and holiday/tithi/event
data into one package. In Rust the idiomatic approach is a small core plus **Cargo
features** so users only compile what they need. A `no_std` core also makes WASM
(future JS/TS bindings) and embedded use trivial.

## Decision

`crates/nepdate` is `#![no_std]` and **never allocates**: formatting returns small `Copy`
values that implement `Display` and have a generic `write_to(&mut impl fmt::Write)`
(ADR-0004 §5), parsing works on `&str`, ranges are iterators. So no `alloc` feature is needed. Anything needing the OS or a dependency is
behind a feature.

| Feature | Default | Enables | Extra deps |
|---|---|---|---|
| *(none)* | – | `NepaliDate`, conversion, arithmetic, weekday, month/weekday enums, `Error`, formatting into any `core::fmt::Write`, strict parsing via `FromStr` | none |
| `std` | **yes** | `std::error::Error` impl | none |
| `serde` | no | `Serialize`/`Deserialize` as `"YYYY-MM-DD"` string | `serde` (no default features) |
| `chrono` | no | `From`/`TryFrom` between `NepaliDate` and `chrono::NaiveDate` | `chrono` (no default features) |
| `jiff` | no | `From`/`TryFrom` between `NepaliDate` and `jiff::civil::Date`; `NepaliDate::today()` and `is_today()` (local date, ADR-0011) | `jiff` (implies `std`) |

Tithi, public holiday and patro data (BS 2077–2084) is **not** a feature of `nepdate`. It is
the separate crate `crates/nepdate-patro` (package `nepdate-patro`, lib `nepdate_patro`), as
in every NepDate port (hub ADR-0004). It is `#![no_std]`, never allocates, has no features, and
its only dependency is `nepdate` with `default-features = false`. The core never depends on it,
and the compiled core contains no event string (CI checks for `World Day Against Human Trafficking`, task L5-06).

Rules:

- The core must build with `cargo build -p nepdate --no-default-features`
  (CI job enforces this, plus a `wasm32-unknown-unknown` build). `nepdate-patro` builds for
  `wasm32-unknown-unknown` too.
- No `unsafe` anywhere in the library: `#![forbid(unsafe_code)]`.
- No proc-macro dependencies in the core.
- Adding a new dependency or feature requires a new ADR.

Deliberately **not** ported (YAGNI, may be added later with an ADR):

- XML serialization, `TypeConverter`, Newtonsoft converters (C#-ecosystem specific).
- `SmartDateParser` fuzzy heuristics beyond the documented formats in `shared/spec/PARSING.md`
  (keeps parsing deterministic and identical across languages).
- `BulkConvert` parallel APIs (in Rust, users can `iter().map(...)` or use `rayon`
  themselves; we document the pattern instead).

## Consequences

- Beginner-friendly: the core has no generics-heavy dependencies to understand.
- A user who only converts dates downloads and compiles no event strings (about 100 KB).
- Patro data changes every year. Each such update is a patch release of both crates
  (ADR-0002).
- `today()` lives behind `jiff` because neither `core` nor `std` can read the local UTC offset (ADR-0011).
- Users who want a `String` call `.to_string()` on the returned formatter value (needs `std`/`alloc` on their side, not ours).
