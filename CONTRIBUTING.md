# Contributing

Bug reports, fixes and documentation improvements are welcome. For a new feature or an API change,
open an issue first so the design can be agreed before you write code.

This project follows the [Code of Conduct](CODE_OF_CONDUCT.md). Report security problems as
described in [SECURITY.md](SECURITY.md), not in a public issue.

## Scope

The calendar data, the specification and the conformance vectors live in the hub,
[`sanamhub/nepdate`](https://github.com/sanamhub/nepdate), and are copied into `shared/` read-only.
A wrong date, holiday or tithi is a hub issue, not a fix here. Never edit `shared/` by hand.

The libraries keep to the API in [`docs/API.md`](docs/API.md) and the decisions in
[`docs/adr/`](docs/adr/README.md). MitiBar's scope is fixed by
[ADR-0008](docs/adr/0008-widget-scope.md) and
[`docs/design/widget-requirements.md`](docs/design/widget-requirements.md); a feature outside that
list needs an ADR first.

## Setup

Rust 1.98 is pinned in [`rust-toolchain.toml`](rust-toolchain.toml); rustup installs it and the
needed components and targets on first use. Install nextest, which `cargo xtask ci` runs:

```bash
cargo install cargo-nextest --locked
cargo xtask ci
```

The last line reads `xtask ci: all 24 steps passed`. `cargo-deny` is optional locally; the CI step
is skipped with a notice when it is missing.

On Linux, building MitiBar also needs `libxkbcommon-dev`, `libwayland-dev`, `libfontconfig1-dev`
and `libdbus-1-dev`.

Useful commands:

```bash
cargo xtask codegen          # regenerate the calendar and patro tables
cargo xtask codegen --check  # fail if the generated files are stale
cargo xtask renderer-check   # fail if the app pulls in a GPU renderer
cargo xtask widget-idle      # measure MitiBar's idle footprint (Windows)
```

## Pull requests

- Keep a pull request to one change.
- Add or update a test for any behaviour change. The shared vector suites must keep passing.
- A public API change updates [`docs/API.md`](docs/API.md) in the same pull request.
- Add a line to the `Unreleased` section of the affected changelog for anything a user would
  notice.
- A significant design decision gets an ADR in [`docs/adr/`](docs/adr/README.md). Accepted ADRs
  are not edited; a later ADR supersedes an earlier one.
- CI must pass on Windows, Linux and macOS.

## Rules the build enforces

- `nepdate` and `nepdate-patro` are `#![no_std]`, `#![forbid(unsafe_code)]`, and allocate nothing.
- `nepdate` never depends on `nepdate-patro` and contains no patro data.
- Generated files in `crates/*/src/generated/` come from `cargo xtask codegen`. Change the
  generator, not the output.
- A parity test may skip a C# row only for a deviation listed in `shared/spec/PARITY.md`, with a
  comment naming it.
- MitiBar uses Slint's software renderer only; `cargo xtask renderer-check` fails on a GPU backend.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/): `type(scope): summary`, imperative,
lower case, no trailing period. Types: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `build`,
`ci`, `perf`. The body says why, not what.

```
fix(parse): reject a day of 00

A day without a leading digit was read as an empty group.
```

## Releasing

Library releases are described in
[`docs/runbooks/release-library.md`](docs/runbooks/release-library.md); MitiBar releases in
[`docs/runbooks/release-mitibar.md`](docs/runbooks/release-mitibar.md). Do not tag or publish
without the maintainer.
