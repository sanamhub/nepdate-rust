# Contributing

Thanks for helping. This repository holds the Rust `nepdate` and `nepdate-patro` libraries and the
MitiBar desktop app.

## Before you start

- Open an issue before a new feature or an API change, so we can agree on it first.
- Data and spec problems (a wrong date, holiday or tithi) belong in the hub,
  [`sanamhub/nepdate`](https://github.com/sanamhub/nepdate). `shared/` here is a read-only copy.

## Setup

- Rust: `rust-toolchain.toml` pins the toolchain; rustup installs it on first use.
- Python 3 (`py` on Windows) runs the reference checks in `shared/spec/`.
- Optional: `cargo-nextest` and `cargo-deny`, which `cargo xtask ci` uses.

## Making a change

1. Run `cargo xtask ci` before you open a pull request. It must pass.
2. One change per pull request.
3. Add tests for every behaviour change.
4. Add a changelog line for anything a user would notice (`crates/*/CHANGELOG.md`).
5. Write an ADR in `docs/adr/` for a significant decision.
6. Use [Conventional Commits](https://www.conventionalcommits.org/), for example
   `feat(nepdate): L2-03 serial conversion`.
7. Follow the writing rules in `.claude/skills/writing-style/SKILL.md` for docs, comments and
   commit messages.
