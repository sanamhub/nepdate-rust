#!/usr/bin/env bash
# Prints the version the two library crates declare, after checking they agree (hub ADR-0004 §3).
# The release workflow reads crates/nepdate/Cargo.toml the same way.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"

version_of() {
  sed -n 's/^version = "\(.*\)"/\1/p' "$1" | head -n1
}

core="$(version_of "$root/crates/nepdate/Cargo.toml")"
patro="$(version_of "$root/crates/nepdate-patro/Cargo.toml")"

if [ -z "$core" ]; then
  echo "version: crates/nepdate/Cargo.toml has no version" >&2
  exit 1
fi
if [ "$core" != "$patro" ]; then
  echo "version: nepdate is $core but nepdate-patro is $patro" >&2
  exit 1
fi
printf '%s\n' "$core"
