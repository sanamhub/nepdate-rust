# ADR-0021: Release tags are `vX.Y.Z`, without a package prefix

- **Status:** accepted
- **Date:** 2026-10-29
- **Approved by:** Sanam
- **Supersedes:** the tag-prefix rule in ADR-0002

## Context

`nepdate-rust` used `nepdate-vX.Y.Z` tags. The other language repositories use `vX.Y.Z`:
`nepdate-go`, `nepdate-python`, `nepdate-typescript`, `nepdate-kotlin`, `nepdate-c` and
`nepdate-cpp` all released `v0.1.0-beta.1`. MitiBar has moved to its own repository, so the
prefix no longer separates two release trains here. It only makes links, tooling and the family's release
pages inconsistent.

The maintainer asked for plain `v` tags across the family.

## Decision

1. `nepdate-rust` release tags are **`vX.Y.Z`** and **`vX.Y.Z-pre.N`**. They release both
   `nepdate` and `nepdate-patro` at that version, as before.
2. The `nepdate-v0.1.0-beta.1` tag is replaced by `v0.1.0-beta.1` on the same commit, `1277a5c`,
   which is the commit crates.io records for both published crates. Nothing is republished.
3. `release-lib.yml`, the library runbook and `cargo xtask changelog` use `vX.Y.Z`.
4. Every language repository uses the same `vX.Y.Z` shape. Swift is the one exception today
   (`0.1.0-beta.1` without the `v`); it should adopt the same prefix at its next release.

## Consequences

- Release URLs, changelog links and `gh release` commands are the same in every language repo.
- A tag no longer says which package it belongs to. For this repository that is unambiguous: one
  tag releases both crates.
- A link to the old `nepdate-v0.1.0-beta.1` tag no longer resolves; the same commit is
  `v0.1.0-beta.1`.
