# ADR-0013: Licensing of this repository

- Status: Accepted
- Date: 2026-09-24 (accepted 2026-09-25)
- Deciders: Sanam Pakuwal

## Context

- NepDate data and algorithm are MIT. The MIT notice must be kept.
- NepDateWidget is used with the author's permission (ADR-0008, `docs/legal/README.md`).
- Slint is triple-licensed: **GPL-3.0-only**, **Slint Royalty-free 2.0** (free for
  desktop apps; requires visible attribution, e.g. the `AboutSlint` widget in the About
  section), or commercial.
- The hub `sanamhub/nepdate` and the later Go, Java, TypeScript and Python ports use MIT too.
  One license across the family keeps copying between them simple.
- The maintainer's other open-source project (`sanamhub/ada-csharp`) uses plain MIT.

## Decision

| Part | License | Notes |
|---|---|---|
| Whole repository (library, widget source, `xtask`, docs; `shared/` is the hub's MIT content) | **MIT** | One `LICENSE` file at the root, `Copyright (c) nepdate contributors`. Each published package also ships a copy |
| NepDate notice | MIT | kept in `shared/data/NOTICE`, `crates/nepdate/LICENSE-THIRD-PARTY` and `crates/nepdate-patro/LICENSE-THIRD-PARTY` (both crates ship NepDate data) |
| Widget **binary distribution** | uses Slint under **Royalty-free 2.0** | Settings → About shows `AboutSlint`; `cargo-deny` allows `LicenseRef-Slint-Royalty-free-2.0` for Slint crates only |
| Bundled fonts | OFL-1.1 | license files shipped alongside |

Why not `MIT OR Apache-2.0` (the Rust convention): dual licensing adds a second license
file and a contribution clause to every port for no practical gain here. Plain MIT matches
upstream NepDate and is accepted by every registry the ports will use.

## Consequences

- `Cargo.toml` uses `license = "MIT"`.
- Widget releases must keep the Slint attribution visible, or switch to GPL (new ADR).
