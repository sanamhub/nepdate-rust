# ADR-0003: Shared data and spec come from the hub, vendored in `shared/`

- Status: Accepted
- Date: 2026-09-24 (revised 2026-09-25: data and spec moved to the hub `sanamhub/nepdate`)
- Deciders: Sanam Pakuwal

## Context

The calendar data, the normative spec, the conformance vectors and the C# deviations are the
same for every NepDate port. They live in the hub repository `sanamhub/nepdate`; each language
has its own repository (hub ADR-0001, data format in hub ADR-0002, parity in hub ADR-0003).
This repository is the Rust spoke: the `nepdate` and `nepdate-patro` crates and the widget.

## Decision

1. `shared/` is a **read-only copy** of the hub's `data/` and `spec/`. `shared/VERSION` holds
   the hub commit it was copied from.
2. Nobody edits `shared/` by hand. CI job `shared-check` (the hub's reusable workflow)
   re-downloads the hub at `VERSION` and fails on any difference. A data or spec fix is a PR to
   the hub.
3. `shared-sync.yml` (the hub's reusable workflow, task L6-06) runs every Tuesday: it copies the
   hub's `main` into `shared/`, runs `cargo xtask codegen`, and opens or updates one PR.
4. Rust reads `shared/` only at dev time: `cargo xtask codegen` turns `shared/data/` into
   committed Rust tables (ADR-0006) and the tests read `shared/spec/vectors/`. The published
   crates contain neither.
5. `docs/API.md` (the Rust public API) stays here; it is not shared.

## Consequences

- Contributors clone normally and tests run offline. No submodule.
- A data update reaches Rust users after the weekly sync PR is merged and a patch release is
  cut.
- The widget release gate (`cargo xtask data-status`, W4-00) checks the data `nepdate-patro` ships,
  not the hub's current state.
