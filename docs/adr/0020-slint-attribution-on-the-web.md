# ADR-0020: Slint attribution on the download pages, not inside the app

- **Status:** accepted
- **Date:** 2026-10-22
- **Approved by:** Sanam
- **Amends:** ADR-0013 (how the widget meets the Slint Royalty-free 2.0 attribution condition)

## Context

ADR-0013 uses Slint under the Royalty-free Desktop, Mobile, and Web Applications License 2.0
and meets its attribution condition with the `AboutSlint` widget in Settings. After testing the
beta, the owner asked for no third-party branding inside the app.

The license's section 2 gives two ways to meet the condition. Either one is enough:

> (a) Display the `AboutSlint` widget in an "About" screen or dialog that is accessible from the
> top level menu of the Application. [...]
>
> (b) Display the Slint attribution badge on a public webpage, preferably where the binaries of
> your Application can be downloaded from, in such a way that it can be easily found by any
> visitor to that page.

Text checked against `LICENSES/LicenseRef-Slint-Royalty-free-2.0.md` in `slint-ui/slint` on
2026-10-22.

## Decision

Use option (b) and remove `AboutSlint` from the app.

- The "Made with Slint" badge (`MadeWithSlint-logo-whitebg.png`, linked to https://slint.dev)
  is shown in `apps/mitibar/README.md`, in the root `README.md` section about MitiBar, and in
  the notes of every `mitibar-v*` GitHub Release, where the binaries are downloaded.
- The release workflow appends the badge to the notes, and its static test checks that it does,
  so a release can't go out without it.
- The app's About section lists the version, the MIT license and the third-party notices
  (Slint, the fonts, NepDate) as plain text. That is not branding, and the notices stay.
- `cargo-deny` keeps allowing `LicenseRef-Slint-Royalty-free-2.0` for the Slint crates only.

## Consequences

- The app shows no third-party branding.
- If MitiBar is ever distributed somewhere without a public page that carries the badge (a store
  listing that can't show it, for example), option (a) comes back for that build, or a new ADR
  picks another license.
