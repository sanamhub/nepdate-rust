# Learn 00: Toolchain setup and the review workflow

Audience: you (C#/.NET developer, new to Rust). Read this before phase L0.
Each later `docs/learn/NN-*.md` file matches one plan phase: read it **before**
reviewing the agent's PR for that phase.

## 1. Install (Windows)

```bash
winget install Rustlang.Rustup
```

Then in a new terminal:

```bash
rustup default stable
```

```bash
rustup component add clippy rustfmt rust-analyzer
```

```bash
cargo install cargo-nextest cargo-llvm-cov cargo-deny --locked
```

Windows needs the MSVC linker: install "Visual Studio Build Tools" with the
"Desktop development with C++" workload if `cargo build` complains about `link.exe`.

Editor: VS Code + the **rust-analyzer** extension (or RustRover). rust-analyzer gives
inline types: turn on "inlay hints"; they teach you what the compiler infers.

## 2. The five commands you will use daily

| Command | C# equivalent | What it does |
|---|---|---|
| `cargo build` | `dotnet build` | compile |
| `cargo test` / `cargo nextest run` | `dotnet test` | run unit + integration + doc tests |
| `cargo clippy --all-targets -- -D warnings` | Roslyn analyzers as errors | lint |
| `cargo fmt` | `dotnet format` | format |
| `cargo doc --open` | DocFX | build API docs, including your doc comments |

## 3. Vocabulary map (C# → Rust)

| C# | Rust | Note |
|---|---|---|
| solution (`.sln`) | workspace (`[workspace]` in root `Cargo.toml`) | |
| project (`.csproj`) | crate / package (`Cargo.toml`) | lib crate = `src/lib.rs`, bin = `src/main.rs` |
| NuGet | crates.io | `cargo add serde` ≈ `dotnet add package` |
| `namespace` | `mod` (module) | a module = a file or folder |
| `public` / `internal` / `private` | `pub` / `pub(crate)` / (default private) | private is the default in Rust |
| `struct` (value type) | `struct` + `#[derive(Clone, Copy)]` | Rust structs *move* by default; `Copy` makes them copy like C# structs |
| `interface` | `trait` | traits can also be implemented for types you don't own |
| `IComparable`, `IEquatable` | `#[derive(PartialOrd, Ord, PartialEq, Eq)]` | derived automatically from fields |
| `ToString()` override | `impl Display` | `format!("{date}")` then works |
| `Parse`/`TryParse` | `impl FromStr` → `"2081-01-01".parse::<NepaliDate>()` | always returns `Result` |
| exceptions | `Result<T, E>` + the `?` operator | errors are values you must handle |
| `null` | `Option<T>` (`Some(x)` / `None`) | no nulls in safe Rust |
| `IEnumerable<T>` + LINQ | `Iterator` + adapters (`map`, `filter`, `collect`) | lazy, zero-cost |
| `const` / `static readonly` | `const` / `static`; `const fn` runs at compile time | |
| `[Conditional]` / `#if` | Cargo features + `#[cfg(feature = "x")]` | |
| xUnit `[Fact]` | `#[test]` in a `#[cfg(test)] mod tests` | tests live next to code |
| `/// <summary>` XML docs | `///` Markdown doc comments; code blocks run as tests | "doctests" |

## 4. How to review an agent PR (checklist you'll reuse every phase)

1. Read the phase file in `docs/plan/`: what were the acceptance criteria (AC)?
2. Run locally: `cargo xtask ci` (runs fmt check, clippy, tests, codegen drift check).
3. Open the diff. For each new `pub` item ask:
   - Does it have a doc comment with an example? (ADR-0007)
   - Could it panic? Search the diff for `unwrap(`, `expect(`, `[` indexing, `as ` casts.
   - Is it in the plan? If not → scope creep; ask the agent to remove it.
4. For anything you don't understand, ask the agent: *"Explain line N like I know C#
   but not Rust"*. Add useful answers to `docs/learn/NOTES.md` (your own notes file).
5. Run `cargo doc --open` and read the public API the way a user would.

## 5. Optional hands-on practice

[Rustlings](https://github.com/rust-lang/rustlings) (small fix-the-code exercises) pairs well
with the Book. Doing the exercises for a chapter before reviewing that phase's PR is plenty.

## 6. Reading order for the Rust Book (free, https://doc.rust-lang.org/book/)

You don't need all of it up front. Minimum per phase:

| Before phase | Book chapters |
|---|---|
| L0 workspace | 1 (Getting Started), 14.3 (Workspaces) |
| L1 core types | 3 (Common Concepts), 5 (Structs), 6 (Enums & match), 9 (Error Handling) |
| L2 conversion | 8 (Collections: arrays/slices only), 11 (Testing) |
| L3 arithmetic/ranges | 10.2 (Traits), 13 (Iterators and Closures) |
| L4 formatting/parsing | 8.2 (Strings), 10.3 (Lifetimes: just the basics) |
| L5 features | 7 (Modules), plus the Cargo book chapter "Features" |
| W0+ widget | 15 (Smart pointers: `Box`, `Rc`, `RefCell`), 16 (Concurrency: threads & channels) |
