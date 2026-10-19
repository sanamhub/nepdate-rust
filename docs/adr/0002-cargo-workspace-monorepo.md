# ADR-0002: One Rust repository and Cargo workspace for library and widget

- **Status:** accepted
- **Date:** 2026-09-24 (revised 2026-09-25: Rust only; shared data and spec vendored from the hub;
  revised 2026-10-07: patro crate `nepdate-patro`, hub ADR-0004)
- **Approved by:** Sanam

## Context

We are building three Rust deliverables:

1. `nepdate`, a library port of the C# NepDate library (Bikram Sambat calendar).
2. `nepdate-patro`, a library with tithi, public holidays and events, built on `nepdate`
   (hub ADR-0004).
3. MitiBar, a minimal cross-platform desktop widget using both libraries.

Other languages get their own repositories; what they share lives in the hub
`sanamhub/nepdate` (hub ADR-0001).

## Decision

This repository is **`sanamhub/nepdate-rust`**, with one Cargo workspace at the root:

```
nepdate-rust/
├── Cargo.toml              # [workspace] root, shared lints + dependency versions
├── LICENSE                 # MIT (ADR-0013)
├── shared/                 # read-only copy of the hub's data/ and spec/ + VERSION   ADR-0003
├── crates/
│   ├── nepdate/            # the core library (no_std), own CHANGELOG.md          ADR-0005
│   └── nepdate-patro/     # tithi, holidays, events; depends on nepdate          ADR-0005
├── apps/
│   └── mitibar/            # the desktop app, own CHANGELOG.md                    ADR-0008
├── xtask/                  # dev automation: codegen, checks, release helpers     ADR-0006
├── benches/nepdate-bench/  # criterion + competitor crates, publish = false        ADR-0014
└── docs/                   # API.md, adr/, design/, runbooks/, legal/
```

- Workspace-level `[workspace.package]` (edition, license, rust-version) and
  `[workspace.lints]` are inherited by every member.
- `Cargo.lock` is committed (the workspace contains a binary).
- Each package has its own `CHANGELOG.md`. Two release trains, two tag prefixes:
  - `nepdate-vX.Y.Z` releases **both** library crates, `nepdate` and `nepdate-patro`, with the
    same version X.Y.Z (hub ADR-0004 §3). `nepdate-patro` depends on
    `nepdate = { path = "../nepdate", version = "~X.Y.0", default-features = false }`, so the
    pair always comes from the same minor. A change to only one of them still bumps both.
  - `mitibar-vX.Y.Z` releases the app, versioned on its own (ADR-0017).

## Consequences

- One CI pipeline. One PR can change library and app together.
- MitiBar always builds against the in-tree libraries (`path` dependencies), so
  library API breaks are caught immediately.
- When publishing to crates.io, `nepdate` must not depend on anything outside `crates/nepdate`
  at build time, and `nepdate-patro` on nothing outside `crates/nepdate-patro` except the
  published `nepdate`. That is why generated code is checked in (ADR-0006).
- `nepdate` never depends on `nepdate-patro`, not even as a dev-dependency, so
  `cargo test -p nepdate` builds without any patro data.

## Alternatives considered

- Separate repositories for the libraries and the app: more ceremony, cross-repo version
  bumps for every change while the API is still moving. Rejected.
- Library-only workspace with the app elsewhere: same problem.
