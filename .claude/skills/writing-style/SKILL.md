---
name: writing-style
description: House writing rules for this repo. Load before writing or editing any prose, including README, specs, ADRs, plans, learn notes, runbooks, changelogs, commit messages, PR descriptions, issue text, code comments and rustdoc. Use it as a final pass on anything already written. Trigger on "write the docs", "update the README", "add an ADR", "commit this", "open a PR", "add comments", or any request that produces text a human will read.
---

# Writing style

Everything written here has to read like an engineer wrote it. Not a model. The reviewer is
learning Rust, and weaker agents follow these docs literally, so plain and exact beats clever.
These rules apply to every agent, not only Claude (`AGENTS.md` points here). The same file
is used in every NepDate repository; change it in the hub (`sanamhub/nepdate`) first.

## Hard rules

1. **No em dashes.** Use a period, a comma, parentheses, or a colon.
2. **No AI filler.** Banned outright: delve, leverage (as a verb), seamless, robust,
   comprehensive, cutting-edge, best-in-class, game-changing, unlock, empower, foster,
   navigate (figurative), realm, landscape, tapestry, "it's worth noting", "it's important to
   note", "in today's world", "at the end of the day".
3. **No closing summary that restates the section.** Stop when the point is made.
4. **No duplication.** If a fact appears in two places, one of them links to the other. Specs
   own behaviour, ADRs own decisions, plans own tasks (`AGENTS.md` §1).
5. **Short paragraphs.** Three or four sentences. Break anything longer.
6. **Plain words.** "fix" not "implement a solution for". "use" not "utilise". "so" not
   "thereby". "start" not "commence".
7. **No hedging stacks.** Pick one: "probably", not "it may potentially be possible that".
8. **No triads for rhythm.** Three items only when there are genuinely three.
9. **Exact over vague.** Numbers, file paths, task IDs (`L2-03`), requirement IDs (`WR-03 AC2`)
   and exact commands. "Fast" is not a requirement. "< 20 ns" is.

## Commits and PRs

- Conventional Commits: `type(scope): summary`. Types: feat, fix, docs, chore, refactor, test,
  build, ci, perf. Scopes: the package or area (`nepdate`, `widget`, `xtask`, `shared`, `ci`; in the
  hub `data`, `spec`, `tools`). Put the task ID first in the summary when there is one.
- Summary in the imperative, lower case, no trailing period, under 72 characters.
- Body explains why, not what. The diff already says what.
- **Never add `Co-Authored-By`, `Generated with`, or any other AI attribution.**
- PR descriptions follow `.github/pull_request_template.md`. "What to look at first" is two or
  three bullets a Rust beginner can act on.

Good:

```
feat(nepdate): L2-03 serial conversion

Year lookup uses partition_point over YEAR_START, so AD to BS is one
binary search over 299 entries instead of a walk from 1901.
```

Bad:

```
feat: 🚀 Implement comprehensive BS/AD conversion

This commit leverages cutting-edge techniques to seamlessly convert...

Co-Authored-By: Claude <noreply@anthropic.com>
```

## Code comments and rustdoc

Comment why, not what. If the code needs a comment to say what it does, rename something.

Good:

```rust
// Ashad and Ashoj both abbreviate to "Ash" in C#. D-04 (ADR-0009) splits them.
```

Bad:

```rust
// This function adds months to the date and returns the new date.
```

Rustdoc on every public item is required (`#![deny(missing_docs)]` from L6). Say what it does,
what it returns, and when it fails (an `# Errors` section for every `Result`). Examples are
doctests, so they compile and run. Skip the marketing.

## Docs, specs and plans

- Lead with what the thing is and who it is for. No throat clearing.
- State limits honestly and early (range BS 1901 to 2199, data coverage, unsigned builds). A doc
  that hides a limit costs more trust than the limit does.
- Tables for facts. Prose for reasoning.
- Code samples must compile. If they can't yet, say so.
- Acceptance criteria are testable statements, one per line, each tied to a test or a line in
  the manual checklist.
- Nepali text is Devanagari in UTF-8. Don't transliterate where the spec says Devanagari.

## Final pass

Read it back and cut:

- Every em dash.
- Every sentence that could be deleted without losing information.
- Every word from the banned list.
- Every paragraph over four sentences.
- Every restatement of something said above.

If a section survives the cut unchanged, it was probably already fine.
