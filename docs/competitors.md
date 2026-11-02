# Other Rust crates for the Bikram Sambat calendar

Checked 2026-11-02 against the crates.io search API, each crate's metadata, GitHub and the crate
sources. The selection follows [`shared/spec/BENCHMARK.md`](../shared/spec/BENCHMARK.md) section 7.
Usage is crates.io's recent downloads (90 days), read from the crates.io API on 2026-11-02. The bar
is 1,000 recent downloads, first release at least 12 months before 2026-11-02, a release or commit
in the last 36 months, a license, not archived or deprecated, and a build on the current toolchain.

## Candidate table

| Crate | Version | Recent 90 days | First release | Last release | License | Verdict |
|---|---|---:|---|---|---|---|
| `npdatetime` | 0.2.11 | 437 | 2026-02-06 | 2026-10-26 | MIT | fails the 12-month bar and 1,000 recent downloads |
| `adtobs` | 0.2.0 | 560 | 2023-11-29 | 2026-04-30 | GPL-3.0 | fails 1,000 recent downloads |
| `nepali-date-converter` | 0.1.1 | 379 | 2026-02-20 | 2026-02-25 | MIT | fails the 12-month bar and 1,000 recent downloads |
| `nepdate-cli` (library `bikram`) | 0.1.5 | 313 | 2024-10-19 | 2026-02-05 | GPL-3.0-or-later | fails 1,000 recent downloads |
| `ndate` | 0.1.0 | 0 | 2021-07-07 | 2021-07-07 | MIT | fails the 36-month bar and 1,000 recent downloads |
| `npltz` | 1.2.1 | 42 | 2026-06-22 | 2026-10-16 | MIT | fails the 12-month bar and 1,000 recent downloads; terminal app |
| `ncal` | 0.3.0 | 46 | 2026-06-16 | 2026-09-02 | MIT | fails the 12-month bar and 1,000 recent downloads; terminal app |
| `teistro-calendar` | 0.0.0 | 12 | 2026-09-10 | 2026-09-10 | Apache-2.0 | fails the 12-month bar and 1,000 recent downloads; placeholder |
| `nepali-chrono` | 0.0.0 | 0 | 2026-03-09 | 2026-03-09 | MIT | fails the 12-month bar and 1,000 recent downloads; placeholder |

`yorion_engine` is a Git repository only, with no crates.io release, so it has no crates.io usage to
measure and is not in the table. The crates.io names `nepdate`, `nepdate-patro`, `nepali-date` and
`bikram-sambat` are this family or placeholders, not candidates.

## Result

No candidate passes both the maturity bar and the 1,000 recent downloads minimum. Rust has no mature
library to compare, so the benchmark reports NepDate only and the report says so. The four crates
that were measured in earlier reports are removed from the harness and their old numbers are not
kept.

## Sources

crates.io API (`/api/v1/crates/<name>` and `/api/v1/crates?q=...`) for every crate above, the crate
sources from static.crates.io, and these repositories:
https://github.com/4mritGiri/npdatetime, https://github.com/amrit073/adtobs-rs,
https://github.com/dhurbachy/Nepali-Date-Converter, https://github.com/khumnath/nepdate-cli,
https://github.com/Yorion-io/yorion_engine.
