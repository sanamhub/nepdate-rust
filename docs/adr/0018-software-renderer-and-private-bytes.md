> Moved to [sanamhub/mitibar](https://github.com/sanamhub/mitibar) as ADR-0007. The text is kept here unchanged.

# ADR-0018: Software renderer by default, idle memory judged on private bytes

- **Status:** accepted, conditional on the Devanagari check in Decision 1
- **Date:** 2026-10-18
- **Approved by:** Sanam
- **Amends:** ADR-0010 (renderer), ADR-0008 §C (the idle memory row)

## Context

The idle measurements ran on one Windows x64 machine (Intel i5-12500H, 32 GB,
Windows 11 26300), release-widget profile, popup closed. The quiet rows from
`cargo xtask widget-idle --minutes 1` (all-CPU load below 15 % before and during):

| Renderer | Popup | Private bytes | Working set | CPU, % of one core | Threads |
|---|---|---|---|---|---|
| femtovg (ADR-0010 default) | created at start | 82.80 MB | 131.79 MB | 0.413 (load 14.4 %, max 46) | 16 |
| femtovg | created on first open | 61.79 MB | 112.92 MB | 0.825 | 15 |
| software | created on first open | 34.68 MB | 60.23 MB | 0.335 | 11 |

MB here is MiB, as the harness prints it. The summary is in `docs/benchmarks-widget.md`.

What the numbers show:

- femtovg loads the OpenGL driver into the process. Creating the popup on first open moved that load from
  startup to the first open, but with the popup never opened femtovg still costs 27 MB more
  private bytes than the software renderer, and about twice the idle CPU.
- MitiBar draws a month grid and a few lines of text in a small popup. It has no animation,
  scrolling list or large surface that needs a GPU.
- The ADR-0008 §C row "Idle RSS with popup closed ≤ 40 MB" is missed by both renderers if RSS
  means the Windows working set. The working set counts every page mapped into the process,
  including shared pages that belong to the GPU driver, DWM and system fonts. Those pages are
  shared with other processes and are not memory MitiBar can free. Private bytes are the
  process's own commit: what it costs the machine to keep MitiBar running.

## Decision

1. **Renderer.** MitiBar uses Slint's software renderer on Windows, macOS and Linux. femtovg
   leaves the default feature set; the dependency is dropped unless removing it turns out to be a
   large change, in which case it stays behind a non-default feature and the reason is recorded in a new
   ADR.

   Condition: ADR-0010 chose femtovg partly for Devanagari shaping, and every committed snapshot
   so far was rendered with femtovg. The check renders the strings in
   `docs/design/devanagari-test-strings.md` and every widget snapshot with the software renderer
   and compares them with the femtovg images. If any conjunct, reph or matra renders wrongly and the
   fix is not a font or feature setting, this decision is void, femtovg stays, and a new ADR
   amends the memory target instead. Nothing ships on the software renderer before that check.

2. **Idle memory metric.** The ADR-0008 §C idle memory row is redefined:

   | Metric | Target |
   |---|---|
   | Idle private bytes with popup closed | ≤ 40 MB |
   | Idle working set with popup closed | recorded, not judged |

   This changes the metric, not the number. It is stated plainly because it would otherwise look
   like moving the goalposts: on the working set, femtovg misses by 73 MB and software by 20 MB;
   on private bytes, femtovg misses by 22 MB and software passes with 5 MB to spare. The table in
   Context is the evidence either way.

   The load rule is unchanged: a row measured at 15 % all-CPU load or more, before or during, is
   reported as LOADED and gets no verdict on any metric.

## Consequences

- `cargo xtask widget-idle` judges private bytes against 40 MiB and prints the working set
  without a verdict.
- 0.1.0 needs a quiet 5-minute row on the software renderer that passes private bytes and CPU,
  after the check.
- If the software renderer looks worse at high DPI (no GPU antialiasing), the check records it; a
  visual regression that a user would notice is a reason to keep femtovg, the same as a shaping
  failure.
- ADR-0008 §C and ADR-0010 are not edited; this ADR is where their changed rows live.
