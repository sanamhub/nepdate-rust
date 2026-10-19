# ADR-0001: Record architecture decisions

- **Status:** accepted
- **Date:** 2026-09-24
- **Approved by:** Sanam

## Context

This repository is a Rust port with decisions that contributors and other ports need to
share. Decisions are written down so that:

- contributors do not re-litigate or silently reverse them,
- a reader can understand *why* code looks the way it does,
- future ports (Go, JS/TS, Python, ...) can reuse the reasoning.

Every major decision is recorded as an ADR in `/docs/adr`.

## Decision

- Use lightweight MADR-style ADRs in `docs/adr/NNNN-kebab-title.md`.
- Sections: Status, Date, Approved by, Context, Decision, Consequences, (Alternatives).
- ADRs may be revised in place until the repository is published. From then on, ADRs are
  **immutable** once accepted. To change a decision, write a new ADR that says
  `Supersedes ADR-NNNN` and set the old one's Status to `Superseded by ADR-MMMM` (the status
  line is the only edit allowed on an accepted ADR).
- Read every accepted ADR before starting work.
- A contributor who believes an ADR is wrong must stop and raise it with the maintainer,
  not work around it.

## Consequences

- Slight overhead per decision; large payoff in consistency across contributors and ports.
- `docs/adr/README.md` keeps the index; update it when adding an ADR.
