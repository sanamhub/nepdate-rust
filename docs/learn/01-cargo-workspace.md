# Learn 01: Cargo, crates and workspaces (phase L0)

## 1. Package, crate, workspace

- **Package** = a folder with a `Cargo.toml` (≈ a `.csproj`).
- **Crate** = one compilation unit inside a package: a *library crate* (`src/lib.rs`)
  and/or *binary crates* (`src/main.rs`, `src/bin/*.rs`). A package has at most one lib.
- **Workspace** = several packages sharing one `Cargo.lock` and one `target/` folder
  (≈ a `.sln`, but builds are shared and faster).

Our workspace: `crates/nepdate` (lib), `xtask` (bin, dev tool), later `crates/nepdate-patro`
(lib, depends on `nepdate`) and `apps/mitibar` (bin).

## 2. Anatomy of `Cargo.toml`

```toml
[package]
name = "nepdate"          # crate name users write in their Cargo.toml
version = "0.1.0"         # semver: MAJOR.MINOR.PATCH
edition = "2024"          # language edition (like LangVersion); opt-in syntax changes

[features]
default = ["std"]         # features on by default
std = []                  # a feature that enables no extra crates

[dependencies]
serde = { version = "1", optional = true, default-features = false }

[dev-dependencies]        # only for tests/benches/examples (like test-project refs)
proptest = "1"
```

`version = "1"` means "any 1.x.y ≥ 1.0.0" (caret requirement). `Cargo.lock` pins the
exact versions. Commit it for apps.

## 3. Inheritance: `[workspace.package]` and `[workspace.lints]`

Settings written once in the root and pulled in by members:

```toml
# root Cargo.toml
[workspace.package]
edition = "2024"
[workspace.lints.clippy]
unwrap_used = "warn"

# crates/nepdate/Cargo.toml
[package]
edition.workspace = true
[lints]
workspace = true
```

This is like `Directory.Build.props` in .NET. `clippy` is Rust's analyzer. With
`-D warnings` in CI, every warning is an error (org rule: lint errors block merge).

## 4. The `xtask` pattern

Instead of shell/PowerShell scripts (which differ per OS), dev automation is a normal
Rust binary in the workspace. `.cargo/config.toml` adds an alias so
`cargo xtask ci` = `cargo run --package xtask -- ci`. It's cross-platform and type-checked.

## 5. Things that surprise C# developers

- There's no global package cache like `~/.nuget` to manage by hand. Cargo handles it
  (`~/.cargo/registry`).
- Builds are per-profile: `target/debug` vs `target/release`. Release builds with LTO
  are slow to compile but fast to run.
- `cargo build` compiles dependencies from source the first time (slow), then caches them.
