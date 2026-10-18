# Runbook: release MitiBar

Owner: maintainer. Approver: a second person (org §10/§12). Decided in ADR-0012. The tag is
`mitibar-vX.Y.Z`; the display name is MitiBar and every identifier is `mitibar` (ADR-0017).
The workflow creates a **draft**; a human installs the assets and publishes it.

## Before tagging

1. CI green on all 3 OSes. `docs/runbooks/widget-manual-test.md` completed on Windows, macOS and
   Linux for this version (attach the ticked copy to the release PR).
2. `apps/mitibar/Cargo.toml` and `apps/mitibar/Packager.toml` have the new `version`; the
   changelog has a `## [X.Y.Z] - YYYY-MM-DD` section; `cargo xtask release-check mitibar X.Y.Z`
   passes.
3. `cargo xtask data-status` exits 0. It fails only when public holidays reach less than 183 days
   ahead; copy any `WARNING` line for tithi or events into the release PR. If it fails, land the
   data through the hub's
   [data-update runbook](https://github.com/sanamhub/nepdate/blob/main/docs/runbooks/data-update.md)
   first, never by lowering the threshold.
4. `docs/benchmarks-widget.md` has current numbers from a quiet machine (ADR-0008 §C), and no
   row is LOADED. **0.1.0 is not releasable yet:** W5 removed the popup-at-start cost and the
   250 ms poll, but femtovg still measures 112.92 MB working set and 0.825 % of one core against
   the 40 MB and about-0 targets. The software renderer measured 60.23 MB and 0.335 %. A new ADR
   for the renderer or the target is needed before 0.1.0.
5. `cargo xtask ci` passes locally.

## Build the packages locally (optional)

Windows and Linux build their own host package; macOS builds both arches and joins them:

```bash
cargo xtask icons --check
cargo xtask package                  # Windows: NSIS + portable zip, Linux: AppImage + deb
# macOS:
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build -p mitibar --profile release-widget --target aarch64-apple-darwin
cargo build -p mitibar --profile release-widget --target x86_64-apple-darwin
mkdir -p target/universal/release-widget
lipo -create -output target/universal/release-widget/mitibar \
  target/aarch64-apple-darwin/release-widget/mitibar \
  target/x86_64-apple-darwin/release-widget/mitibar
MITIBAR_BINARIES_DIR=target/universal/release-widget cargo xtask package
```

Outputs are in `dist/` (override with `CARGO_TARGET_DIR` for the build cache, not for `dist/`).
`cargo xtask package` needs `cargo-packager` 0.11.8; the release workflow installs it with
`cargo install cargo-packager --version 0.11.8 --locked`.

| OS | Formats |
|---|---|
| Windows x86_64 | NSIS `.exe` installer (per-user, no admin) and a portable `.zip` |
| macOS universal | `.dmg` containing `MitiBar.app`; `LSUIElement` is set, so no Dock icon |
| Linux x86_64 | `.AppImage` and `.deb` |

## Check the downloads

`SHA256SUMS` uses one line per asset: `<sha256>  <filename>`. The SBOM is
`MitiBar-<version>.cdx.json` (CycloneDX 1.5, JSON).

```bash
sha256sum -c SHA256SUMS
gh attestation verify mitibar_<version>_x64-setup.exe --repo sanamhub/nepdate-rust
```

On Windows without `sha256sum`:

```powershell
Get-Content SHA256SUMS | ForEach-Object {
  $hash, $name = $_ -split '\s+', 2
  $actual = (Get-FileHash -Algorithm SHA256 $name).Hash.ToLower()
  if ($actual -ne $hash) { throw "$name: checksum mismatch" }
  "$name ok"
}
```

## Install and uninstall

### Windows

1. Run `mitibar_<version>_x64-setup.exe` (cargo-packager names the installer after the binary
   and version). It installs for the current user without administrator rights. The portable
   alternative is `MitiBar-<version>-windows-x64-portable.zip`; unpack it anywhere and run
   `MitiBar.exe`.
2. The installer adds Start Menu and uninstall entries. It does not create a login item; that
   happens only when the app starts with the default `launch_at_login = true` (or the setting is
   turned on).
3. Uninstall from Settings > Apps > MitiBar, or run the uninstaller in the install folder. The
   per-user install folder is `%LOCALAPPDATA%\Programs\MitiBar` (not
   `%LOCALAPPDATA%\MitiBar`, which is the log folder on case-insensitive Windows).
4. Check the app left no login item:
   `reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v mitibar` must report that
   the value does not exist, and `%LOCALAPPDATA%\Programs\MitiBar` must be gone.
   Settings live in `%APPDATA%\mitibar` and logs in `%LOCALAPPDATA%\mitibar`; the uninstaller
   keeps both on purpose. Remove those folders by hand if the test needs a clean profile.

### macOS

1. Open the `.dmg`, drag `MitiBar.app` to `/Applications`, eject the disk image. Unsigned 0.x
   builds show the Gatekeeper warning: Control-click the app, choose Open, then confirm.
2. Quit from the menu-bar icon. Uninstall: drag `MitiBar.app` from `/Applications` to the Trash.
3. Remove the per-user state if needed: `~/Library/Application Support/mitibar` (settings and
   log). If "Launch at login" was on, also remove MitiBar from System Settings > General >
   Login Items.
4. A signed and notarized 1.0 build must open without the Gatekeeper bypass.

### Linux

1. AppImage: `chmod +x MitiBar-<version>-linux-x86_64.AppImage` and run it. It needs a desktop
   session with a tray (StatusNotifier/AppIndicator), OpenGL (the femtovg renderer), fontconfig,
   xkbcommon and D-Bus. MitiBar uses no GTK or WebKit. The AppImage and `.deb` names below are
   the expected cargo-packager names; confirm them against the first workflow run.
2. Debian/Ubuntu: `sudo apt install ./MitiBar-<version>-linux-amd64.deb` (or `dpkg -i` followed
   by `apt-get -f install`). Uninstall with `sudo apt remove mitibar`.
3. Settings and log live in `~/.config/mitibar` and `~/.local/share/mitibar`. Remove those
   folders for a clean-profile test. A login item is
   `~/.config/autostart/mitibar.desktop`; remove it if it was enabled.

## Smoke test after installing

Do this on each OS on a clean user account, with the network disconnected or a firewall log open:

1. The tray/menu bar shows today's BS day. Compare the date with
   `cargo run -p nepdate --example convert --features jiff -- --today` and with the wall clock.
2. Open the popup: the header shows the BS long date, weekday, AD date in the language's digits,
   and the clock. The month grid shows today, the AD day, the tithi and the next-holiday line.
3. Click today's cell: the detail card shows the tithi, holiday and events. Copy BS and AD, then
   paste into a text editor and check the exact strings.
4. Open Convert from the header and convert `2081/01/01` both ways; open the Days tab and check a
   one-month span. Open Settings from the tray: switch to Nepali, back to English, change the
   theme, close and reopen the popup. The app must still be responsive.
5. Quit from the tray. Launch again: one tray icon, not two, and the second launch opens the
   first instance's popup. On Windows, confirm no second process remains.
6. No network requests: on Windows, Resource Monitor or `Get-NetTCPConnection -OwningProcess
   <pid>` shows no connections; on macOS/Linux, `lsof -i -p <pid>` is empty. The app has no
   telemetry and no update check (ADR-0008 B.8).
7. Uninstall and repeat the cleanup check in the OS section above.

## Known gaps before 1.0

These are accepted for 0.x previews and recorded here so a 1.0 release cannot skip them.

| Gap | Today | Needed before 1.0 |
|---|---|---|
| Signing credentials | The Windows and macOS signing steps exist and are gated on `WINDOWS_SIGNING_ENABLED` / `MACOS_SIGNING_ENABLED`, but no certificate, Developer ID or notarization secrets are configured here. 0.x ships unsigned and says so. | Provision Azure Artifact Signing and the Apple Developer ID plus notarization secrets, set both variables, and verify a signed asset. |
| Signing not exercised | `xtask/src/workflow.rs` checks the order (sign `mitibar.exe`, package, sign the NSIS setup; sign `MitiBar.app`, build the DMG, notarize and staple it), but no workflow run has exercised the tools. | A `workflow_dispatch` dry run with both variables on, on all 3 OSes. |
| Windows ARM64 package | The matrix builds `aarch64-pc-windows-msvc` and names the installer and portable zip arm64, but this was not built or installed locally. | The first workflow run on `windows-latest`; optionally a real ARM64 machine. |
| Quiet performance | W5: femtovg 112.92 MB working set and 0.825 % of one core against 40 MB and about 0; the software renderer measured 60.23 MB and 0.335 %. Both miss the RSS target (`docs/benchmarks-widget.md`). | A new ADR for a renderer or target change, then a quiet 5-minute run and 5 cold starts. |
| macOS and Linux packages | Built only in the release workflow; not built or installed locally on this Windows machine. | The first workflow run and the 3-OS manual checklist. |

## Releasing

```bash
git tag mitibar-vX.Y.Z
git push origin mitibar-vX.Y.Z
```

`release-mitibar.yml` runs `preflight`, builds and packages on the 3 OSes (signing only where
`WINDOWS_SIGNING_ENABLED` or `MACOS_SIGNING_ENABLED` is `true`), computes `SHA256SUMS`, creates
the CycloneDX SBOM, attests the artifacts, and waits for the `production` approval. It then
creates a **draft** GitHub Release with the changelog section and every asset; it never
publishes.

The approver downloads each artifact, checks it against `SHA256SUMS` and the attestation,
runs the ticked manual test and the smoke test above on a clean VM or user account, then
publishes the draft. The release notes name any unsigned-preview warnings.

## After releasing

1. Read the release page: notes match the changelog, the draft is now published, and the assets
   include the installers, `SHA256SUMS` and the SBOM.
2. Check `gh attestation verify` on one asset from a different machine.
3. Add an empty `## [Unreleased]` to `apps/mitibar/CHANGELOG.md` for the next cycle.
4. Update `docs/plan/PROGRESS.md` with the release row.

## Rolling back

A desktop release is rolled back by hiding it, not deleting it:

1. Edit the GitHub Release, tick **Set as a pre-release**, and save. It leaves "latest".
2. Edit the previous good release and mark it **latest**.
3. Add a note to the broken release: what fails, the workaround, and the fixed version.
4. Fix forward with a patch version (`X.Y.Z+1`). Never reuse a tag or replace an asset in place:
   people may already have the old bytes.
5. If the failure is a bad login item or installer, the runbook owner removes it from the
   affected OS as described above. The app has no auto-update, so users keep the version they
   installed until they download the fixed one.
