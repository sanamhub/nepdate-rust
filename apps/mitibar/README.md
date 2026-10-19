# MitiBar

MitiBar keeps today's Bikram Sambat date in the tray. It opens a month grid, day detail, a BS/AD
converter and settings, and on Windows 11 it can draw the date and time inside the taskbar. It
makes no network requests.

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

0.1.0 is not released. The Windows build is complete; the release waits for a quiet idle
measurement and a manual pass on macOS and Linux. Release instructions are in
[`docs/runbooks/release-mitibar.md`](../../docs/runbooks/release-mitibar.md).

## License

MIT, see the repository [`LICENSE`](../../LICENSE). The bundled Noto fonts are under the SIL Open
Font License. Slint is used under its royalty-free licence.
