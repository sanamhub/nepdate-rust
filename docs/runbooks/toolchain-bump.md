# Runbook: bump the Rust toolchain or a major dependency

1. Read the release notes (https://releases.rs for Rust; the CHANGELOG for Slint/jiff).
2. Change `rust-toolchain.toml` `channel` (or the dependency version) on a branch.
3. `cargo update` (dependencies only), then `cargo xtask ci`. Fix new clippy lints properly;
   don't blanket-`allow` them.
4. MSRV (`rust-version`) changes only with a CHANGELOG entry, and for the library only in a
   minor release (≥ 0.x+1).
5. PR titled `chore: bump toolchain to 1.NN`. It needs normal review.
