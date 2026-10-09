# Learn 12: Shipping a desktop app (phase W4)

## 1. Release profiles

```toml
[profile.release-widget]
inherits = "release"
opt-level = "s"      # optimise for size
lto = "fat"          # whole-program optimisation across crates
codegen-units = 1    # slower compile, smaller/faster binary
panic = "abort"      # no unwinding tables → smaller; a panic ends the process
strip = true         # remove symbols
```

Measure with `cargo build --profile release-widget` and check the file size.
`cargo bloat` (optional tool) shows which crates take space.

## 2. Packaging

`cargo-packager` builds the OS-native formats from one config: NSIS installer (Windows),
`.app`/`.dmg` (macOS), AppImage/`.deb` (Linux). A tray app on macOS sets `LSUIElement`
so it has no Dock icon.

## 3. Code signing (why users see warnings)

Unsigned apps trigger Windows SmartScreen and macOS Gatekeeper warnings. Signing needs paid
certificates: Authenticode on Windows, an Apple Developer ID plus notarization on macOS.
Certificates live only in CI secrets (org §6). v0.x previews may ship unsigned with a note
(ADR-0012).

## 4. Releases are approval-gated

The agent prepares everything (version bump, changelog). A human tags, approves the
`production` environment, and for the widget publishes the draft (org §12). The library
is published by trusted publishing: GitHub proves to crates.io which workflow is running,
so no password or token is stored anywhere. Rollback = hide the release and point users to
the previous one (runbook).
