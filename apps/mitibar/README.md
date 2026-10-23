<p align="center">
  <img src="https://raw.githubusercontent.com/sanamhub/nepdate-rust/main/apps/mitibar/icons/128x128.png" width="96" alt="MitiBar icon">
</p>

<h1 align="center">MitiBar</h1>

<p align="center">
  <a href="https://github.com/sanamhub/nepdate-rust/releases"><img src="https://img.shields.io/github/v/release/sanamhub/nepdate-rust?include_prereleases&filter=mitibar-v*&label=MitiBar" alt="MitiBar release"></a>
  <a href="https://github.com/sanamhub/nepdate-rust/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License: MIT"></a>
</p>

MitiBar keeps today's Bikram Sambat date in the tray. It opens a month grid, day detail, a BS/AD
converter and settings, and on Windows 11 it can draw the date and time inside the taskbar. It
makes no network requests.

<a href="https://slint.dev"><img src="https://raw.githubusercontent.com/slint-ui/slint/v1.18.1/logo/MadeWithSlint-logo-whitebg.png" alt="Made with Slint" width="160"></a>

## Download

Get it from the [releases](https://github.com/sanamhub/nepdate-rust/releases): an NSIS installer
or portable zip for Windows (x64 and ARM64), a universal `.dmg` for macOS, and an `.AppImage` or
`.deb` for Linux. The beta is not code-signed, so Windows SmartScreen and macOS Gatekeeper warn on
the first start; the release notes say how to open it and how to check a download against
`SHA256SUMS` and its build attestation.

## Build

From the repository root:

```bash
cargo run -p mitibar --release
```

On Linux, install `libxkbcommon-dev`, `libwayland-dev`, `libfontconfig1-dev` and `libdbus-1-dev`
first. The renderer is software only, so no OpenGL or GPU driver is needed.

Settings live in `%APPDATA%\mitibar\settings.toml` on Windows,
`~/Library/Application Support/mitibar/settings.toml` on macOS and
`~/.config/mitibar/settings.toml` on Linux. Logs go to the matching local data folder.

## Status

0.1.0-beta.1 is the first public pre-release, unsigned. 0.1.0 waits for a quiet idle and
cold-start measurement and a manual pass on macOS and Linux. Release instructions are in
[`docs/runbooks/release-mitibar.md`](https://github.com/sanamhub/nepdate-rust/blob/main/docs/runbooks/release-mitibar.md).

## License

MIT, see the repository [`LICENSE`](https://github.com/sanamhub/nepdate-rust/blob/main/LICENSE). The bundled Noto fonts are under the SIL Open
Font License. Slint is used under its royalty-free licence.
