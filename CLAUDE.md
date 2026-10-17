# CLAUDE.md

All agent instructions live in [AGENTS.md](AGENTS.md). Read it first, then
`docs/plan/00-ROADMAP.md`. Load the `writing-style` skill before writing any prose.

Org exceptions to SEA org instructions: none. (Record any exception here with the named
approver, per SEA §14.)

Project-specific notes:
- Libraries = `crates/nepdate` (Rust port of MIT NepDate data and algorithm) and
  `crates/nepdate-patro` (tithi, holidays, events; hub ADR-0004). Other languages
  have their own repositories; shared data and spec come from the hub `sanamhub/nepdate` into
  `shared/`, which is read-only here (ADR-0003).
- Desktop app = `apps/mitibar` (display name MitiBar, identifiers `mitibar`; ADR-0017).
  NepDateWidget is a reference, used with its author's permission; scope is fixed by ADR-0008.
- Org "Clean Architecture" default is applied as module layers inside the widget crate (ADR-0015).
