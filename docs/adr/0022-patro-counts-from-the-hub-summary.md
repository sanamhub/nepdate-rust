# ADR-0022: Read the patro counts from the hub's summary, not from literals

- **Status:** accepted
- **Date:** 2026-10-31
- **Approved by:** Sanam
- **Amends:** ADR-0006 (codegen also reads `shared/spec/vectors/patro-summary.json`)

## Context

The patro tables have counts that follow the data: coverage days, tithi days and names, holidays,
event days and events, distinct names and their UTF-8 bytes, plus the coverage and tithi range
edges. The library pinned several of them in compile-time asserts, and the tests pinned all of
them as literals. The hub now generates `spec/vectors/patro-summary.json` from the same data, and
every port vendors it as `shared/spec/vectors/patro-summary.json`. A real data change, such as a
renamed event, changes those counts, so a port that pins them cannot merge the weekly sync pull
request without a person editing numbers by hand.

## Decision

1. `cargo xtask codegen` reads `shared/spec/vectors/patro-summary.json` and checks every
   data-derived count and range edge it generates against it. A mismatch fails with the field and
   both values in the message. A missing summary fails with a message to re-sync `shared/`, so a
   port whose `shared/` predates the file is not silently checked against nothing.
2. The `nepdate-patro` library keeps only the invariants the lookups rely on, such as the name
   tables fitting their index type and the holiday bits covering the coverage. It does not pin a
   count that a data update changes.
3. The tests read the summary at run time through a small helper: `tests/summary.rs` for the
   integration tests and a reader in the `lib.rs` test module. The workspace has no JSON crate and
   these tests do not add one, because the file has a fixed generated shape. The tests still
   compare each generated table with the summary, so dropped or duplicated data fails them.
4. ADR-0004 §6 keeps the table sizes it records as facts of their day.

## Consequences

- A hub data update that passes the hub's own vector check reaches this port without editing a
  number: codegen and the tests follow the summary.
- A port whose `shared/` predates the summary fails codegen with an instruction to re-sync it.
- The generated tables stay deterministic and unchanged for the same data; the summary is a check
  on their counts, not an input to their contents.
