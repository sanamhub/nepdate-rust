# Benchmarks (task L6-02)

B1 to B8 of [`shared/spec/BENCHMARK.md`](../shared/spec/BENCHMARK.md), measured with the
`benches/nepdate-bench` criterion harness on the reference machine, plus the four competitor
crates for B1 and B2. The C# column is the .NET 10 column of the hub's
[`docs/benchmarks/dotnet.md`](https://github.com/sanamhub/nepdate/blob/1eab1cb/docs/benchmarks/dotnet.md)
(task H6, hub commit `1eab1cb`).

## Machine and versions

| Item | Value |
|---|---|
| CPU | 12th Gen Intel Core i5-12500H (4 performance + 8 efficiency cores, 16 threads, base 3.1 GHz) |
| RAM | 31.7 GB |
| OS | Windows 11 Home 10.0.26300 |
| Power plan | Ultimate Performance |
| Affinity | logical CPU 2 (mask `0x4`), set per command (see below) |
| rustc / cargo | 1.98.1 (48a229cea 2026-09-01), windows-gnu |
| criterion | 0.8.2 |
| Competitors | npdatetime 0.2.10, adtobs 0.2.0, nepali-date-converter 0.1.1, nepdate-cli 0.1.5 (pinned) |
| Date of run | 2026-10-15 |

The machine was not fully idle: the C++ port's agent was running on the same machine, but no
heavy build was in progress when the benchmarks started (CPU load about 2 %). Laptop numbers are
indicative, not a quiet-machine guarantee.

## Command

`cargo xtask bench` runs, in order and one at a time, each pinned to logical CPU 2 (mask `0x4`):

```text
$ start /b /affinity 4 /wait cargo run -p nepdate-bench --release --bin alloc
$ start /b /affinity 4 /wait cargo run -p nepdate-bench --release --bin agree
$ start /b /affinity 4 /wait cargo bench -p nepdate-bench
```

(`/b` keeps the output in the same console; without it `start` opens a new window.) The bench
crate's release profile is the workspace one (`lto = "fat"`, `codegen-units = 1`), the settings
ADR-0004 measured with. criterion settings: warm-up 3 s, measurement 5 s, 100 samples,
`Throughput::Elements(64)`, so the report is time per single operation (iteration time / 64).

## Results

Time per single operation. The `nepdate` column is criterion's mean; bytes/op come from the
`alloc` binary (a counting `#[global_allocator]`, 10,000 rounds over the 64 inputs). B5 has two
rows: the allocation-free `write_to` and, for comparison, `to_string` (which allocates).

| ID | Operation | nepdate ns/op | bytes/op | C# .NET 10 ns/op | C# bytes/op | ADR-0004 target ns/op |
|---|---|---:|---:|---:|---:|---:|
| B1 | BS to AD | 2.90 | 0 | 13.29 | 0 | ≤ 8 |
| B2 | AD to BS | 3.10 | 0 | 18.64 | 0 | ≤ 8 |
| B3 | add 1000 days | 3.19 | 0 | 17.50 | 0 | ≤ 8 |
| B4 | add 13 months, Clamp | 2.60 | 0 | 18.85 | 0 | ≤ 8 |
| B5 | long Nepali date with weekday, `write_to` | 16.04 | 0 | 153.02 | 435 | ≤ 40 |
| B5 | long Nepali date with weekday, `to_string` (allocates, for comparison) | 128.91 | 55 | 153.02 | 435 | - |
| B6 | strict parse `YYYY/MM/DD` | 18.38 | 0 | 21.55 | 0 | ≤ 20 |
| B7 | patro info of a day | 3.46 | 0 | 47.93 | 51 | ≤ 10 |
| B8 | patro month grid | 298.80 | 0 | 3,132.66 | 1,429 | ≤ 250 |

Every B-row beats the C# column and allocates 0 bytes. B8 misses its ADR-0004 target (298.80 ns
against 250 ns) but is 10.5 times faster than C#. It was not micro-optimised: the plan says to
report a miss rather than speculate. See the note below.

### Competitors (B1 and B2 only)

The `agree` binary compares each competitor with `nepdate` on the 64 dates and prints every
differing date with both answers. A rejected input counts as a difference, so a fast wrong
answer is never listed as agreeing.

| ID | Competitor | ns/op | agrees |
|---|---|---:|---|
| B1 | npdatetime | 2,805.86 | no (2 of 64 differ) |
| B1 | adtobs | 165.17 | no (3 of 64 differ) |
| B1 | nepali-date-converter (includes string parsing) | 94.63 | no (7 of 64 differ) |
| B1 | nepdate-cli | 60.89 | no (3 of 64 differ) |
| B2 | npdatetime | 1,679.56 | no (2 of 64 differ) |
| B2 | adtobs | 68,364.55 | no (5 of 64 differ) |
| B2 | nepali-date-converter (includes string parsing) | 113.35 | no (7 of 64 differ) |
| B2 | nepdate-cli | 147.12 | no (3 of 64 differ) |

`adtobs` B2 ran over the 62 inputs its AD range accepts: its `1944..=2033` range rejects AD
1943-04-14 (BS 2000-01-01) and AD 2034-03-08 (BS 2090-11-24, the last of the 64 inputs). Those
two count as "differ" above.

Apart from those two rejections, every difference is one or two days, in both directions for
the same dates: the competitor uses a different length for some month of that BS year, so it
is a disagreement about the calendar table, not about the conversion arithmetic. `nepdate`
uses the hub's table (`shared/data`, the same as the C# NepDate library and checked against its
golden files). The differing dates, as `agree` prints them:

| BS date (nepdate) | AD (nepdate) | npdatetime | adtobs | nepali-date-converter | nepdate-cli |
|---|---|---|---|---|---|
| 2073-07-28 | 2016-11-13 | same | same | 2016-11-14 | same |
| 2079-05-06 | 2022-08-22 | same | same | 2022-08-23 | same |
| 2083-09-05 | 2026-12-20 | same | same | 2026-12-21 | same |
| 2085-02-17 | 2028-05-30 | same | same | 2028-05-31 | same |
| 2086-07-23 | 2029-11-08 | same | 2029-11-09 | 2029-11-09 | 2029-11-09 |
| 2088-01-04 | 2031-04-19 | 2031-04-18 | 2031-04-18 | 2031-04-18 | 2031-04-17 |
| 2089-06-10 | 2032-09-27 | 2032-09-26 | 2032-09-26 | 2032-09-26 | 2032-09-26 |

The first four rows are past or current years, which a reader can check against a published
calendar. BS years after the current one have no official calendar yet, so every library,
`nepdate` included, ships a projection for them; differences there show that the projections
differ, not which one is right.

## Notes

- `alloc` prints 0 calls and 0 bytes for every library B-operation; the only allocating row is
  the labelled B5 `to_string`: one allocation per call, of the exact length (55 bytes on
  average over the 64 dates). `Display` collects the pieces on the stack and hands the
  `String` one write; before that change it was 3 allocations, 141 bytes and 267.81 ns per
  call. The `crates/*/tests/no_alloc.rs` tests enforce the 0 bytes, and the single exact-size
  allocation of `to_string`, in CI.
- The B5 `to_string` row was measured on 2026-10-15 after that change, with the machine at
  about 59 % load (the C++ port's build); the `write_to` row ran about 20 % slower in the same
  run, so 128.91 ns is if anything high.
- B8 reads the tithi, holiday flag and every event name (both languages) for every day of the
  month, as the plan specifies. That is heavier than the ADR-0004 prototype (which measured a
  lighter read at 209 ns quiet / 285 ns busy), and the run above was not on a fully idle machine.
- The B6 target (≤ 20 ns) was met at 18.38 ns, so the fallback fast-path in ADR-0004 was not
  needed.
