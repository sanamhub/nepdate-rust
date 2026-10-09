# ADR-0001: Record architecture decisions

- Status: Accepted
- Date: 2026-09-24
- Deciders: Sanam Pakuwal

## Context

This repository will be implemented largely by AI coding agents and reviewed by a
developer who is learning Rust. Decisions must be written down so that:

- agents do not re-litigate or silently reverse them,
- the reviewer can understand *why* code looks the way it does,
- future ports (Go, JS/TS, Python, ...) can reuse the reasoning.

Org policy (SEA §2, §8) requires major decisions as ADRs, stored immutably in `/docs/adr`.

## Decision

- Use lightweight MADR-style ADRs in `docs/adr/NNNN-kebab-title.md`.
- Sections: Status, Date, Deciders, Context, Decision, Consequences, (Alternatives).
- Before phase L0 is merged (planning), ADRs may still be revised in place during plan review.
  From then on, ADRs are **immutable** once Accepted. To change a decision, write a new ADR that
  says `Supersedes ADR-NNNN` and set the old one's Status to `Superseded by ADR-MMMM`
  (the status line is the only edit allowed on an accepted ADR).
- Agents MUST read every Accepted ADR before starting a phase (see `AGENTS.md`).
- An agent that believes an ADR is wrong must stop and raise it with the human,
  not work around it.

## Consequences

- Slight overhead per decision; large payoff in consistency across agents and ports.
- `docs/adr/README.md` keeps the index; update it when adding an ADR.
