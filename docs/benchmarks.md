# Benchmarks

B1 to B8 of [`shared/spec/BENCHMARK.md`](../shared/spec/BENCHMARK.md), measured with the
`benches/nepdate-bench` criterion harness on the reference machine. Rust has no mature competitor
that passes [`docs/competitors.md`](competitors.md), so this run reports this port only. The C#
column is the .NET 10 column of the hub's
[`docs/benchmarks/dotnet.md`](https://github.com/sanamhub/nepdate/blob/main/docs/benchmarks/dotnet.md),
measured on the same machine on 2026-11-02.

## Machine and versions

| Item | Value |
|---|---|
| CPU | 12th Gen Intel Core i5-12500H (4 performance + 8 efficiency cores, 16 threads, base 3.1 GHz) |
| RAM | 31.7 GB |
| OS | Windows 11 Home 10.0.26300 |
| Power plan | Ultimate Performance |
| Affinity | logical CPU 2 (mask `0x4`), set with `start /b /affinity 4 /wait` |
| rustc | 1.99.0 (b940084d7 2026-09-28), windows-gnu |
| cargo | 1.99.0 (5f94df478 2026-08-27) |
| criterion | 0.8.2 |
| Competitors | none; no Rust crate passes the bar |
| Date of run | 2026-11-02 |

The machine had the owner's normal desktop applications open. No other build, test suite or
benchmark ran during the run. The load logger read 5 and 1 percent before the run and 0 and 5
percent after it; one sample on each side returned no value. The process was pinned to logical CPU 2
for the whole run.

## Command

From the repository root:

```text
cargo xtask bench
```

`xtask` runs, in order and one at a time, each pinned to logical CPU 2: the `alloc` binary and the
criterion bench. The bench crate uses the workspace release profile (`lto = "fat"`,
`codegen-units = 1`, the settings ADR-0004 measured with). criterion settings are warm-up 3 s,
measurement 5 s, 100 samples and `Throughput::Elements(64)`, so the report is time per single
operation (iteration time / 64).

The console log is [`raw/2026-11-02/bench.log`](raw/2026-11-02/bench.log). The criterion estimates
are under `raw/2026-11-02/criterion/`.

## Results

Time per single operation. The `nepdate` column is criterion's mean and standard deviation, divided
by 64. Bytes/op come from the `alloc` binary (a counting `#[global_allocator]`, 10,000 rounds over
the 64 inputs). B5 has two rows: the allocation-free `write_to` and, for comparison, `to_string`,
which allocates.

| ID | Operation | nepdate ns/op | std ns | bytes/op | C# .NET 10 ns/op | C# bytes/op | ADR-0004 target ns/op |
|---|---|---:|---:|---:|---:|---:|---:|
| B1 | BS to AD | 2.95 | 0.02 | 0 | 13.50 | 0 | ≤ 8 |
| B2 | AD to BS | 3.31 | 0.03 | 0 | 20.10 | 0 | ≤ 8 |
| B3 | add 1000 days | 3.24 | 0.02 | 0 | 19.05 | 0 | ≤ 8 |
| B4 | add 13 months, Clamp | 2.11 | 0.03 | 0 | 20.19 | 0 | ≤ 8 |
| B5 | long Nepali date with weekday, `write_to` | 14.67 | 0.98 | 0 | 150.41 | 435 | ≤ 40 |
| B5 | long Nepali date with weekday, `to_string` (allocates, for comparison) | 112.06 | 1.07 | 56 | 150.41 | 435 | - |
| B6 | strict parse `YYYY/MM/DD` | 12.60 | 0.62 | 0 | 22.42 | 0 | ≤ 20 |
| B7 | patro info of a day | 3.38 | 0.05 | 0 | 48.21 | 51 | ≤ 10 |
| B8 | patro month grid | 199.49 | 1.50 | 0 | 3,112.76 | 1,429 | ≤ 250 |

Every B-row beats the C# column and allocates 0 bytes. All ADR-0004 targets are met, including B8
at 199.49 ns against 250 ns and B6 at 12.60 ns against 20 ns.

## Notes

- `alloc` prints 0 calls and 0 bytes for every library operation. The only allocating row is the
  labelled B5 `to_string`: 640,000 calls and 35,480,000 bytes over 10,000 rounds of 64 dates, which
  is one allocation of 55.4 bytes per call. The `crates/*/tests/no_alloc.rs` tests enforce the 0
  bytes and the single exact-size allocation.
- B8 reads the tithi, holiday flag and every event name (both languages) for every day of the
  month, as BENCHMARK section 3 specifies.
- A fast wrong answer is never ranked first. There is no competitor row this time, so the ranking
  rule has nothing to apply to.
