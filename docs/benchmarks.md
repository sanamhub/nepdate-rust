# Benchmarks

B1 to B8 of [`shared/spec/BENCHMARK.md`](../shared/spec/BENCHMARK.md), measured with the
`benches/nepdate-bench` criterion harness on the reference machine. Rust has no mature competitor
that passes [`docs/competitors.md`](competitors.md), so this run reports this port only. The C#
column is the .NET 10 column of the hub's
[`docs/benchmarks/dotnet.md`](https://github.com/sanamhub/nepdate/blob/main/docs/benchmarks/dotnet.md),
measured on the same machine on 2026-11-16.

## Machine and versions

| Item | Value |
|---|---|
| CPU | 12th Gen Intel Core i5-12500H (4 performance + 8 efficiency cores, 16 threads, base 3.1 GHz) |
| RAM | 31.7 GB |
| OS | Windows 11 Home 10.0.26300 |
| Power plan | Ultimate Performance |
| Affinity | logical CPU 2 (mask `0x4`), set with `start /b /affinity 4 /wait` |
| rustc | 1.100.0 (0ef1bc386 2026-11-09), windows-msvc (the `1.100` channel of `rust-toolchain.toml`) |
| cargo | 1.100.0 (3acc79c0c 2026-11-05) |
| criterion | 0.8.2 |
| Competitors | none; no Rust crate passes the bar |
| Date of run | 2026-11-17 |

The machine was idle: no other build, test suite or benchmark ran. The idle gate (30 one-second
samples before the run) read 9.1 percent on average with a peak of 12.1. During the run, 128
one-second samples averaged 16.0 percent with a peak of 25.7. The process was pinned to logical CPU 2
for the whole run. The samples are in [`raw/2026-11-17/cpu-samples.txt`](raw/2026-11-17/cpu-samples.txt)
and [`raw/2026-11-17/idle-gate.txt`](raw/2026-11-17/idle-gate.txt).

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

The console log is [`raw/2026-11-17/bench.log`](raw/2026-11-17/bench.log). The criterion estimates
are under `raw/2026-11-17/criterion/`. The workspace was built before the run, so compiling did not
overlap the measurement.

## Results

Time per single operation. The `nepdate` column is criterion's mean and standard deviation, divided
by 64. Bytes/op come from the `alloc` binary (a counting `#[global_allocator]`, 10,000 rounds over
the 64 inputs). B5 has two rows: the allocation-free `write_to` and, for comparison, `to_string`,
which allocates.

| ID | Operation | nepdate ns/op | std ns | bytes/op | C# .NET 10 ns/op | C# bytes/op | ADR-0004 target ns/op |
|---|---|---:|---:|---:|---:|---:|---:|
| B1 | BS to AD | 2.95 | 0.08 | 0 | 13.27 | 0 | ≤ 8 |
| B2 | AD to BS | 3.89 | 0.26 | 0 | 18.70 | 0 | ≤ 8 |
| B3 | add 1000 days | 3.34 | 0.02 | 0 | 20.06 | 0 | ≤ 8 |
| B4 | add 13 months, Clamp | 2.17 | 0.13 | 0 | 19.30 | 0 | ≤ 8 |
| B5 | long Nepali date with weekday, `write_to` | 15.97 | 0.48 | 0 | 139.04 | 435 | ≤ 40 |
| B5 | long Nepali date with weekday, `to_string` (allocates, for comparison) | 105.01 | 3.66 | 56 | 139.04 | 435 | - |
| B6 | strict parse `YYYY/MM/DD` | 13.07 | 0.53 | 0 | 21.13 | 0 | ≤ 20 |
| B7 | patro info of a day | 3.42 | 0.13 | 0 | 48.63 | 51 | ≤ 10 |
| B8 | patro month grid | 219.14 | 15.18 | 0 | 3,274.50 | 1,429 | ≤ 250 |

Every B-row beats the C# column and allocates 0 bytes. All ADR-0004 targets are met, including B8
at 219.14 ns against 250 ns and B6 at 13.07 ns against 20 ns.

## Notes

- Against the 2026-11-16 run on Rust 1.98.1, B2 is 17 percent slower (3.31 ns then), B5 `write_to`
  and B8 are 7 percent slower, and the other rows moved by less than 6 percent. A first run on
  1.100 gave the same shape (B2 3.84 ns, B8 210.21 ns), so the shift is not noise.

- `alloc` prints 0 calls and 0 bytes for every library operation. The only allocating row is the
  labelled B5 `to_string`: 640,000 calls and 35,480,000 bytes over 10,000 rounds of 64 dates, which
  is one allocation of 55.4 bytes per call. The `crates/*/tests/no_alloc.rs` tests enforce the 0
  bytes and the single exact-size allocation.
- B8 reads the tithi, holiday flag and every event name (both languages) for every day of the
  month, as BENCHMARK section 3 specifies.
- A fast wrong answer is never ranked first. There is no competitor row this time, so the ranking
  rule has nothing to apply to.
