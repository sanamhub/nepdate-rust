# MitiBar performance (W4-05)

Measured on 2026-10-18 for MitiBar 0.1.0, against the targets in ADR-0008 §C. The machine is
shared and was busy, so every runtime number carries the all-CPU load beside it.

## Machine and method

| Item | Value |
|---|---|
| Machine | Acer Nitro AN515-58, Intel Core i5-12500H, 12 cores / 16 logical processors, 32 GB RAM |
| OS | Windows 11 Home 10.0.26300 |
| Toolchain | rustc 1.98.1, x86_64-pc-windows-msvc |
| Build | `cargo build -p mitibar --profile release-widget` (`opt-level = "s"`, fat LTO, `panic = "abort"`, stripped) |
| Load sample | `Win32_PerfFormattedData_PerfOS_Processor` `_Total.PercentProcessorTime` |

`LOADED` means the all-CPU average during the row is above 20 %. A loaded row is reported but
**not judged**: a quiet re-run is needed before it can pass or fail. Binary size is not a timing
measurement and is judged even on a loaded machine.

The cold-start number is the `tray ready in ...` line that `main` logs from the start of `main`
to the first `update_tray` call. Each run was a fresh process with the popup closed and settings
`launch_at_login = false`, `taskbar_text = true`, English, 12-hour clock, seconds hidden. The
settings file was temporary and was removed after the run.

Idle RSS is `WorkingSet64` after five minutes with the popup closed. Idle CPU is the process
`TotalProcessorTime` delta divided by wall time, as a percentage of one core (the same unit as
the W1 and W1B checks). Load was sampled 30 times, once every 10 s, during the five minutes.

## Results

| Metric | Target | Measured | All-CPU load | Verdict |
|---|---|---|---|---|
| Windows x64 binary, stripped | ≤ 15 MB (stretch ≤ 8 MB) | 12,519,936 bytes = 12.52 MB = 11.94 MiB | 61.8 % average during the idle window (size is load-independent) | meets 15 MB, misses the 8 MB stretch |
| macOS universal binary | ≤ 15 MB | not measured locally; built only in `release-mitibar.yml` | not measured | not judged |
| Linux x86_64 binary | ≤ 15 MB | not measured locally; built only in `release-mitibar.yml` | not measured | not judged |
| Idle RSS, popup closed, 5 min | ≤ 40 MB | 153.86 MB working set (89.48 MB private) | 61.8 % average, range 19 to 100 % | LOADED, no verdict; the raw number is above 40 MB |
| Cold start to tray-ready | ≤ 500 ms | 164.0 ms median of 5 (155.7 to 227.0 ms) | 7 to 99 % per run, about 44 % average of the before/after samples | LOADED, no verdict; the raw numbers are below 500 ms |
| Idle CPU, popup closed, 5 min | ~0 %, wake at most once a minute | 6.87 % of one core (21.328 CPU seconds in 310.4 s) | 61.8 % average, range 19 to 100 % | LOADED, no verdict; the raw number is above the target |

### Cold start, all five runs

| Run | Tray-ready | Load before | Load after |
|---|---|---|---|
| 1 | 162.5 ms | 7 % | 26 % |
| 2 | 155.7 ms | 30 % | 60 % |
| 3 | 198.0 ms | 25 % | 44 % |
| 4 | 227.0 ms | 58 % | 99 % |
| 5 | 164.0 ms | 38 % | 53 % |

## What this means

- Binary size meets the 15 MB requirement and misses the 8 MB stretch goal. No feature was added
  or removed to change it.
- The idle RSS (153.86 MB) is far above the 40 MB target, and idle CPU (6.87 % of one core) is
  above the about-0 % target. Both rows are LOADED, so they are not verdicts: the machine was
  between 19 % and 100 % busy, averaging 61.8 %. The numbers are high enough to need a quiet
  re-run before the human accepts 0.1.0; they are reported here rather than hidden.
- Cold start is below 500 ms even on this load. It is still marked LOADED and not judged.
- macOS and Linux were not built or measured on this Windows machine. Their sizes and timings
  come from the release workflow run, which needs a GitHub remote.
