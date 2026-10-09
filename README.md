# nepdate-rust

Bikram Sambat (Nepali calendar) for Rust, plus a tiny cross-platform desktop widget.
Part of the NepDate family: shared data and spec live in the hub
[`sanamhub/nepdate`](https://github.com/sanamhub/nepdate); other languages have their own
repositories.

| Part | Path | What |
|---|---|---|
| `nepdate` crate | `crates/nepdate` | `no_std`, zero-dependency BS↔AD conversion, formatting (English/Nepali), parsing, arithmetic, fiscal year |
| `nepdate-patro` crate | `crates/nepdate-patro` | the patro (पात्रो, the Nepali almanac): tithi, public holidays and events per BS day, on top of `nepdate`. Separate so the core carries no patro data. Same version as `nepdate`, released together |
| `nepdate-widget` | `apps/nepdate-widget` | tray / menu-bar app (Windows, macOS, Linux): today's BS date, month calendar, converter, clock with time zone |
| Shared data and spec | `shared/` | read-only copy of the hub's `data/` and `spec/`, updated by a weekly PR. Read only at dev time |

**Status:** L0 to L2 done (workspace, core types, BS↔AD conversion); L3 (arithmetic, fiscal year, ranges) ready for review; next: phase L4.

Supported range: **BS 1901-01-01 to 2199-12-30** (AD 1844-04-11 to 2143-04-15).
Patro data (`nepdate-patro`): BS 2077-01-01 to 2084-08-20; tithi 2078-01-02 to 2083-12-30,
public holidays until 2084-08-18.

Performance goal: faster than C# NepDate on every operation of
[`shared/spec/BENCHMARK.md`](shared/spec/BENCHMARK.md) (B1 to B8), with zero heap allocation in
both crates. How: [ADR-0004](docs/adr/0004-date-representation-and-conversion-algorithm.md).

## Where to start

| You are | Read |
|---|---|
| an AI agent implementing a phase | [AGENTS.md](AGENTS.md) → [docs/plan/00-ROADMAP.md](docs/plan/00-ROADMAP.md) |
| the reviewer learning Rust | [docs/learn/00-setup-and-workflow.md](docs/learn/00-setup-and-workflow.md) |
| wondering "why is it like this?" | [docs/adr/README.md](docs/adr/README.md) |
| looking for the algorithm or data | [shared/spec/README.md](shared/spec/README.md) (changes go to the hub) |

## Repository layout

```
shared/          hub copy: data/ (calendar JSON), spec/ (specs, vectors), VERSION (hub commit)
crates/nepdate/  the core library       (created in phase L0)
crates/nepdate-patro/  tithi, holidays, events (created in phase L5)
apps/            the widget             (created in phase W0)
xtask/           dev automation: codegen, checks, release helpers (created in phase L0)
docs/API.md      the public Rust API contract
docs/adr/        architecture decisions for the Rust port
docs/plan/       phases, tasks, acceptance criteria, widget requirements
docs/learn/      Rust learning notes, one per phase
docs/runbooks/   release and manual-test procedures
```

## Credits and license

- Calendar data and algorithm from [NepDate](https://github.com/RajuPrasai/NepDate) by Raju
  Prasai and contributors, MIT. See [shared/data/NOTICE](shared/data/NOTICE).
- The widget follows [NepDateWidget](https://github.com/RajuPrasai/NepDateWidget), with its
  author's permission ([docs/legal](docs/legal/README.md)).
- This repository: MIT ([ADR-0013](docs/adr/0013-licensing.md)).
- Not affiliated with the Government of Nepal. Holiday data is informational.
