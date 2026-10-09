# AGENTS.md: rules for AI coding agents

You are implementing a planned project. The plan is detailed on purpose: **follow it;
don't redesign it.** The human reviewer is learning Rust, so clarity beats cleverness.

## 0. Before writing any code

1. Read `README.md`, this file, and **every** ADR in `docs/adr/` whose Status is Accepted.
2. Open `docs/plan/00-ROADMAP.md`, find the phase you were asked to do, and read that
   phase file completely, including the spec sections it lists under "Read first".
3. Check the phase's preconditions (e.g. W0 requires L2 merged). If any is unmet: **stop and
   say so.**

## 1. Sources of truth (in priority order)

1. `shared/spec/*.md` + `shared/spec/vectors/*`: behaviour. Vectors win over prose if they disagree
   (then report the disagreement).
2. `docs/adr/*`: decisions and constraints.
3. `docs/plan/*`: tasks, file paths, acceptance criteria.
4. `shared/`: a read-only copy of the hub `sanamhub/nepdate` (ADR-0003). **Never edit it.** A
   data or spec problem is reported to the human as a hub issue; the fix arrives through the
   weekly `shared-sync` PR.

Never use the C# NepDate docs (README/llms.txt) as a source: they are known to be wrong
(`shared/spec/PARITY.md` §1). The C# code and the golden vectors are the reference. NepDateWidget may be read
for widget behaviour and wording, but only `docs/plan/widget-requirements.md` decides what is
built (ADR-0008).

## 2. Hard rules

- No new public API beyond `docs/API.md`. No new dependencies beyond those named in the ADRs.
  Need one? Stop and ask.
- Libraries (`nepdate`, `nepdate-patro`): `#![no_std]`, `#![forbid(unsafe_code)]`, no
  allocation, no panics on any input (no `unwrap`/`expect`/`panic!`/unchecked indexing in
  non-test code).
- `nepdate` never depends on `nepdate-patro` and contains no patro data (ADR-0005). Patro
  code and tables go in `crates/nepdate-patro` only.
- Don't edit generated files by hand (`crates/nepdate/src/generated/*`,
  `crates/nepdate-patro/src/generated/*`). Change `xtask` and run `cargo xtask codegen`.
- Don't edit an Accepted ADR (only its Status line, when superseding). New decision → new
  ADR, proposed to the human. (Until L0 is merged the human may still revise ADRs, ADR-0001.)
- Don't weaken, skip or delete a test to make CI pass. Parity/vector tests may skip rows
  **only** for deviations listed in `shared/spec/PARITY.md`, with a comment naming the D-ID.
- Keep features out that aren't in the plan ("no feature slop"). If a plan item looks
  wrong, raise it; don't silently "improve" it.
- Secrets never go in code, logs, or commits.

## 3. How to work

- One task ID at a time (`L2-03`). Small commits: `feat(nepdate): L2-03 serial conversion`.
- Write the test for an acceptance criterion first, see it fail, then implement.
- Run `cargo xtask ci` before saying you're done. Paste the summary line in the PR.
- Match the style of existing code. Comments explain *why*, not *what*.
- In every PR description, include a **"What to look at first"** section (2–3 bullets) and a
  **"Rust concepts used"** list linking to the relevant `docs/learn/` section. The reviewer
  learns from these.

## 4. Writing

All prose (docs, ADRs, plans, changelog, commits, PRs, issues, comments, rustdoc) follows
[`.claude/skills/writing-style/SKILL.md`](.claude/skills/writing-style/SKILL.md). It applies to
every agent, not only Claude. The short version: no em dashes, no filler words, plain words,
short paragraphs, exact numbers and IDs, why-not-what in comments and commit bodies, and
**no AI attribution** (`Co-Authored-By`, "Generated with") in commits or PRs.

## 5. When stuck

- Compiler error you don't understand: read the full message (`rustc --explain E0XXX`).
  Don't add `clone()`, `unsafe`, `'static` or `Rc` at random to silence it.
- Spec ambiguity: write the question in the PR / chat and stop. Don't guess.
- Toolchain/crate API mismatch (e.g. a Slint API name changed): check the docs for the
  pinned version (docs.rs / docs.slint.dev), then note the difference in the PR.

## 6. Definition of Done

See "Global Definition of Done" in `docs/plan/00-ROADMAP.md`. A task is done when all of it
is true, not when the code compiles.
